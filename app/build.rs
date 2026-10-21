use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=APP_SLOT");
    println!("cargo:rerun-if-changed=build.rs");
    let slot: usize = env::var("APP_SLOT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    // (origin, length) of each slot, matching the bootloader's flash_map
    let (origin, length) = [
        (0x0801_0000u32, 64 * 1024u32),
        (0x0802_0000, 128 * 1024),
        (0x0804_0000, 128 * 1024),
        (0x0806_0000, 128 * 1024),
    ][slot];
    let memory_x = format!(
        "MEMORY\n{{\n  FLASH : ORIGIN = {origin:#x}, LENGTH = {length}\n  RAM : ORIGIN = 0x20000000, LENGTH = 128K\n}}\n"
    );
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out.join("memory.x"), memory_x).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
}
