#!/usr/bin/env python3
"""Read the six active-low end-stop inputs over USB CDC."""

import argparse
import time

import serial


ENDSTOPS = (
    "dreapta 9 (D9)",
    "dreapta 6 (D6)",
    "dreapta 5 (D5)",
    "stanga 12 (D12)",
    "stanga 11 (D11)",
    "stanga 10 (D10)",
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--port",
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
        port.write(b"\x50")
        port.flush()
        states = port.read(len(ENDSTOPS))

    if len(states) != len(ENDSTOPS):
        raise RuntimeError(f"Răspuns end-stop incomplet: {len(states)} din 6 octeți")
    if any(state not in (0, 1) for state in states):
        raise RuntimeError(f"Răspuns end-stop invalid: {states!r}")

    for name, pressed in zip(ENDSTOPS, states):
        print(f"{name}: {pressed} ({'apăsat, pin LOW' if pressed else 'liber, pin HIGH'})")


if __name__ == "__main__":
    main()
