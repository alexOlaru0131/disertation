#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
build_dir="$PWD/target-local"
cloud_bin=cloud

cargo build --manifest-path cloud/Cargo.toml \
  --target-dir "$build_dir" --bin "$cloud_bin"

cargo build --manifest-path raspi/Cargo.toml \
  --target-dir "$build_dir" --bin raspi

port_listening() {
  [[ -n $(ss -H -ltn 'sport = :8080') ]]
}

"$build_dir/debug/$cloud_bin" &
cloud_pid=$!

cleanup() {
  kill "$cloud_pid" 2>/dev/null || true
  wait "$cloud_pid" 2>/dev/null || true
}
trap cleanup EXIT

for ((i = 0; i < 50; i++)); do
  if port_listening; then
    "$build_dir/debug/raspi" ws://127.0.0.1:8080
    wait "$cloud_pid"
    exit $?
  fi
  sleep 0.1
done

exit 1
