// Compile the bootloader's real source files, not copies of them.
#[path = "../../bootloader/src/crc.rs"]
pub mod crc;
#[path = "../../bootloader/src/frame.rs"]
pub mod frame;

#[cfg(test)]
mod tests {
    use super::crc::crc32;
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
}
