#!/bin/bash
# HKC P275MV PLUS – Custom Table Reader v10
# ⚠️  Brak zapisu profili — domyślnie emuluje DDCCIReadTable producenta
#
# Użycie: sudo ./hkc_custom_table.sh [bus] [addr] [mode]
#   mode: all       = wszystkie znane tryby (default + custom)
#         custom    = tylko custom każdego trybu
#         default   = tylko default każdego trybu
#         N         = konkretny profile_id/VCP 0x22 z tabeli, np. 2=Standard default, 3=Standard custom
#         id:N      = surowy profile_id N, kompaktowy output
#         brak      = custom (domyślnie)
#
#   VCP_SELECT_PREFIX=0x01  prefiks selektora 0x99 dla odczytu (wg VIEW MORE WIDGET)
#   TABLE_USE_SELECTOR=1    eksperymentalnie użyj SetVCP 0x99 przed odczytem tabeli
#   RESTORE_AFTER_READ=0    nie przywracaj aktywnego profilu po odczycie

BUS=${1:-15}
ADDR=${2:-0x37}
MODE=${3:-custom}
RETRIES=1
RETRY_DELAY=0.35
SCENE_RETRIES=1
VCP_READ_DELAY=0.25
VCP_SELECT_DELAY=0.10
VCP_SELECT_PREFIX=${VCP_SELECT_PREFIX:-0x01}
VALIDATE_STRICT=${VALIDATE_STRICT:-0}
DEBUG_DDC=${DEBUG_DDC:-0}
TABLE_USE_SELECTOR=${TABLE_USE_SELECTOR:-0}
RESTORE_AFTER_READ=${RESTORE_AFTER_READ:-1}

RED='\033[0;31m'; GRN='\033[0;32m'; YEL='\033[0;33m'
CYN='\033[0;36m'; DIM='\033[2m'; RST='\033[0m'
BOLD='\033[1m'

MODE_NAMES=(
    "Standard"
    "DyDs/ULL FPS"
    "DyDs/LD"
    "RTS/RPG"
    "FPS"
    "MOBA"
    "Movie"
    "Reading"
    "Night"
    "Eye Care"
    "Mac View"
    "E-Book"
    "sRGB"
    "Adobe"
    "DCI-P3"
)

# Profile IDs dla VCP 0x22 z docs/p275mv_plus.txt.
# -1 oznacza, ze nie mamy potwierdzonego ID.
MODE_DEFAULT=(   2   27  29   4   6   7   9  11  13  15  17  19  21  23  25 )
MODE_CUSTOM=(    3   28  30   5  -1   8  10  12  14  16  18  20  22  24  26 )

declare -A SCENE_LABEL
for (( m=0; m<15; m++ )); do
    default_id=${MODE_DEFAULT[$m]}
    custom_id=${MODE_CUSTOM[$m]}

    if (( default_id >= 0 )) && [[ -n "${SCENE_LABEL[$default_id]}" ]]; then
        SCENE_LABEL[$default_id]+=" / ${MODE_NAMES[$m]} — default"
    elif (( default_id >= 0 )); then
        SCENE_LABEL[$default_id]="${MODE_NAMES[$m]} — default"
    fi

    if (( custom_id >= 0 )) && [[ -n "${SCENE_LABEL[$custom_id]}" ]]; then
        SCENE_LABEL[$custom_id]+=" / ${MODE_NAMES[$m]} — custom"
    elif (( custom_id >= 0 )); then
        SCENE_LABEL[$custom_id]="${MODE_NAMES[$m]} — custom"
    fi
done

calc_cs() {
    local chk=0x6E
    for b in "$@"; do chk=$(( chk ^ b )); done
    printf "0x%02X" "$chk"
}

_VCP_OK=0; _VCP_MAX=0; _VCP_CUR=0; _VCP_RAW=""; _VCP_REPLY_CODE=0
FAIL_STAGE=""
VALIDATION_WARNING=""

vcp_set() {
    local code=$1
    local value=$2
    local code_hex hi lo cs

    code_hex=$(printf "0x%02X" "$code")
    hi=$(printf "0x%02X" $(( (value >> 8) & 0xFF )))
    lo=$(printf "0x%02X" $(( value & 0xFF )))
    cs=$(calc_cs 0x51 0x84 0x03 "$code" "$hi" "$lo")

    i2ctransfer -y "$BUS" w7@"$ADDR" \
        0x51 0x84 0x03 "$code_hex" "$hi" "$lo" "$cs" 2>/dev/null
}

