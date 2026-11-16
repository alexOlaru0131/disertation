#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'Utilizare: %s [--simulation | --raspberry | --cloud]\n' "${0##*/}"
  printf '  --simulation  Porneste cloud-ul si simularea Raspberry Pi\n'
  printf '  --raspberry   Porneste doar aplicatia Raspberry Pi\n'
  printf '  --cloud       Porneste doar serverul cloud si interfata\n'
}

if (( $# != 1 )); then
  usage
  if (( $# == 0 )); then exit 0; else exit 2; fi
fi

case "$1" in
  --simulation | --raspberry | --cloud) mode=$1 ;;
  *) usage >&2; exit 2 ;;
esac

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
build_dir="$PWD/target-local"
export ROBOT_CONFIG="$PWD/config.json"

if [[ $mode != --raspberry ]]; then
  cargo build --manifest-path cloud/Cargo.toml --target-dir "$build_dir" --bin cloud
fi
if [[ $mode != --cloud ]]; then
  cargo build --manifest-path raspi/Cargo.toml --target-dir "$build_dir" --bin raspi
fi

case $mode in
  --cloud) exec "$build_dir/debug/cloud" ;;
  --raspberry) exec "$build_dir/debug/raspi" ;;
  --simulation)
    "$build_dir/debug/cloud" &
    cloud_pid=$!
    cleanup() {
      kill "$cloud_pid" 2>/dev/null || true
      wait "$cloud_pid" 2>/dev/null || true
    }
    trap cleanup EXIT

    for ((i = 0; i < 50; i++)); do
      if ! kill -0 "$cloud_pid" 2>/dev/null; then
        wait "$cloud_pid" || true
        exit 1
      fi
      if [[ -n $(ss -H -ltn 'sport = :8080') ]]; then
        "$build_dir/debug/raspi"
        wait "$cloud_pid"
        exit $?
      fi
      sleep 0.1
    done
    printf 'Cloud-ul nu a pornit pe portul 8080.\n' >&2
    exit 1
    ;;
esac
