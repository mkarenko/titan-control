#!/bin/bash
# HKC P275MV PLUS - single scene/profile table reader
#
# Usage:
#   ./profile_reader.sh 1
#   ./profile_reader.sh sceneId:1
#   ./profile_reader.sh arg=sceneId:1 bus:13 addr:0x37 maxPos:40
#
# Notes:
#   This uses only VCP Get frames, but on this monitor the table-read sequence
#   can still preview/load the requested profile on screen.

BUS=15
ADDR=0x37
SCENE_ID=""
MAX_POS=40
RETRIES=${RETRIES:-1}
RETRY_DELAY=${RETRY_DELAY:-0.35}
VCP_READ_DELAY=${VCP_READ_DELAY:-0.25}
DEBUG_DDC=${DEBUG_DDC:-0}
ALLOW_INVALID=${ALLOW_INVALID:-0}
IGNORE_RAW_ZERO=${IGNORE_RAW_ZERO:-0}

RED='\033[0;31m'; GRN='\033[0;32m'; DIM='\033[2m'; RST='\033[0m'
BOLD='\033[1m'

for arg in "$@"; do
    arg="${arg#arg=}"
    case "$arg" in
        [0-9]*) SCENE_ID="$arg" ;;
        sceneId:*) SCENE_ID="${arg#sceneId:}" ;;
        bus:*) BUS="${arg#bus:}" ;;
        addr:*) ADDR="${arg#addr:}" ;;
        maxPos:*) MAX_POS="${arg#maxPos:}" ;;
        debug:*) DEBUG_DDC="${arg#debug:}" ;;
        *)
            echo "Unknown arg: $arg"
            echo "Usage: $0 sceneId:N [bus:N] [addr:0xNN] [maxPos:N]"
            exit 1
            ;;
    esac
done

if [[ ! "$SCENE_ID" =~ ^[0-9]+$ ]] || (( SCENE_ID < 0 || SCENE_ID > 255 )); then
    echo "Missing or invalid sceneId. Example: $0 1"
    exit 1
fi

if [[ ! "$MAX_POS" =~ ^[0-9]+$ ]] || (( MAX_POS < 0 || MAX_POS > 255 )); then
    echo "Invalid maxPos: $MAX_POS"
    exit 1
fi

calc_cs() {
    local chk=0x6E
    for b in "$@"; do chk=$(( chk ^ b )); done
    printf "0x%02X" "$chk"
}

_VCP_MAX=0
_VCP_CUR=0
_VCP_RAW=""
WARNINGS=()
TABLE_LAYOUT="unknown"

vcp_get() {
    local code=$1 code_hex cs raw
    code_hex=$(printf "0x%02X" "$code")
    cs=$(calc_cs 0x51 0x82 0x01 "$code")

    i2ctransfer -y "$BUS" w5@"$ADDR" 0x51 0x82 0x01 "$code_hex" "$cs" 2>/dev/null
    sleep "$VCP_READ_DELAY"
    raw=$(i2ctransfer -y "$BUS" r11@"$ADDR" 2>/dev/null)
    _VCP_RAW="$raw"
    (( DEBUG_DDC != 0 )) && printf "debug get 0x%02X raw: %s\n" "$code" "$raw" >&2

    local -a b
    read -r -a b <<< "$raw"

    (( ${#b[@]} < 11 )) && return 1
    [[ "${b[0],,}" != "0x6e" ]] && return 1
    [[ "${b[2],,}" != "0x02" ]] && return 1
    [[ "${b[3],,}" != "0x00" ]] && return 1
    (( ${b[4]} == (code & 0xFF) )) || return 1

    _VCP_MAX=$(( ${b[6]} * 256 + ${b[7]} ))
    _VCP_CUR=$(( ${b[8]} * 256 + ${b[9]} ))
    return 0
}

vcp_get_retry() {
    local code=$1
    for (( attempt=1; attempt <= RETRIES; attempt++ )); do
        vcp_get "$code" && return 0
        (( attempt < RETRIES )) && sleep "$RETRY_DELAY"
    done
    return 1
}

is_color_temp_value() {
    case $1 in
        5|6|8|11|12|13) return 0 ;;
        *) return 1 ;;
    esac
}

detect_table_layout() {
    if is_color_temp_value "${buffer[7]}"; then
        TABLE_LAYOUT="with_profile_id"
    elif is_color_temp_value "${buffer[8]}"; then
        TABLE_LAYOUT="without_profile_id"
    else
        TABLE_LAYOUT="unknown"
    fi
}

valid_table_core() {
    case "$TABLE_LAYOUT" in
        with_profile_id)
            (( buffer[2] >= 1 && buffer[2] <= 0x1E )) || return 1
            (( buffer[3] <= 100 )) || return 1
            (( buffer[4] <= 100 )) || return 1
            (( buffer[6] <= 5 )) || return 1
            is_color_temp_value "${buffer[7]}" || return 1
            ;;
        without_profile_id)
            (( buffer[2] <= 100 )) || return 1
            (( buffer[3] <= 100 )) || return 1
            (( buffer[5] <= 5 )) || return 1
            is_color_temp_value "${buffer[8]}" || return 1
            ;;
        *)
            return 1
            ;;
    esac
    return 0
}