vcp_select() {
    local code=$1
    local value=$(( (VCP_SELECT_PREFIX << 8) | (code & 0xFF) ))
    vcp_set 0x99 "$value" || return 1
    sleep "$VCP_SELECT_DELAY"
}

vcp_select_retry() {
    local code=$1
    for (( attempt=1; attempt <= RETRIES; attempt++ )); do
        vcp_select "$code" && return 0
        (( attempt < RETRIES )) && sleep "$RETRY_DELAY"
    done
    return 1
}

restore_profile_if_needed() {
    local profile_id=$1
    (( RESTORE_AFTER_READ != 0 )) || return 0
    (( profile_id >= 1 && profile_id <= 0x1E )) || return 0
    vcp_set 0x22 "$profile_id" >/dev/null 2>&1 || return 1
    sleep 0.35
}

vcp_get() {
    local code=$1
    local code_hex
    code_hex=$(printf "0x%02X" "$code")
    local cs
    cs=$(calc_cs 0x51 0x82 0x01 "$code")
    i2ctransfer -y "$BUS" w5@"$ADDR" \
        0x51 0x82 0x01 "$code_hex" "$cs" 2>/dev/null
    sleep "$VCP_READ_DELAY"
    local raw
    raw=$(i2ctransfer -y "$BUS" r11@"$ADDR" 2>/dev/null)
    _VCP_RAW="$raw"
    (( DEBUG_DDC != 0 )) && printf "debug get 0x%02X raw: %s\n" "$code" "$raw" >&2
    local -a b
    read -r -a b <<< "$raw"

    _VCP_OK=0; _VCP_MAX=0; _VCP_CUR=0; _VCP_REPLY_CODE=0
    (( ${#b[@]} < 11 )) && return 1
    [[ "${b[0],,}" != "0x6e" ]] && return 1
    [[ "${b[2]}" != "0x02" ]] && return 1
    [[ "${b[3]}" != "0x00" ]] && return 1
    _VCP_REPLY_CODE=$(( ${b[4]} ))
    (( _VCP_REPLY_CODE == (code & 0xFF) )) || return 1
    _VCP_OK=1
    _VCP_MAX=$(( ${b[6]} * 256 + ${b[7]} ))
    _VCP_CUR=$(( ${b[8]} * 256 + ${b[9]} ))
    return 0
}

reset_sequence() {
    vcp_get 0x99 >/dev/null 2>&1
    sleep 0.20
    vcp_get 0xFE >/dev/null 2>&1
    sleep 0.20
}

vcp_get_retry() {
    local code=$1
    for (( attempt=1; attempt <= RETRIES; attempt++ )); do
        vcp_get "$code" && return 0
        (( attempt < RETRIES )) && sleep "$RETRY_DELAY"
    done
    return 1
}

valid_one_of() {
    local value=$1
    shift
    local candidate
    for candidate in "$@"; do
        (( value == candidate )) && return 0
    done
    return 1
}

valid_range() {
    local value=$1 min=$2 max=$3
    (( value >= min && value <= max ))
}

validate_scene_buffer() {
    valid_range "${buffer[3]}" 0 100 || return 1   # brightness
    valid_range "${buffer[4]}" 0 100 || return 1   # contrast
    valid_range "${buffer[6]}" 0 5 || return 1     # sharpness
    valid_range "${buffer[17]}" 0 100 || return 1
    valid_range "${buffer[18]}" 0 100 || return 1
    valid_range "${buffer[19]}" 0 100 || return 1
    valid_range "${buffer[20]}" 0 100 || return 1
    valid_range "${buffer[21]}" 0 100 || return 1
    valid_range "${buffer[22]}" 0 100 || return 1
    valid_range "${buffer[23]}" 0 100 || return 1
    valid_range "${buffer[24]}" 0 100 || return 1
    valid_range "${buffer[25]}" 0 100 || return 1
    valid_range "${buffer[26]}" 0 100 || return 1
    valid_range "${buffer[27]}" 0 100 || return 1
    valid_range "${buffer[28]}" 0 100 || return 1
    valid_range "${buffer[30]}" 0 4 || return 1    # low blue
    valid_range "${buffer[31]}" 0 4 || return 1    # aspect
    return 0
}

field_label() {
    case $1 in
        0)   echo "header_0";;
        1)   echo "header_1";;
        2)   echo "profile_id";;
        3)   echo "brightness";;
        4)   echo "contrast";;
        5)   echo "dcr";;
        6)   echo "sharpness";;
        7)   echo "color_temp";;
        8)   echo "unknown_08";;
        9)   echo "unknown_09";;
        10)  echo "unknown_10";;
        11)  echo "unknown_11";;
        12)  echo "unknown_12";;
        13)  echo "unknown_13";;
        14)  echo "unknown_14";;
        15)  echo "unknown_15";;
        16)  echo "unknown_16";;
        17)  echo "hue_r";;
        18)  echo "hue_g";;
        19)  echo "hue_b";;
        20)  echo "hue_y";;
        21)  echo "hue_c";;
        22)  echo "hue_m";;
        23)  echo "sat_r";;
        24)  echo "sat_g";;
        25)  echo "sat_b";;
        26)  echo "sat_y";;
        27)  echo "sat_c";;
        28)  echo "sat_m";;
        29)  echo "eyeshield";;
        30)  echo "low_blue";;
        31)  echo "aspect";;
        32)  echo "gamma";;
        33)  echo "unknown_33";;
        34)  echo "unknown_34";;
        35)  echo "unknown_35";;
        36)  echo "unknown_36";;
        37)  echo "unknown_37";;
    esac
}

