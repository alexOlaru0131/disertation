#!/usr/bin/env python3
"""Send 20 PCA9685 angles (centidegrees) to two I2C modules over USB CDC."""

import argparse
import time

import serial


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "angles",
        nargs="+",
        type=int,
        help="one angle for all outputs or 20 angles in centidegrees (0-36000)",
    )
    parser.add_argument(
        "--port",
        default="/dev/serial/by-id/usb-Alex_Software_Racheta_Debug_001-if00",
    )
    args = parser.parse_args()
    if len(args.angles) not in (1, 20):
        parser.error("provide one angle or exactly 20 angles")
    if any(angle < 0 or angle > 36_000 for angle in args.angles):
        parser.error("angles must be between 0 and 36000 centidegrees")
    angles = args.angles * 20 if len(args.angles) == 1 else args.angles

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
        payload = bytearray((0x20,))
        for angle in angles:
            payload.extend(angle.to_bytes(2, "little"))
        port.write(payload)
        reply = port.read(1)
        if reply == b"\xe1":
            error = port.read(4)
            if len(error) != 4:
                raise RuntimeError("Incomplete PCA9685 error response")
            address, code, stage, detail = error
            raise RuntimeError(
                f"PCA9685 0x{address:02X}: error {code}, stage/channel {stage}, detail {detail}"
            )
        if reply != b"\x20":
            raise RuntimeError(f"Unexpected PCA9685 response: {reply!r}")
        count = port.read(1)
        if count != b"\x14":
            raise RuntimeError(f"Unexpected acknowledged angle count: {count!r}")

    print("PCA9685 0x41 (first 10) and 0x42 (last 10): 20 angles confirmed")


if __name__ == "__main__":
    main()
