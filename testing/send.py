#!/usr/bin/env python3
"""Upload a firmware image to one bootloader slot.

usage: send.py <image.bin> <slot 0-3> [corrupt]
"""
import struct, sys, time, zlib
import serial

PORT, BAUD = "/dev/ttyACM0", 115200
START, ACK = 0xA5, 0x06
BEGIN, DATA, END = 0x01, 0x02, 0x03
CHUNK = 240   

# Must match bootloader/src/flash_map.rs
RAM_START, RAM_END = 0x2000_0000, 0x2002_0000
SLOTS = [                                                  
    (0x0801_0000, 0x0802_0000),
    (0x0802_0000, 0x0804_0000),
    (0x0804_0000, 0x0806_0000),
    (0x0806_0000, 0x0808_0000),
]


def make_frame(payload: bytes) -> bytes:
    body = struct.pack("<H", len(payload)) + payload      
    return bytes([START]) + body + struct.pack("<I", zlib.crc32(body))


def check_vectors(data: bytes, slot: int):
    """Return why the bootloader would refuse to boot this image from this slot, or None if OK."""
    if len(data) < 8:
        return "image is too short to hold a vector table"
    sp, rv = struct.unpack_from("<II", data)                
    base, end = SLOTS[slot]
    if not RAM_START <= sp <= RAM_END:
        return f"initial stack pointer {sp:#010x} is outside RAM"
    if rv % 2 == 0:
        return f"reset vector {rv:#010x} is even (Thumb code addresses must be odd)"
    if not base <= rv < end:
        return (f"reset vector {rv:#010x} is outside slot {slot} ({base:#010x}..{end:#010x}); "
                f"was the image built with APP_SLOT={slot}?")
    return None


def send_frame(ser, payload: bytes, what: str, retries: int = 3) -> bool:
    frame = make_frame(payload)
    for attempt in range(1, retries + 1):
        ser.reset_input_buffer()
        ser.write(frame)
        reply = ser.read(1)                               
        if reply == bytes([ACK]):
            return True
        print(f"  {what}, attempt {attempt}: got {reply!r}")
    return False


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    data = open(sys.argv[1], "rb").read()
    slot = int(sys.argv[2])
    if not data:
        sys.exit("image is empty")
    if not 0 <= slot < len(SLOTS):
        sys.exit(f"slot must be 0-{len(SLOTS) - 1}")
    problem = check_vectors(data, slot)
    if problem:
        sys.exit(f"refusing to upload: {problem}")

    crc = zlib.crc32(data)                                   
    if len(sys.argv) > 3 and sys.argv[3] == "corrupt":
        data = data[:-1] + bytes([data[-1] ^ 0xFF])       
        print("sending a corrupted last byte on purpose; END should fail")

    with serial.Serial(PORT, BAUD, timeout=10) as ser:      
        time.sleep(0.2)
        begin = bytes([BEGIN, slot]) + struct.pack("<II", len(data), crc)
        if not send_frame(ser, begin, "BEGIN"):
            sys.exit("BEGIN refused (wrong slot, image too big, or not in update mode)")
        total = (len(data) + CHUNK - 1) // CHUNK
        for n, i in enumerate(range(0, len(data), CHUNK), 1):
            payload = bytes([DATA]) + struct.pack("<I", i) + data[i:i + CHUNK]
            if not send_frame(ser, payload, f"chunk {n}/{total}"):
                sys.exit(f"chunk {n} failed")
        print(f"{total} chunks sent")
        if not send_frame(ser, bytes([END]), "END"):
            sys.exit("END refused: the CRC of what is in flash does not match")
    print(f"done: slot {slot} updated ({len(data)} bytes)")


if __name__ == "__main__":
    main()
