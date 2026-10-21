use crate::crc::crc32;

pub const START_BYTE: u8 = 0xA5;
pub const MAX_PAYLOAD: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    WaitStart,
    LenLow,
    LenHigh,
    Payload,
    Crc,
}

#[derive(PartialEq, Eq, Debug)]
pub enum FeedResult {
    Pending,
    Frame(usize),
    BadLength,
    BadCrc,
}

pub struct FrameParser {
    state: State,
    buf: [u8; 2 + MAX_PAYLOAD],
    len: usize,
    pos: usize,
    crc_pos: usize,
    crc_bytes: [u8; 4],
}

impl Default for FrameParser {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameParser {
    pub fn new() -> Self {
        Self {
            state: State::WaitStart,
            buf: [0; 2 + MAX_PAYLOAD],
            len: 0,
            pos: 0,
            crc_pos: 0,
            crc_bytes: [0; 4],
        }
    }

    pub fn buf_write(&mut self, byte: u8) {
        self.buf[self.pos] = byte;
        self.pos += 1;
    }

    pub fn reset(&mut self) {
        self.state = State::WaitStart;
        self.pos = 0;
        self.crc_bytes = [0, 0, 0, 0];
        self.crc_pos = 0;
        self.len = 0;
    }

    pub fn feed(&mut self, byte: u8) -> FeedResult {
        match self.state {
            State::WaitStart => {
                if byte == START_BYTE {
                    self.state = State::LenLow;
                }
                FeedResult::Pending
            }
            State::LenLow => {
                self.buf_write(byte);
                self.len = byte as usize;
                self.state = State::LenHigh;
                FeedResult::Pending
            }
            State::LenHigh => {
                self.buf_write(byte);
                self.len += (byte as usize) * 256;
                if self.len > MAX_PAYLOAD || self.len == 0 {
                    self.reset();
                    return FeedResult::BadLength;
                }
                self.state = State::Payload;
                FeedResult::Pending
            }
            State::Payload => {
                self.buf_write(byte);
                if self.pos >= self.len + 2 {
                    self.state = State::Crc;
                }
                FeedResult::Pending
            }
            State::Crc => {
                self.crc_bytes[self.crc_pos] = byte;
                self.crc_pos += 1;
                if self.crc_pos == 4 {
                    // Validate payload.
                    let crc_result = crc32(&self.buf[0..2 + self.len]);
                    let host_crc = u32::from_le_bytes(self.crc_bytes);

                    let lencp = self.len;
                    self.reset();
                    match crc_result ^ host_crc {
                        0 => {
                            self.len = lencp;
                            return FeedResult::Frame(self.len);
                        }
                        _ => return FeedResult::BadCrc,
                    }
                }
                FeedResult::Pending
            }
        }
    }

    /// Return the payload, which excludes the 2 start bytes.
    pub fn payload(&self) -> &[u8] {
        &self.buf[2..2 + self.len]
    }
}