field_label_display() {
    case $1 in
        0|1) echo "";;
        2)   echo "Profile ID";;
        3)   echo "Brightness";;
        4)   echo "Contrast";;
        5)   echo "DCR";;
        6)   echo "Sharpness";;
        7)   echo "Color Temperature";;
        8|9|10|11|12|13|14|15|16) echo "?";;
        17)  echo "Hue R";;
        18)  echo "Hue G";;
        19)  echo "Hue B";;
        20)  echo "Hue Y";;
        21)  echo "Hue C";;
        22)  echo "Hue M";;
        23)  echo "Saturation R";;
        24)  echo "Saturation G";;
        25)  echo "Saturation B";;
        26)  echo "Saturation Y";;
        27)  echo "Saturation C";;
        28)  echo "Saturation M";;
        29)  echo "Eyeshield Reminder";;
        30)  echo "Low Blue Light";;
        31)  echo "Aspect Ratio";;
        32)  echo "Gamma";;
        33|34|35|36|37) echo "?";;
    esac
}

field_decode() {
    local pos=$1 val=$2
    case $pos in
        2|3|4) printf "%d" "$val";;
        5)   (( val == 0 )) && echo "Off" || echo "On";;
        6)   printf "%d (0-5)" "$val";;
        7)   case $val in
                5) echo "Warm";; 6) echo "Natural";; 8) echo "Cool";;
                11) echo "User 1";; 12) echo "User 2";; 13) echo "User 3";;
                *) printf "0x%02X" "$val";;
             esac;;
        17|18|19|20|21|22) printf "%d" "$val";;
        23|24|25|26|27|28) printf "%d" "$val";;
        29)  case $val in
                0) echo "Off";; 1) echo "30min";; 2) echo "1h";; 3) echo "1.5h";;
                4) echo "2h";; 5) echo "2.5h";; 6) echo "3h";; 7) echo "3.5h";; 8) echo "4h";;
                *) printf "%d" "$val";;
             esac;;
        30)  printf "Level %d (0-4)" "$val";;
        31)  case $val in 1) echo "Wide";; 2) echo "4:3";; 4) echo "4";; *) printf "%d" "$val";; esac;;
        32)  case $val in
                2) echo "1.8";; 4) echo "2.0";; 6) echo "2.2";; 8) echo "2.4";;
                10) echo "2.6";; 12) echo "S-Curve";; *) printf "%d" "$val";;
             esac;;
        *)   printf "%d" "$val";;
    esac
}

