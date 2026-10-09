#!/usr/bin/env bash
set -euo pipefail

PROJECT="$HOME/disertation/cod_stm_l452re-p"
BIN="${1:-$PROJECT/build/Debug/cod_stm_l452re-p.bin}"
ELF="${1:-$PROJECT/build/Debug/cod_stm_l452re-p.elf}"
CLI="$HOME/STMicroelectronics/STM32Cube/STM32CubeProgrammer/bin/STM32_Programmer_CLI"

UART_PORT="${UART_PORT:-/dev/ttyACM0}"
python3 - "$UART_PORT" <<'PY'
import os
import sys
import termios

fd = os.open(sys.argv[1], os.O_RDWR | os.O_NOCTTY)
try:
    termios.tcsendbreak(fd, 0)
finally:
    os.close(fd)
PY

cmake --build cod_stm_l452re-p/build/Debug/
arm-none-eabi-objcopy -O binary "$ELF" "$BIN"
"$CLI" -c port=SWD mode=UR reset=HWrst freq=1000 \
  -d "$BIN" 0x08000000 -v -rst
