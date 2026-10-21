const SECTORS: [(u32, u32); 4] = [
    (0x0801_0000, 0x0802_0000),
    (0x0802_0000, 0x0804_0000),
    (0x0804_0000, 0x0806_0000),
    (0x0806_0000, 0x0808_0000),
];

pub fn sector_of(addr: u32) -> Option<u8> {
    for (i, &(start, end)) in SECTORS.iter().enumerate() {
        if addr >= start && addr < end {
            return Some(i as u8 + 4);
        }
    }
    None
}
