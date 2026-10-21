#!/usr/bin/env bash
set -euo pipefail

CHIP=STM32F411RETx
TARGET=thumbv7em-none-eabihf
APP_BASE=0x08010000
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-empty}"

fail() { echo "FAIL: $1"; exit 1; }

elf_path() {   # elf_path <crate-dir>
    local name
    name=$(sed -n 's/^name = "\(.*\)"/\1/p' "$ROOT/$1/Cargo.toml" | head -1)
    echo "$ROOT/$1/target/$TARGET/release/$name"
}

app_region_erased() {   # true if the first 8 words of the app slot are all 0xFFFFFFFF
    local out
    out=$(probe-rs read --chip "$CHIP" b32 "$APP_BASE" 8 2>/dev/null)
    ! echo "$out" | sed 's/^[0-9a-fA-F]*://' | tr -s ' \n' '\n' \
        | grep -v '^$' | grep -qvi '^ffffffff$'
}

echo "== build =="
(cd "$ROOT/bootloader" && cargo build --release)
[ "$MODE" = app ] && (cd "$ROOT/app" && cargo build --release)

echo "== erase whole chip =="
probe-rs erase --chip "$CHIP"
app_region_erased || fail "app region is not blank after erase"

echo "== flash bootloader =="
probe-rs download --chip "$CHIP" "$(elf_path bootloader)"

if [ "$MODE" = app ]; then
    echo "== flash app =="
    probe-rs download --chip "$CHIP" "$(elf_path app)"
    app_region_erased && fail "app region is still blank after flashing the app"
else
    app_region_erased || fail "flashing the bootloader wrote into the app region"
fi

echo "== reset =="
probe-rs reset --chip "$CHIP"
sleep 4    # startup blinks are 2 x 1 s, plus margin

if [ "$MODE" = app ]; then
    echo "Expected: 2 slow blinks, then FAST blinking at 100 ms (the app)."
else
    echo "Expected: 2 slow blinks, then the error blink (your err pattern),"
    echo "NOT the app's 100 ms blink, and NOT a dark LED."
fi
read -r -p "Does the LED match? [y/N] " ans
[ "$ans" = y ] || fail "LED behaviour did not match"
echo "PASS ($MODE)"
