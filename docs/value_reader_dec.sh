#!/bin/bash
# HKC P275MV PLUS - decimal VCP/i2c reader wrapper.
#
# Usage:
#   ./value_reader_dec.sh 195
#   ./value_reader_dec.sh 247 73 70
#   ./value_reader_dec.sh bus:13 addr:0x37 195 247

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
hex_args=()

for arg in "$@"; do
    case "$arg" in
        bus:*|addr:*)
            hex_args+=("$arg")
            ;;
        *)
            if [[ ! "$arg" =~ ^[0-9]+$ ]]; then
                echo "Invalid decimal VCP code: $arg" >&2
                exit 1
            fi

            if (( arg < 0 || arg > 255 )); then
                echo "Decimal VCP code out of range 0..255: $arg" >&2
                exit 1
            fi

            hex_args+=("$(printf "%02x" "$arg")")
            ;;
    esac
done

if (( ${#hex_args[@]} == 0 )); then
    echo "Usage: $0 [bus:N] [addr:0xNN] <vcp_dec> [vcp_dec...]" >&2
    echo "Example: $0 195" >&2
    echo "Example: $0 247 73 70" >&2
    exit 1
fi

exec "$script_dir/reader.sh" "${hex_args[@]}"
