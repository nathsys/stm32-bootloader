# STM32 UART bootloader (Rust)

A small bootloader for the NUCLEO-F411RE (STM32F411RE), written in `no_std` Rust. It validates and launches an application from flash, and accepts new application images over UART with CRC-checked frames.

## Layout

| Directory | What it is |
|---|---|
| `bootloader/` | The bootloader (flash sectors 0-3) |
| `app/` | A test application, linked for the slot given by `APP_SLOT` |
| `testing/` | Host-side tests (CRC, frame parser, sector map) and `send.py`, the uploader |

Application slots (one flash sector each):

| Slot | Start address | Size |
|---|---|---|
| 0 | `0x0801_0000` | 64 KB |
| 1 | `0x0802_0000` | 128 KB |
| 2 | `0x0804_0000` | 128 KB |
| 3 | `0x0806_0000` | 128 KB |

At boot, the first slot with a valid image is launched. If no slot is valid, or button B1 is held during reset, the bootloader waits for an update over UART.

## Install the bootloader

```bash
cd bootloader
cargo build --release
probe-rs download --chip STM32F411RETx target/thumbv7em-none-eabihf/release/bootloader
probe-rs reset --chip STM32F411RETx
```

## Build and upload an application

Build the image for the slot you want (the slot number sets the link address):

```bash
cd app
APP_SLOT=0 cargo objcopy --release -- -O binary ../app0.bin
cd ..
```

Put the bootloader into update mode: hold **B1** (the blue button), press reset, and keep holding B1 until the startup blink ends (about 1 s). Then:

```bash
python3 testing/send.py app0.bin 0
```

Power-cycle or reset the board to run the new application. Use a different slot number for a different slot (`APP_SLOT=1` when building, `send.py app1.bin 1` when uploading).

## Tests

Host-side tests need no board:

```bash
cd testing
cargo test --target "$(rustc -vV | sed -n 's/^host: //p')"
```