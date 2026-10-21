use stm32f4xx_hal::flash::FlashExt;
use stm32f4xx_hal::pac;

use crate::flash_map::{FIRST_APP_SECTOR, FLASH_BASE, LAST_APP_SECTOR, sector_of};

pub enum FlashError {
    OutOfRange,
    Hardware,
    Verify,
}

pub fn erase_sector(flash: &mut pac::FLASH, sector: u8) -> Result<(), FlashError> {
    if !(FIRST_APP_SECTOR..=LAST_APP_SECTOR).contains(&sector) {
        return Err(FlashError::OutOfRange);
    }
    let mut unl = FlashExt::unlocked(flash);
    unl.erase(sector).map_err(|_| FlashError::Hardware)?;
    Ok(())
}

pub fn write(flash: &mut pac::FLASH, addr: u32, data: &[u8]) -> Result<(), FlashError> {
    if data.is_empty() {
        return Ok(());
    }
    let addr_end = addr
        .checked_add(data.len() as u32 - 1)
        .ok_or(FlashError::OutOfRange)?;
    if sector_of(addr).is_none() || sector_of(addr_end).is_none() {
        return Err(FlashError::OutOfRange);
    }
    let offset = (addr - FLASH_BASE) as usize;
    {
        let mut unl = FlashExt::unlocked(flash);
        unl.program(offset, data.iter())
            .map_err(|_| FlashError::Hardware)?;
    }
    let stored: &[u8] = &flash.read()[offset..offset + data.len()];
    if stored != data {
        return Err(FlashError::Verify);
    }
    Ok(())
}

pub fn read(flash: &pac::FLASH, addr: u32, len: usize) -> Option<&[u8]> {
    let start = addr.checked_sub(FLASH_BASE)? as usize;
    FlashExt::read(flash).get(start..start.checked_add(len)?)
}
