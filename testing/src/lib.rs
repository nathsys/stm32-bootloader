// Compile the bootloader's real source files, not copies of them.
#[path = "../../bootloader/src/crc.rs"]
pub mod crc;

#[path = "../../bootloader/src/frame.rs"]
pub mod frame;

#[path = "../../bootloader/src/flash_map.rs"]
pub mod flash_map;

#[cfg(test)]
mod tests {
    use super::crc::crc32;
    use super::flash_map::{SLOT_COUNT, sector_of, sectors_for, slot};
    use super::frame::{FeedResult, FrameParser, START_BYTE};

    //  helpers

    /// Builds a valid frame: start byte, length (little-endian), payload, CRC-32 of length+payload.
    fn make_frame(payload: &[u8]) -> Vec<u8> {
        let mut body = vec![payload.len() as u8, (payload.len() >> 8) as u8];
        body.extend_from_slice(payload);
        let crc = crc32(&body);
        let mut frame = vec![START_BYTE];
        frame.extend_from_slice(&body);
        frame.extend_from_slice(&crc.to_le_bytes());
        frame
    }

    /// Feeds every byte and returns the result of the last one.
    fn feed_all(p: &mut FrameParser, bytes: &[u8]) -> FeedResult {
        let mut last = FeedResult::Pending;
        for &b in bytes {
            last = p.feed(b);
        }
        last
    }

    // crc

    #[test]
    fn crc_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn crc_empty_input() {
        assert_eq!(crc32(b""), 0);
    }

    // frame parser

    #[test]
    fn valid_frame() {
        let mut p = FrameParser::new();
        let frame = make_frame(b"Hi");
        let (last, rest) = frame.split_last().unwrap();
        for &b in rest {
            assert_eq!(p.feed(b), FeedResult::Pending);
        }
        assert_eq!(p.feed(*last), FeedResult::Frame(2));
        assert_eq!(p.payload(), b"Hi");
    }

    #[test]
    fn two_valid_frames_back_to_back() {
        let mut p = FrameParser::new();
        assert_eq!(feed_all(&mut p, &make_frame(b"one")), FeedResult::Frame(3));
        assert_eq!(p.payload(), b"one");
        assert_eq!(
            feed_all(&mut p, &make_frame(b"second")),
            FeedResult::Frame(6)
        );
        assert_eq!(p.payload(), b"second");
    }

    #[test]
    fn flipped_payload_byte_is_bad_crc() {
        let mut p = FrameParser::new();
        let mut frame = make_frame(b"Hello");
        frame[3] ^= 0x01; // index 0 start, 1-2 length, 3 = first payload byte
        assert_eq!(feed_all(&mut p, &frame), FeedResult::BadCrc);
    }

    #[test]
    fn flipped_crc_byte_is_bad_crc() {
        let mut p = FrameParser::new();
        let mut frame = make_frame(b"Hello");
        let last = frame.len() - 1;
        frame[last] ^= 0x80;
        assert_eq!(feed_all(&mut p, &frame), FeedResult::BadCrc);
    }

    #[test]
    fn length_limits() {
        // exactly the maximum is accepted
        let mut p = FrameParser::new();
        assert_eq!(
            feed_all(&mut p, &make_frame(&[0x55; 256])),
            FeedResult::Frame(256)
        );
        // 257 is rejected as soon as the length is known
        let mut p = FrameParser::new();
        assert_eq!(
            feed_all(&mut p, &[START_BYTE, 0x01, 0x01]),
            FeedResult::BadLength
        );
        // 0 is rejected too
        let mut p = FrameParser::new();
        assert_eq!(
            feed_all(&mut p, &[START_BYTE, 0x00, 0x00]),
            FeedResult::BadLength
        );
    }

    #[test]
    fn recovers_after_bad_crc() {
        let mut p = FrameParser::new();
        let mut bad = make_frame(b"broken");
        bad[3] ^= 0xFF;
        assert_eq!(feed_all(&mut p, &bad), FeedResult::BadCrc);
        assert_eq!(feed_all(&mut p, &make_frame(b"fine")), FeedResult::Frame(4));
        assert_eq!(p.payload(), b"fine");
    }

    #[test]
    fn garbage_before_start_is_ignored() {
        let mut p = FrameParser::new();
        assert_eq!(feed_all(&mut p, &[0x00, 0x13, 0xFF]), FeedResult::Pending);
        assert_eq!(feed_all(&mut p, &make_frame(b"ok")), FeedResult::Frame(2));
    }