looks_like_empty_chunk() {
    local offset=$1
    (( buffer[offset] == 0x51 && buffer[offset + 1] == 0x01 && buffer[offset + 2] == 0x00 && buffer[offset + 3] == 0x00 ))
}

canonical_raw() {
    local pos=$1

    if [[ "$TABLE_LAYOUT" == "with_profile_id" ]]; then
        case $pos in
            2) echo "${buffer[3]}" ;;
            3) echo "${buffer[4]}" ;;
            4) echo "${buffer[6]}" ;;
            5) echo 0 ;;
            6) echo 0 ;;
            7) echo 50 ;;
            8) echo "${buffer[7]}" ;;
            9) echo "${buffer[8]}" ;;
            10) echo "${buffer[9]}" ;;
            11) echo "${buffer[10]}" ;;
            12) echo "${buffer[11]}" ;;
            13) echo "${buffer[12]}" ;;
            14) echo "${buffer[13]}" ;;
            15) echo "${buffer[14]}" ;;
            16) echo "${buffer[15]}" ;;
            17) echo "${buffer[16]}" ;;
            18) echo "${buffer[17]}" ;;
            19) echo "${buffer[18]}" ;;
            20) echo "${buffer[19]}" ;;
            21) echo "${buffer[20]}" ;;
            22) echo "${buffer[21]}" ;;
            23) echo "${buffer[22]}" ;;
            24) echo "${buffer[23]}" ;;
            25) echo "${buffer[24]}" ;;
            26) echo "${buffer[25]}" ;;
            27) echo "${buffer[26]}" ;;
            28) echo "${buffer[27]}" ;;
            29) echo "${buffer[28]}" ;;
            30) echo "${buffer[30]}" ;;
            31) echo 0 ;;
            32) echo 0 ;;
            33) echo "${buffer[32]}" ;;
            34) echo "${buffer[33]}" ;;
            35) echo "${buffer[34]}" ;;
            36) echo "${buffer[35]}" ;;
            37) echo "${buffer[36]}" ;;
            *) echo "${buffer[$pos]}" ;;
        esac
    else
        echo "${buffer[$pos]}"
    fi
}

