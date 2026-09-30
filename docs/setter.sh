#!/bin/bash
# HKC P275MV PLUS - set once and confirm the value by retrying reads.
#
# Usage:
#   ./setter.sh 10 11  # VCP 0x10, decimal value 11 (0x000B)
#   ./setter.sh d8 1
#   ./setter.sh 61 0
#   ./setter.sh bus:13 addr:0x37 d8 1
#
# Sends DDC/CI Set VCP:
#   51 84 03 <code> <hi> <lo> <checksum>

set -euo pipefail

# The public result is exactly one line. Internal diagnostics remain silent.
finish() {
    local status=$?
    trap - EXIT
    if (( status == 0 )); then printf 'success\n'; else printf 'error\n'; fi
}
trap finish EXIT
exec 2>/dev/null

BUS=14
ADDR=0x37
DDC_DEST=0x6E
DDC_SRC=0x51
SET_CMD=0x84
SET_LEN=0x03
COMMIT_DELAY=${COMMIT_DELAY:-0.125}
# Vendor controls may use 99=00F6; it is not a confirmed persistence command.
# E0 is a plain profile selector, matching the working ddcutil command.
# DO_COMMIT=0/1 overrides this choice.
DO_COMMIT=${DO_COMMIT:-auto}
SETTLE_DELAY=${SETTLE_DELAY:-0.250}
VERIFY_READ_DELAY=${VERIFY_READ_DELAY:-0.250}
VERIFY_MAX_READS=${VERIFY_MAX_READS:-5}
RETRY_DELAY=${RETRY_DELAY:-0.250}

args=()

for arg in "$@"; do
    case "$arg" in
        bus:*) BUS="${arg#bus:}" ;;
        addr:*) ADDR="${arg#addr:}" ;;
        *) args+=("$arg") ;;
    esac
done

if (( ${#args[@]} != 2 )); then
    exit 1
fi

normalize_hex() {
    local value=${1#0x}
    value=${value#0X}
    value=${value,,}

    if [[ ! "$value" =~ ^[0-9a-f]+$ ]]; then
        echo "Invalid hex value: $1" >&2
        exit 1
    fi

    while [[ ${#value} -gt 1 && "$value" == 0* ]]; do
        value=${value#0}
    done
    if (( ${#value} > 2 )); then
        echo "VCP code out of range 0x00..0xff: $1" >&2
        exit 1
    fi
    printf "%x" "$((16#$value))"
}

format_byte() {
    local value=$1
    if (( value < 0 || value > 255 )); then
        echo "Byte out of range 0x00..0xff: $value" >&2
        exit 1
    fi

    printf "0x%02x" "$value"
}

calc_cs() {
    local chk=$DDC_DEST
    for b in "$@"; do
        chk=$(( chk ^ b ))
    done
    printf "0x%02x" "$chk"
}

code_hex=$(normalize_hex "${args[0]}")
value_decimal=${args[1]}

if [[ ! "$value_decimal" =~ ^[0-9]+$ ]]; then
    echo "Invalid decimal value: $value_decimal (expected 0..65535)" >&2
    exit 1
fi

# Remove leading zeros before checking length and parsing to avoid octal
# interpretation and integer overflow on excessively long input.
while [[ ${#value_decimal} -gt 1 && "$value_decimal" == 0* ]]; do
    value_decimal=${value_decimal#0}
done
if (( ${#value_decimal} > 5 )) || (( 10#$value_decimal > 65535 )); then
    echo "Decimal value out of range 0..65535: ${args[1]}" >&2
    exit 1
fi

code=$((16#$code_hex))
value=$((10#$value_decimal))

if (( code > 255 )); then
    echo "VCP code out of range 0x00..0xff: ${args[0]}" >&2
    exit 1
fi

if [[ "$DO_COMMIT" == "auto" ]]; then
    case "$code_hex" in
        4|5|8|39|60|cc|d6|e0|e2) DO_COMMIT=0 ;;
        *) DO_COMMIT=1 ;;
    esac
fi
if [[ "$DO_COMMIT" != "0" && "$DO_COMMIT" != "1" ]]; then
    echo "DO_COMMIT must be auto, 0, or 1" >&2
    exit 1
fi

if [[ ! "$VERIFY_MAX_READS" =~ ^[1-9][0-9]?$ ]]; then
    exit 1
fi

for option in COMMIT_DELAY SETTLE_DELAY VERIFY_READ_DELAY RETRY_DELAY; do
    if [[ ! "${!option}" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
        echo "$option must be a nonnegative number of seconds" >&2
        exit 1
    fi
done

hi=$(( (value >> 8) & 0xff ))
lo=$(( value & 0xff ))
checksum=$(calc_cs "$DDC_SRC" "$SET_CMD" "$SET_LEN" "$code" "$hi" "$lo")

write_value() {
    if ! i2ctransfer -y "$BUS" "w7@$ADDR" "$DDC_SRC" "$SET_CMD" "$SET_LEN" \
        "$(format_byte "$code")" "$(format_byte "$hi")" "$(format_byte "$lo")" "$checksum" >/dev/null; then
        LAST_ERROR="setting write failed"
        return 1
    fi
    if [[ "$DO_COMMIT" == "1" ]]; then
        sleep "$COMMIT_DELAY"
        if ! i2ctransfer -y "$BUS" "w7@$ADDR" 0x51 0x84 0x03 0x99 0x00 0xf6 0xd7 >/dev/null; then
            LAST_ERROR="99=00F6 write failed; the setting may already have changed"
            return 1
        fi
    fi
}

read_current() {
    local request_checksum response byte check
    local -a bytes
    request_checksum=$(calc_cs "$DDC_SRC" 0x82 0x01 "$code")
    if ! i2ctransfer -y "$BUS" "w5@$ADDR" "$DDC_SRC" 0x82 0x01 \
        "$(format_byte "$code")" "$request_checksum" >/dev/null; then
        LAST_ERROR="verification request failed"
        return 1
    fi
    sleep "$VERIFY_READ_DELAY"
    if ! response=$(i2ctransfer -y "$BUS" "r11@$ADDR"); then
        LAST_ERROR="verification response failed"
        return 1
    fi
    read -r -a bytes <<< "$response"
    for byte in "${bytes[@]}"; do
        if [[ ! "$byte" =~ ^0x[0-9a-fA-F]{2}$ ]]; then
            LAST_ERROR="invalid byte in verification response"
            return 1
        fi
    done
    if (( ${#bytes[@]} >= 3 )) && (( bytes[0] == 0x6e && bytes[1] == 0x80 )); then
        LAST_ERROR="NULL response; monitor did not provide a setting value"
        return 1
    fi
    if (( ${#bytes[@]} != 11 )); then
        LAST_ERROR="verification response is not 11 bytes"
        return 1
    fi
    if (( bytes[0] != 0x6e || bytes[1] != 0x88 || bytes[2] != 0x02 || bytes[4] != code )); then
        LAST_ERROR="wrong verification header or VCP code"
        return 1
    fi
    check=0x50
    for byte in "${bytes[@]}"; do check=$(( check ^ byte )); done
    if (( check != 0 )); then
        LAST_ERROR="verification checksum mismatch"
        return 1
    fi
    (( bytes[3] == 0 )) || return 1
    (( bytes[8] * 256 + bytes[9] == value )) || return 1
    return 0
}

write_value || exit 1
sleep "$SETTLE_DELAY"
for ((attempt=1; attempt<=VERIFY_MAX_READS; attempt++)); do
    if read_current; then exit 0; fi
    if (( attempt < VERIFY_MAX_READS )); then sleep "$RETRY_DELAY"; fi
done
exit 1