    #[test]
    fn payload_is_empty_after_a_failed_frame() {
        let mut p = FrameParser::new();
        feed_all(&mut p, &[START_BYTE, 0x01, 0x01]); // BadLength
        assert!(p.payload().is_empty());

        let mut bad = make_frame(b"xyz");
        bad[3] ^= 0x01;
        feed_all(&mut p, &bad); // BadCrc
        assert!(p.payload().is_empty());
    }

    // Flash map
    #[test]
    fn sector_boundaries() {
        assert_eq!(sector_of(0x0800_FFFF), None);
        assert_eq!(sector_of(0x0801_0000), Some(4));
        assert_eq!(sector_of(0x0801_FFFF), Some(4));
        assert_eq!(sector_of(0x0802_0000), Some(5));
        assert_eq!(sector_of(0x0803_FFFF), Some(5));
        assert_eq!(sector_of(0x0804_0000), Some(6));
        assert_eq!(sector_of(0x0806_0000), Some(7));
        assert_eq!(sector_of(0x0807_FFFF), Some(7));
        assert_eq!(sector_of(0x0808_0000), None);
    }

    #[test]
    fn bootloader_area_and_ram_are_never_mapped() {
        assert_eq!(sector_of(0x0800_0000), None);
        assert_eq!(sector_of(0x0800_4000), None);
        assert_eq!(sector_of(0x2000_0000), None);
        assert_eq!(sector_of(0), None);
    }

    #[test]
    fn twenty_kb_app_fits_in_one_sector() {
        let base = 0x0801_0000u32;
        let last = base + 20 * 1024 - 1;
        assert_eq!(sector_of(base), Some(4));
        assert_eq!(sector_of(last), Some(4));
    }

    #[test]
    fn sectors_for_ranges() {
        assert_eq!(sectors_for(0x0801_0000, 20 * 1024), Some((4, 4)));
        assert_eq!(sectors_for(0x0801_0000, 64 * 1024), Some((4, 4))); // fits exactly
        assert_eq!(sectors_for(0x0801_0000, 64 * 1024 + 1), Some((4, 5))); // one byte over
        assert_eq!(sectors_for(0x0801_0000, 448 * 1024), Some((4, 7))); // the whole area
        assert_eq!(sectors_for(0x0801_0000, 448 * 1024 + 1), None); // past the end
        assert_eq!(sectors_for(0x0807_FFFF, 1), Some((7, 7))); // last byte of flash
        assert_eq!(sectors_for(0x0807_FFFF, 2), None);
    }

    #[test]
    fn sectors_for_rejects_bad_input() {
        assert_eq!(sectors_for(0x0800_FFFF, 2), None); // starts in the bootloader's area
        assert_eq!(sectors_for(0x0801_0000, 0), None); // empty
        assert_eq!(sectors_for(0x0801_0000, u32::MAX), None); // would overflow
    }

    // App slots

    #[test]
    fn slot_table() {
        let expected = [
            (0x0801_0000, 0x0802_0000), // 64 KB
            (0x0802_0000, 0x0804_0000), // 128 KB
            (0x0804_0000, 0x0806_0000),
            (0x0806_0000, 0x0808_0000),
        ];
        assert_eq!(SLOT_COUNT, expected.len());
        for (i, &(base, end)) in expected.iter().enumerate() {
            let s = slot(i).unwrap();
            assert_eq!((s.base, s.end), (base, end), "slot {i}");
        }
    }

    #[test]
    fn slot_index_out_of_range_is_none() {
        assert!(slot(SLOT_COUNT).is_none());
        assert!(slot(usize::MAX).is_none());
    }

    #[test]
    fn each_slot_is_exactly_one_sector() {
        for i in 0..SLOT_COUNT {
            let s = slot(i).unwrap();
            let sector = 4 + i as u8;
            assert_eq!(
                sectors_for(s.base, s.end - s.base),
                Some((sector, sector)),
                "slot {i}"
            );
            // One byte more spills into the next sector, or past the end of flash for the last slot.
            let next = if i + 1 < SLOT_COUNT {
                Some((sector, sector + 1))
            } else {
                None
            };
            assert_eq!(
                sectors_for(s.base, s.end - s.base + 1),
                next,
                "slot {i} + 1 byte"
            );
        }
    }
}