read_scene() {
    local scene_id=$1
    local restore_profile_id=-1
    FAIL_STAGE=""
    VALIDATION_WARNING=""
    for (( i=0; i<38; i++ )); do buffer[$i]=0; done

    reset_sequence
    if ! vcp_get_retry 0x99; then
        FAIL_STAGE="get 0x99"
        return 1
    fi
    if (( TABLE_USE_SELECTOR != 0 )) && ! vcp_select_retry 0x22; then
        FAIL_STAGE="select 0x22 via 0x99"
        return 1
    fi
    if ! vcp_get_retry 0x22; then
        FAIL_STAGE="get 0x22"
        return 1
    fi
    restore_profile_id=$_VCP_CUR

    if (( TABLE_USE_SELECTOR != 0 )) && ! vcp_select_retry "$scene_id"; then
        FAIL_STAGE="select scene $scene_id via 0x99"
        restore_profile_if_needed "$restore_profile_id"
        return 1
    fi
    if vcp_get_retry "$scene_id"; then
        buffer[2]=$(( (_VCP_MAX >> 8) & 0xFF ))
        buffer[3]=$(( _VCP_MAX & 0xFF ))
        buffer[4]=$(( (_VCP_CUR >> 8) & 0xFF ))
        buffer[5]=$(( _VCP_CUR & 0xFF ))
    else
        FAIL_STAGE="get scene $scene_id"
        restore_profile_if_needed "$restore_profile_id"
        return 1
    fi

    if (( TABLE_USE_SELECTOR != 0 )) && ! vcp_select_retry 0xFE; then
        FAIL_STAGE="select 0xFE via 0x99"
        restore_profile_if_needed "$restore_profile_id"
        return 1
    fi
    local offset=6 fe_fail=0
    for (( i=0; i<8; i++ )); do
        if vcp_get_retry 0xFE || vcp_get_retry 0xFF; then
            (( offset   < 38 )) && buffer[$offset]=$(( (_VCP_MAX >> 8) & 0xFF ))
            (( offset+1 < 38 )) && buffer[$((offset+1))]=$(( _VCP_MAX & 0xFF ))
            (( offset+2 < 38 )) && buffer[$((offset+2))]=$(( (_VCP_CUR >> 8) & 0xFF ))
            (( offset+3 < 38 )) && buffer[$((offset+3))]=$(( _VCP_CUR & 0xFF ))
        else
            fe_fail=$((fe_fail + 1))
        fi
        offset=$((offset + 4))
    done

    vcp_get 0xFE >/dev/null 2>&1
    if (( fe_fail > 2 )); then
        FAIL_STAGE="get 0xFE chunks ($fe_fail failed)"
        restore_profile_if_needed "$restore_profile_id"
        return 1
    fi
    if ! validate_scene_buffer; then
        VALIDATION_WARNING="buffer outside known ranges"
        if (( VALIDATE_STRICT != 0 )); then
            FAIL_STAGE="validate buffer"
            restore_profile_if_needed "$restore_profile_id"
            return 1
        fi
    fi
    restore_profile_if_needed "$restore_profile_id"
    return 0
}

read_and_print_compact() {
    local scene_id=$1
    local label="${SCENE_LABEL[$scene_id]:-scene $scene_id}"

    declare -a buffer
    local success=false

    for (( try=1; try <= SCENE_RETRIES; try++ )); do
        if read_scene "$scene_id"; then
            success=true; break
        fi
        if (( try < SCENE_RETRIES )); then
            reset_sequence
            sleep 0.50
        fi
    done

    if ! $success; then
        echo "[$scene_id] $label: FAIL ${FAIL_STAGE:-unknown}"
        return
    fi

    echo "[$scene_id] $label"
    [[ -n "$VALIDATION_WARNING" ]] && echo "  warning: $VALIDATION_WARNING"

    printf "  hex:"
    for (( i=0; i<38; i++ )); do printf " %02X" "${buffer[$i]}"; done
    printf "\n"

    for (( i=0; i<38; i++ )); do
        local fl val decoded
        fl=$(field_label "$i")
        val=${buffer[$i]}
        decoded=$(field_decode "$i" "$val")
        printf "  %02d %-14s %3d  %s\n" "$i" "$fl" "$val" "$decoded"
    done
}

