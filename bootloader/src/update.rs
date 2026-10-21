use stm32f4xx_hal::flash::FlashExt;
use stm32f4xx_hal::pac;

use crate::crc::crc32;
use crate::flash_driver;
use crate::flash_map;
use crate::flash_map::{APP_SPACE_BASE, sectors_for};

const CMD_BEGIN: u8 = 0x01;
const CMD_DATA: u8 = 0x02;
const CMD_END: u8 = 0x03;

pub struct Updater {
    active: bool,
    base: u32,
    size: u32,
    remaining: u32,
    expected_crc: u32,
}

impl Updater {
    pub fn new() -> Self {
        Self {
            active: false,
            base: 0,
            size: 0,
            remaining: 0,
            expected_crc: 0,
        }
    }

    /// true = ACK, false = NACK
    pub fn handle(&mut self, flash: &mut pac::FLASH, payload: &[u8]) -> bool {
        let Some((&cmd, rest)) = payload.split_first() else {
            return false;
        };
        let result = match cmd {
            CMD_BEGIN => self.begin(flash, rest),
            CMD_DATA => self.data(flash, rest),
            CMD_END => self.end(flash),
            _ => None,
        };
        if result.is_none() {
            self.active = false;
        }
        result.is_some()
    }

    // Receive app info and erase required sectors from flash.
    fn begin(&mut self, flash: &mut pac::FLASH, rest: &[u8]) -> Option<()> {
        (rest.len() == 9).then_some(())?;
        let slot = flash_map::slot(rest[0] as usize)?;
        let size = u32::from_le_bytes(rest.get(1..5)?.try_into().ok()?);
        let expected_crc = u32::from_le_bytes(rest.get(5..9)?.try_into().ok()?);

        let last_byte = slot.base.checked_add(size.checked_sub(1)?)?;
        (last_byte < slot.end).then_some(())?;

        let (first, last) = sectors_for(slot.base, size)?;
        for s in first..=last {
            flash_driver::erase_sector(flash, s).ok()?;
        }
        self.base = slot.base;
        self.size = size;
        self.remaining = size;
        self.expected_crc = expected_crc;
        self.active = true;
        Some(())
    }

    fn data(&mut self, flash: &mut pac::FLASH, rest: &[u8]) -> Option<()> {
        self.active.then_some(())?;
        let offset = u32::from_le_bytes(rest.get(0..4)?.try_into().ok()?);
        let chunk = rest.get(4..)?;
        let received = self.size - self.remaining;
        let len = chunk.len() as u32;
        let end = offset.checked_add(len)?;
        (len != 0).then_some(())?;
        if offset == received {
            (len <= self.remaining).then_some(())?;
            flash_driver::write(flash, self.base.checked_add(offset)?, chunk).ok()?;
            self.remaining -= len;
        } else if end <= received {
            let stored = flash_driver::read(flash, self.base.checked_add(offset)?, chunk.len())?;
            (stored == chunk).then_some(())?;
        } else {
            return None;
        }
        Some(())
    }

    fn end(&mut self, flash: &mut pac::FLASH) -> Option<()> {
        self.active.then_some(())?;
        (self.remaining == 0).then_some(())?;
        let image = flash_driver::read(flash, self.base, self.size as usize)?;
        (crc32(image) == self.expected_crc).then_some(())?;
        self.active = false;
        Some(())
    }
}
