#!/usr/bin/env python3
"""Upload a firmware image to one bootloader slot.

usage: send.py <image.bin> <slot 0-3>
"""
import struct, sys, zlib
import serial

PORT, BAUD = "/dev/ttyACM0", 115200
START, ACK = 0xA5, 0x06
BEGIN, DATA, END = 0x01, 0x02, 0x03
CHUNK = 240  

RAM_START, RAM_END = 0x2000_0000, 0x2002_0000
SLOTS = [                                               
    (0x0801_0000, 0x0802_0000),
    (0x0802_0000, 0x0804_0000),
    (0x0804_0000, 0x0806_0000),
    (0x0806_0000, 0x0808_0000),
]


def check_vectors(data: bytes, slot: int):
    """Exit if the bootloader would refuse to boot this image from this slot."""
    sp, rv = struct.unpack_from("<II", data)               
    base, end = SLOTS[slot]
    if not RAM_START <= sp <= RAM_END:
        sys.exit(f"initial stack pointer {sp:#010x} is outside RAM")
    if rv % 2 == 0 or not base <= rv < end:
        sys.exit(f"reset vector {rv:#010x} is not odd and inside slot {slot}; "
                 f"was the image built with APP_SLOT={slot}?")


def send_frame(ser, payload: bytes, what: str):
    body = struct.pack("<H", len(payload)) + payload      
    frame = bytes([START]) + body + struct.pack("<I", zlib.crc32(body))
    for _ in range(3):                                 
        ser.write(frame)
        if ser.read(1) == bytes([ACK]):               
            return
    sys.exit(f"{what} failed")


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    data = open(sys.argv[1], "rb").read()
    slot = int(sys.argv[2])
    if not 0 <= slot < len(SLOTS) or len(data) < 8:
        sys.exit(__doc__)
    check_vectors(data, slot)

    with serial.Serial(PORT, BAUD, timeout=10) as ser:
        send_frame(ser, bytes([BEGIN, slot]) + struct.pack("<II", len(data), zlib.crc32(data)), "BEGIN")
        for offset in range(0, len(data), CHUNK):
            send_frame(ser, bytes([DATA]) + struct.pack("<I", offset) + data[offset:offset + CHUNK],
                       f"chunk at offset {offset}")
        send_frame(ser, bytes([END]), "END (CRC mismatch)")
    print(f"done: slot {slot} updated ({len(data)} bytes)")


if __name__ == "__main__":
    main()
