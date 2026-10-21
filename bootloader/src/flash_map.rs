/// Start of the chip's flash memory.
pub const FLASH_BASE: u32 = 0x0800_0000;

/// RAM is 128 KB. A valid initial stack pointer lies in RAM_START..=RAM_END.
pub const RAM_START: u32 = 0x2000_0000;
pub const RAM_END: u32 = 0x2002_0000;

pub const FIRST_APP_SECTOR: u8 = 4;
pub const LAST_APP_SECTOR: u8 = 7;

pub const APP_SPACE_BASE: u32 = 0x0801_0000;
pub const APP_SPACE_END: u32 = 0x0808_0000;

const SECTORS: [(u32, u32); 4] = [
    (APP_SPACE_BASE, 0x0802_0000),
    (0x0802_0000, 0x0804_0000),
    (0x0804_0000, 0x0806_0000),
    (0x0806_0000, APP_SPACE_END),
];

pub struct AppSlot {
    pub base: u32,
    pub end: u32,
}

pub const SLOT_COUNT: usize = SECTORS.len();

pub fn slot(index: usize) -> Option<AppSlot> {
    SECTORS.get(index).map(|&(base, end)| AppSlot { base, end })
}

pub fn sector_of(addr: u32) -> Option<u8> {
    for (i, &(start, end)) in SECTORS.iter().enumerate() {
        if addr >= start && addr < end {
            return Some(i as u8 + FIRST_APP_SECTOR);
        }
    }
    None
}

pub fn sectors_for(addr: u32, len: u32) -> Option<(u8, u8)> {
    if len == 0 {
        return None;
    }
    let end = addr.checked_add(len - 1)?;
    let first = sector_of(addr)?;
    let last = sector_of(end)?;
    Some((first, last))
}
