#!/usr/bin/env bash
clear

set -euo pipefail

cargo build --release

app_port=/dev/serial/by-id/usb-Alex_Software_Racheta_Debug_001-if00
if [[ -e "$app_port" ]]; then
  python3 enter_bootloader.py "$app_port"
  sleep 2
fi

cargo hf2 --release

python3 read_imu.py "$app_port"

sleep 5s

for i in $(seq 1 10); do
  python3 test_pca9685.py "$((i * 1000))"
  sleep 0.5
done

sleep 5s

python3 test_led.py --no-confirm
