#!/usr/bin/env bash
set -euo pipefail

CHIP=STM32F411RETx
TARGET=thumbv7em-none-eabihf
ROOT="$(cd "$(dirname "$0")" && pwd)"

flash() {
    local dir="$ROOT/$1"
    cd "$dir"
    cargo build --release
    # the ELF is named after the package in Cargo.toml, not the folder
    local name
    name=$(sed -n 's/^name = "\(.*\)"/\1/p' Cargo.toml | head -1)
    echo "Flashing $1 ($name)"
    probe-rs download --chip "$CHIP" "target/$TARGET/release/$name"
}

flash bootloader
flash app

probe-rs reset --chip "$CHIP"
