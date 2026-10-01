#!/usr/bin/env python3
"""Enter the Feather M0 UF2 bootloader via its application USB CDC port."""

import argparse
import errno
from pathlib import Path
import time

import serial


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("port", help="Application serial port (e.g. /dev/ttyACM0)")
    args = parser.parse_args()

    with serial.Serial(args.port, 1200, timeout=1) as port:
        port.dtr = True
        time.sleep(0.3)
        try:
            port.dtr = False
        except OSError as exc:
            # Deconectarea USB poate surveni chiar in timpul ioctl-ului DTR.
            if exc.errno not in (
                errno.EPIPE,
                errno.EPROTO,
                errno.ENODEV,
                errno.EIO,
                errno.ESHUTDOWN,
            ):
                raise

    boot_volume = Path("/dev/disk/by-label/FEATHERBOOT")
    deadline = time.monotonic() + 6
    while not boot_volume.exists() and time.monotonic() < deadline:
        time.sleep(0.1)
    if not boot_volume.exists():
        raise RuntimeError("FEATHERBOOT nu a aparut dupa 1200-bps touch")
    print("Feather M0 Express este in bootloader (FEATHERBOOT).")


if __name__ == "__main__":
    main()
