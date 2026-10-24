#!/usr/bin/env python3
"""Test the A0 LED via USB CDC with visual on/off confirmation."""

import argparse
import time

import serial


def confirm(message):
    return input(f"{message} [d/N] ").strip().lower() in ("d", "da")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--port",
        default="/dev/serial/by-id/usb-Alex_Software_Racheta_Debug_001-if00",
    )
    parser.add_argument(
        "--no-confirm",
        action="store_true",
        help="send both commands without visual confirmation",
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
        try:
            port.write(b"\x30")
            port.flush()
            time.sleep(0.1)
            if args.no_confirm:
                print("0x30 trimis: LED-ul ar trebui sa fie aprins.")
                time.sleep(1)
            else:
                on_confirmed = confirm("LED-ul de pe A0 este aprins?")
        finally:
            port.write(b"\x35")
            port.flush()

        time.sleep(0.1)
        if not args.no_confirm:
            off_confirmed = confirm("LED-ul de pe A0 este stins?")

    if args.no_confirm:
        print("0x35 trimis: LED-ul ar trebui sa fie stins (neconfirmat vizual).")
    elif on_confirmed and off_confirmed:
        print("LED pe A0: aprinderea si stingerea au fost confirmate.")
    else:
        raise RuntimeError("LED pe A0: aprinderea sau stingerea nu a fost confirmata")


if __name__ == "__main__":
    main()
