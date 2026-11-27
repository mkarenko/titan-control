#!/bin/bash
# Measures the resident memory of the Linux (GTK 4) build for comparison with the Windows port.
#
# Builds the application in this repository and starts it twice inside a container-ish environment:
#   1. a normal start, which builds the settings window and then exits because no monitor is attached;
#   2. `TITAN_CHECK_UI=1` ("layout check"), which builds the whole window and quits.
# An Xvfb display provides the X connection and `GSK_RENDERER=cairo` keeps GTK off the GPU, so the numbers reflect the
# toolkit, not a driver.
#
# Usage (from a Debian 13 image with the GTK dev packages, or as root in such a container):
#   apt-get update && apt-get install -y build-essential pkg-config libgtk-4-dev libadwaita-1-dev \
#       blueprint-compiler curl ca-certificates xvfb
#   ./docs/windows/linux-baseline.sh
set -euo pipefail

REPO=$(cd "$(dirname "$0")/../.." && pwd)
cd "$REPO"

if [ ! -x "$HOME/.cargo/bin/cargo" ] && ! command -v cargo >/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal >/dev/null
  . "$HOME/.cargo/env"
fi

echo "== building =="
cargo build --release 2>&1 | tail -2
ls -l target/release/titan_control

export DISPLAY=:99
export GSK_RENDERER=cairo
Xvfb :99 -screen 0 1920x1200x24 >/dev/null 2>&1 &
XVFB=$!
sleep 2

# Samples the resident size of a process until it exits.
sample() {
  local label="$1"; shift
  "$@" >/tmp/run.log 2>&1 &
  local app=$!
  local peak=0
  local last=0
  for _ in $(seq 1 80); do
    if [ -r "/proc/$app/status" ]; then
      local rss
      rss=$(awk '/VmRSS/ {print $2}' /proc/$app/status)
      last=${rss:-$last}
      if [ "${rss:-0}" -gt "$peak" ]; then peak=$rss; fi
    else
      break
    fi
    sleep 0.1
  done
  wait $app 2>/dev/null || true
  echo "$label: peak rss = ${peak} kB, last sample = ${last} kB"
  tail -2 /tmp/run.log | sed 's/^/    /'
}

echo "== normal start (settings window, no monitor attached) =="
sample "normal" env TITAN_CHECK_LANG=en ./target/release/titan_control

echo "== layout check (whole window built) =="
sample "layout" env TITAN_CHECK_UI=1 TITAN_CHECK_LANG=en ./target/release/titan_control

kill $XVFB 2>/dev/null || true
