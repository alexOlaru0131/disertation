#!/usr/bin/env bash
set -euo pipefail

cargo build --release

# Bootloader-ul existent necesita dublu RESET la prima incarcare; ulterior
# aplicatia noua poate intra in bootloader la 1200 bps prin USB CDC.
app_port=/dev/serial/by-id/usb-Alex_Software_Racheta_Debug_001-if00
if [[ -e "$app_port" ]]; then
    python3 enter_bootloader.py "$app_port"
    sleep 2
fi

cargo hf2 --release

python3 read_imu.py "$app_port"
