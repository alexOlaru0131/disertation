#!/usr/bin/env python3
"""Request one measurement frame from the Feather M0 over USB CDC."""

import argparse
import struct
import time

import serial


FIELDS = (
    ("accel X", "m/s²"),
    ("accel Y", "m/s²"),
    ("accel Z", "m/s²"),
    ("gyro X", "rad/s"),
    ("gyro Y", "rad/s"),
    ("gyro Z", "rad/s"),
    ("mag X", "µT"),
    ("mag Y", "µT"),
    ("mag Z", "µT"),
    ("temperatura", "°C"),
)


def read_exact(port, size):
    data = port.read(size)
    if len(data) != size:
        raise RuntimeError(f"Răspuns incomplet: {len(data)} din {size} octeți")
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "port",
        nargs="?",
        default="/dev/serial/by-id/usb-Alex_Software_Racheta_Debug_001-if00",
    )
    args = parser.parse_args()

    deadline = time.monotonic() + 8
    while True:
        try:
            port = serial.Serial(args.port, 115200, timeout=3)
            break
        except serial.SerialException:
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)

    with port:
        port.write(b"\x10")
        header = read_exact(port, 1)[0]
        if header == 0xE0:
            code, detail, extra = read_exact(port, 3)
            raise RuntimeError(f"Senzor: eroare {code}, detaliu {detail}, supliment {extra}")
        if header != 0x10:
            raise RuntimeError(f"Antet necunoscut: 0x{header:02X}")
        values = struct.unpack("<10i", read_exact(port, 40))

    for (name, unit), value in zip(FIELDS, values):
        print(f"{name:12s} {value / 100:9.2f} {unit}")


if __name__ == "__main__":
    main()