read_and_print_full() {
    local scene_id=$1
    local label="${SCENE_LABEL[$scene_id]:-scene $scene_id}"

    echo -e "${CYN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RST}"
    echo -e "${BOLD}  [$scene_id] $label${RST}"
    echo -e "${CYN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RST}"

    declare -a buffer
    local success=false

    for (( try=1; try <= SCENE_RETRIES; try++ )); do
        if read_scene "$scene_id"; then
            success=true
            (( try > 1 )) && echo -e "  ${YEL}(udało się z próby #$try)${RST}"
            break
        else
            if (( try < SCENE_RETRIES )); then
                echo -e "  ${YEL}próba $try/$SCENE_RETRIES fail — retry...${RST}"
                reset_sequence
                sleep 0.50
            fi
        fi
    done

    if ! $success; then
        echo -e "  ${RED}FAIL po $SCENE_RETRIES próbach: ${FAIL_STAGE:-unknown}${RST}\n"
        return
    fi

    if [[ -n "$VALIDATION_WARNING" ]]; then
        echo -e "  ${YEL}warning: $VALIDATION_WARNING${RST}"
    fi

    printf "  ${DIM}Hex:"
    for (( i=0; i<38; i++ )); do printf " %02X" "${buffer[$i]}"; done
    printf "${RST}\n\n"

    all_zero=true
    for (( i=2; i<38; i++ )); do
        (( ${buffer[$i]} != 0 )) && all_zero=false && break
    done
    if $all_zero; then
        echo -e "  ${DIM}(bufor pusty)${RST}\n"
        return
    fi

    printf "  ${BOLD}%-5s %-20s %5s  %-20s${RST}\n" "Pos" "Setting" "Raw" "Decoded"
    printf "  ─────────────────────────────────────────────────────────\n"

    for (( i=2; i<38; i++ )); do
        val=${buffer[$i]}
        label=$(field_label_display "$i")
        decoded=$(field_decode "$i" "$val")
        [[ -z "$label" ]] && continue
        if [[ "$label" == "?" ]]; then
            printf "  ${DIM}[%02d] %-20s %5d  %s${RST}\n" "$i" "—" "$val" "$decoded"
        else
            printf "  ${GRN}[%02d] %-20s %5d  %s${RST}\n" "$i" "$label" "$val" "$decoded"
        fi
    done
    echo ""
}

if [[ "$MODE" =~ ^id:([0-9]+)$ ]]; then
    sid="${BASH_REMATCH[1]}"
    if ! i2ctransfer -y "$BUS" r1@"$ADDR" >/dev/null 2>&1; then
        echo "FAIL no_i2c"; exit 1
    fi
    read_and_print_compact "$sid"
    exit 0
fi


echo -e "${CYN}╔═══════════════════════════════════════════════════════════╗"
echo -e "║  HKC P275MV PLUS – Custom Table Reader v10               ║"
echo -e "║  ⚠️  Brak zapisu profili — DDCCIReadTable-style Get        ║"
echo -e "║  Bus=$BUS  Addr=$ADDR  Mode=$MODE                           ║"
echo -e "║  SelectPrefix=$VCP_SELECT_PREFIX                                         ║"
echo -e "║  RestoreAfterRead=$RESTORE_AFTER_READ                                      ║"
echo -e "╚═══════════════════════════════════════════════════════════╝${RST}"
echo ""

if ! i2ctransfer -y "$BUS" r1@"$ADDR" >/dev/null 2>&1; then
    echo -e "${RED}Brak dostępu do I2C bus $BUS. Uruchom z sudo:${RST}"
    echo -e "${RED}  sudo $0 $BUS $ADDR${RST}"
    exit 1
fi


scenes_to_read=()

if [[ "$MODE" == "all" ]]; then
    for (( m=0; m<15; m++ )); do
        (( MODE_DEFAULT[$m] >= 0 )) && scenes_to_read+=("${MODE_DEFAULT[$m]}")
        (( MODE_CUSTOM[$m] >= 0 )) && scenes_to_read+=("${MODE_CUSTOM[$m]}")
    done
elif [[ "$MODE" == "custom" ]]; then
    for (( m=0; m<15; m++ )); do
        (( MODE_CUSTOM[$m] >= 0 )) && scenes_to_read+=("${MODE_CUSTOM[$m]}")
    done
elif [[ "$MODE" == "default" ]]; then
    for (( m=0; m<15; m++ )); do
        (( MODE_DEFAULT[$m] >= 0 )) && scenes_to_read+=("${MODE_DEFAULT[$m]}")
    done
elif [[ "$MODE" =~ ^[0-9]+$ ]] && (( MODE >= 0 && MODE <= 255 )); then
    scenes_to_read+=("$MODE")
else
    echo -e "${RED}Nieznany tryb: $MODE${RST}"
    echo "  Użycie: sudo $0 [bus] [addr] [all|custom|default|scene_id|id:N]"
    exit 1
fi

for scene_id in "${scenes_to_read[@]}"; do
    read_and_print_full "$scene_id"
done
