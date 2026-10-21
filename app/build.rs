use std::{env, fs, path::PathBuf};

// Use the bootloader's own slot table, so the app is always linked where the bootloader looks.
#[allow(dead_code)]
#[path = "../bootloader/src/flash_map.rs"]
mod flash_map;

fn main() {
    println!("cargo:rerun-if-env-changed=APP_SLOT");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../bootloader/src/flash_map.rs");
    let index: usize = env::var("APP_SLOT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let slot = flash_map::slot(index)
        .unwrap_or_else(|| panic!("APP_SLOT must be 0-{}", flash_map::SLOT_COUNT - 1));
    let memory_x = format!(
        "MEMORY\n{{\n  FLASH : ORIGIN = {:#x}, LENGTH = {}\n  RAM : ORIGIN = {:#x}, LENGTH = {}\n}}\n",
        slot.base,
        slot.end - slot.base,
        flash_map::RAM_START,
        flash_map::RAM_END - flash_map::RAM_START,
    );
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out.join("memory.x"), memory_x).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
}
