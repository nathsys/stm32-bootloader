#!/usr/bin/env bash
# usage: ./flash.sh [bootloader | app <slot 0-3> | all]
#   all (default) = bootloader + app in slot 0
set -euo pipefail

CHIP=STM32F411RETx
TARGET=thumbv7em-none-eabihf
ROOT="$(cd "$(dirname "$0")" && pwd)"

build_and_flash() { 
    local dir="$ROOT/$1" slot="${2:-0}"
    local name
    name=$(sed -n 's/^name = "\(.*\)"/\1/p' "$dir/Cargo.toml" | head -1)
    echo "== $1 (slot $slot) =="
    (cd "$dir" && APP_SLOT="$slot" cargo build --release)
    probe-rs download --chip "$CHIP" "$dir/target/$TARGET/release/$name"
}

case "${1:-all}" in
    bootloader) build_and_flash bootloader ;;
    app)        build_and_flash app "${2:?usage: ./flash.sh app <slot 0-3>}" ;;
    all)        build_and_flash bootloader; build_and_flash app 0 ;;
    *)          echo "usage: $0 [bootloader | app <slot 0-3> | all]"; exit 1 ;;
esac

probe-rs reset --chip "$CHIP"