field_label_display() {
    case $1 in
            2) echo "Brightness" ;;
            3) echo "Contrast" ;;
            4) echo "Sharpness" ;;
            5) echo "Color Enhancement" ;;
            6) echo "CR Enhancement" ;;
            7) echo "Shadow Balance" ;;
            8) echo "Color Temperature" ;;
            9) echo "Color Temp R User 1" ;;
            10) echo "Color Temp B User 1" ;;
            11) echo "Color Temp Y User 1" ;;
            12) echo "Color Temp R User 2" ;;
            13) echo "Color Temp B User 2" ;;
            14) echo "Color Temp Y User 2" ;;
            15) echo "Color Temp R User 3" ;;
            16) echo "Color Temp B User 3" ;;
            17) echo "Color Temp Y User 3" ;;
            18) echo "Hue R" ;;
            19) echo "Hue G" ;;
            20) echo "Hue B" ;;
            21) echo "Hue Y" ;;
            22) echo "Hue C" ;;
            23) echo "Hue M" ;;
            24) echo "Saturation R" ;;
            25) echo "Saturation G" ;;
            26) echo "Saturation B" ;;
            27) echo "Saturation Y" ;;
            28) echo "Saturation C" ;;
            29) echo "Saturation M" ;;
            30) echo "Low Blue Light" ;;
            31) echo "HDR" ;;
            33) echo "Gamma" ;;
            34) echo "Super Resolution" ;;
            35) echo "Night Vision" ;;
            36) echo "Dynamic OD" ;;
            37) echo "Local Dimming" ;;
            *) echo "-" ;;
        esac
}

field_decode() {
    local pos=$1 val=$2
    case $pos in
            2|3) printf "%d" "$val" ;;
            4) printf "%d (0-5)" "$val" ;;
            5) printf "%d (0-10)" "$val" ;;
            6) printf "%d (0-5)" "$val" ;;
            7) printf "%d" "$val" ;;
            8)
                case $val in
                    5) echo "Warm" ;;
                    6) echo "Natural" ;;
                    8) echo "Cool" ;;
                    11) echo "User 1" ;;
                    12) echo "User 2" ;;
                    13) echo "User 3" ;;
                    *) printf "0x%02X" "$val" ;;
                esac
                ;;
            9|10|11|12|13|14|15|16|17) printf "%d" "$val" ;;
            18|19|20|21|22|23|24|25|26|27|28|29) printf "%d" "$val" ;;
            30) printf "%d (0-100, 25-step)" "$val" ;;
            31) case $val in 0) echo "Off" ;; 1) echo "Auto" ;; 2) echo "Game" ;; 3) echo "Movie" ;; *) printf "%d" "$val" ;; esac ;;
            33)
                case $val in
                    2) echo "1.8" ;;
                    4) echo "2.0" ;;
                    6) echo "2.2" ;;
                    8) echo "2.4" ;;
                    10) echo "2.6" ;;
                    12) echo "S-Curve" ;;
                    *) printf "%d" "$val" ;;
                esac
                ;;
            34) (( val == 0 )) && echo "Off" || printf "Level %d (0-5)" "$val" ;;
            35) case $val in 0) echo "Off" ;; 1) echo "Level 1" ;; 2) echo "Level 2" ;; 3) echo "Auto-Level 1" ;; 4) echo "Auto-Level 2" ;; *) printf "%d" "$val" ;; esac ;;
            36) case $val in 0) echo "Off" ;; 1) echo "Level 1" ;; 2) echo "Level 2" ;; 3) echo "Level 3" ;; 4) echo "Topspeed" ;; *) printf "%d" "$val" ;; esac ;;
            37) case $val in 0) echo "Off" ;; 1) echo "Low" ;; 2) echo "Smooth" ;; 3) echo "Middle" ;; 6) echo "High" ;; *) printf "%d" "$val" ;; esac ;;
            *) printf "%d" "$val" ;;
        esac
}

