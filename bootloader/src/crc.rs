pub fn crc32(data: &[u8]) -> u32 {
    let mut reg: u32 = u32::MAX;
    for &b in data {
        reg ^= b as u32;
        for _ in 0..8 {
            let last_one = reg & 0b1 != 0;
            reg >>= 1;
            if last_one {
                reg ^= 0xEDB8_8320;
            }
        }
    }
    !reg
}