read_scene() {
    local -n out=$1
    local offset=6 chunks_needed chunk_code

    for (( i=0; i<=MAX_POS; i++ )); do out[$i]=0; done

    if ! vcp_get_retry 0x99; then
        WARNINGS+=("get 0x99 failed, continuing")
    fi
    vcp_get_retry 0x22 || { echo "FAIL get 0x22"; return 1; }

    if ! vcp_get_retry "$SCENE_ID"; then
        printf "FAIL get sceneId %d\n" "$SCENE_ID"
        return 1
    fi

    out[2]=$(( (_VCP_MAX >> 8) & 0xFF ))
    out[3]=$(( _VCP_MAX & 0xFF ))
    out[4]=$(( (_VCP_CUR >> 8) & 0xFF ))
    out[5]=$(( _VCP_CUR & 0xFF ))

    offset=6
    chunks_needed=$(( (MAX_POS - offset + 4) / 4 ))
    for (( i=0; i<chunks_needed; i++ )); do
        chunk_code=0xFE
        if ! vcp_get_retry "$chunk_code"; then
            chunk_code=0xFF
            vcp_get_retry "$chunk_code" || {
                printf "FAIL get chunk %d at pos %02d\n" "$i" "$offset"
                return 1
            }
        fi

        (( offset <= MAX_POS )) && out[$offset]=$(( (_VCP_MAX >> 8) & 0xFF ))
        (( offset + 1 <= MAX_POS )) && out[$((offset+1))]=$(( _VCP_MAX & 0xFF ))
        (( offset + 2 <= MAX_POS )) && out[$((offset+2))]=$(( (_VCP_CUR >> 8) & 0xFF ))
        (( offset + 3 <= MAX_POS )) && out[$((offset+3))]=$(( _VCP_CUR & 0xFF ))
        offset=$(( offset + 4 ))
    done

    vcp_get 0xFE >/dev/null 2>&1
    return 0
}

if ! i2ctransfer -y "$BUS" r1@"$ADDR" >/dev/null 2>&1; then
    echo "No I2C access: bus=$BUS addr=$ADDR"
    exit 1
fi

declare -a buffer
read_scene buffer || exit 1
detect_table_layout
if ! valid_table_core; then
    printf "SceneId: %d  Bus: %s  Addr: %s\n" "$SCENE_ID" "$BUS" "$ADDR"
    printf "Layout: %s\n" "$TABLE_LAYOUT"
    for warning in "${WARNINGS[@]}"; do
        printf "Warning: %s\n" "$warning"
    done
    printf "Hex:"
    for (( i=0; i<=MAX_POS; i++ )); do printf " %02X" "${buffer[$i]}"; done
    printf "\n"
    if (( ALLOW_INVALID == 0 )); then
        echo "FAIL invalid table core; retry this scene"
        exit 1
    fi
    echo "Warning: invalid table core, printing anyway"
fi

printf "SceneId: %d  Bus: %s  Addr: %s\n" "$SCENE_ID" "$BUS" "$ADDR"
printf "Layout: %s\n" "$TABLE_LAYOUT"
for warning in "${WARNINGS[@]}"; do
    printf "Warning: %s\n" "$warning"
done
printf "Hex:"
for (( i=0; i<=MAX_POS; i++ )); do printf " %02X" "${buffer[$i]}"; done
printf "\n\n"

printf "  ${BOLD}%-5s %-20s %5s  %-20s${RST}\n" "Pos" "Setting" "Raw" "Decoded"
printf "  ─────────────────────────────────────────────────────────\n"

for (( i=0; i<=MAX_POS; i++ )); do
    raw=$(canonical_raw "$i")
    (( IGNORE_RAW_ZERO != 0 && raw == 0 )) && continue
    if [[ "$TABLE_LAYOUT" != "with_profile_id" ]] && (( i + 3 <= MAX_POS )) && looks_like_empty_chunk "$i"; then
        printf "  ${DIM}[%02d] %-20s %5s  %s${RST}\n" "$i" "end/empty chunk" "-" "51 01 00 00"
        i=$(( i + 3 ))
        continue
    fi
    label=$(field_label_display "$i")
    decoded=$(field_decode "$i" "$raw")
    if [[ "$label" == "-" ]]; then
        printf "  ${DIM}[%02d] %-20s %5d  %s${RST}\n" "$i" "-" "$raw" "$decoded"
    else
        printf "  ${GRN}[%02d] %-20s %5d  %s${RST}\n" "$i" "$label" "$raw" "$decoded"
    fi
done
