#!/bin/bash
# HKC P275MV PLUS - simple VCP/i2c reader with decoding.
#
# Usage:
#   ./reader.sh c3
#   ./reader.sh f7 49 46
#   ./reader.sh bus:13 addr:0x37 c3 f7

BUS=14
ADDR=0x37
VCP_READ_DELAY=${VCP_READ_DELAY:-0.25}
VALUE_READ_DELAY=${VALUE_READ_DELAY:-0.20}
FAIL_RETRY_DELAY=${FAIL_RETRY_DELAY:-0.50}
FAIL_MAX_RETRIES=${FAIL_MAX_RETRIES:-3}
DEBUG_DDC=${DEBUG_DDC:-0}
DDC_DEST=0x6E
DDC_SRC=0x51
VCP_DDCCI_INIT=0x99

codes=()

for arg in "$@"; do
    case "$arg" in
        bus:*) BUS="${arg#bus:}" ;;
        addr:*) ADDR="${arg#addr:}" ;;
        0x*) codes+=("${arg#0x}") ;;
        *) codes+=("$arg") ;;
    esac
done

if (( ${#codes[@]} == 0 )); then
    echo "Usage: $0 [bus:N] [addr:0xNN] <vcp_hex> [vcp_hex...]"
    echo "Example: $0 c3"
    echo "Example: $0 f7 49 46"
    exit 1
fi

calc_cs() {
    local chk=$DDC_DEST
    for b in "$@"; do chk=$(( chk ^ b )); done
    printf "0x%02X" "$chk"
}

feature_name() {
    case "$1" in
        02) echo "New Control Value" ;;
        0b) echo "Color Temperature Increment" ;;
        0c) echo "Color Temperature Request" ;;
        0e) echo "Clock" ;;
        13) echo "Backlight Control" ;;
        20) echo "Horizontal Position" ;;
        04) echo "Restore Factory" ;;
        05) echo "Restore Brightness/Contrast" ;;
        08) echo "Restore Color" ;;
        10) echo "Brightness" ;;
        12) echo "Contrast" ;;
        14) echo "Color Temperature" ;;
        16) echo "Color Temp R User 1" ;;
        17) echo "Color Temp G User 1" ;;
        18) echo "Color Temp B User 1" ;;
        19) echo "Color Temp R User 2" ;;
        1a) echo "Color Temp G User 2" ;;
        1b) echo "Color Temp B User 2" ;;
        1c) echo "Color Temp R User 3" ;;
        1d) echo "Color Temp G User 3" ;;
        1e) echo "Color Temp B User 3" ;;
        22) echo "Profile selector" ;;
        26) echo "Gamma" ;;
        30) echo "Refresh Rate" ;;
        31) echo "Refresh Rate Position / Aspect Ratio" ;;
        32) echo "Crosshair Color" ;;
        33) echo "Stopwatch Time" ;;
        34) echo "Crosshair Shape" ;;
        35) echo "Stopwatch Position" ;;
        36) echo "Game Time Position" ;;
        37) echo "Game Time Position" ;;
        38) echo "Magnifier Position" ;;
        39) echo "Rear LED Lighting" ;;
        3a) echo "Full Game" ;;
        3b) echo "Alignment Aid" ;;
        3d) echo "Game Crosshair" ;;
        3e) echo "Stopwatch" ;;
        3f) echo "Game Time" ;;
        40) echo "Color Enhance" ;;
        41) echo "CR Enhance" ;;
        42) echo "Shadow Balance" ;;
        43) echo "Game Rush" ;;
        44) echo "Super Resolution" ;;
        45) echo "Night Vision" ;;
        46) echo "Halo Control" ;;
        47) echo "Local Dimming" ;;
        48) echo "DyDs" ;;
        49) echo "Dynamic OverDrive" ;;
        4a) echo "HDR" ;;
        4b) echo "Magnifier Mode" ;;
        4c) echo "Magnifier Size" ;;
        4d) echo "Magnification" ;;
        51) echo "Unknown 0x51" ;;
        52) echo "Active Control" ;;
        55) echo "Unknown 0x55" ;;
        59) echo "Saturation R" ;;
        5a) echo "Saturation Y" ;;
        5b) echo "Saturation G" ;;
        5c) echo "Saturation C" ;;
        5d) echo "Saturation B" ;;
        5e) echo "Saturation M" ;;
        60) echo "Output Range / Input Source" ;;
        61) echo "Quick Boot" ;;
        62) echo "Audio Volume (standard)" ;;
        63) echo "HawkEye Vision" ;;
        64) echo "HawkEye Size" ;;
        65) echo "HawkEye Position" ;;
        66) echo "HawkEye Level" ;;
        68) echo "Unknown 0x68" ;;
        69) echo "Unknown 0x69" ;;
        6c) echo "Video Black Level R" ;;
        6e) echo "Video Black Level G" ;;
        70) echo "Video Black Level B" ;;
        8d) echo "Audio Mute (standard)" ;;
        87) echo "Sharpness" ;;
        99) echo "Window Control" ;;
        9a) echo "Window Background" ;;
        9b) echo "Hue R" ;;
        9c) echo "Hue Y" ;;
        9d) echo "Hue G" ;;
        9e) echo "Hue C" ;;
        9f) echo "Hue B" ;;
        a0) echo "Hue M" ;;
        a8) echo "Unknown 0xA8" ;;
        ac) echo "Horizontal Frequency" ;;
        ae) echo "Vertical Frequency" ;;
        b2) echo "Sub-pixel Layout" ;;
        b4) echo "Source Timing Mode" ;;
        b6) echo "Display Technology" ;;
        c0) echo "OSD Show Time" ;;
        c1) echo "OSD H-Position" ;;
        c2) echo "OSD V-Position" ;;
        c3) echo "OSD Transparency" ;;
        c5) echo "Power LED" ;;
        c6) echo "Power Saving" ;;
        c7) echo "Reset Factory" ;;
        c8) echo "Display Controller" ;;
        c9) echo "Firmware Level" ;;
        ca) echo "OSD" ;;
        cc) echo "OSD Language" ;;
        d6) echo "Power Mode" ;;
        d8) echo "Low Blue Light" ;;
        dc) echo "Display Mode" ;;
        df) echo "VCP Version" ;;
        e0) echo "Manufacturer Specific 0xE0" ;;
        e1) echo "DCR" ;;
        e2) echo "Adaptive-Sync" ;;
        f0) echo "Manufacturer Specific 0xF0" ;;
        f3) echo "Usage Time / Manufacturer 0xF3" ;;
        fa) echo "Manufacturer Specific 0xFA" ;;
        fd) echo "Manufacturer Specific 0xFD" ;;
        fe) echo "Firmware / Manufacturer 0xFE" ;;
        ff) echo "Manufacturer Specific 0xFF" ;;
        f6) echo "Audio Volume" ;;
        f7) echo "Audio Mute" ;;
        *) echo "Unknown 0x$1" ;;
    esac
}

decode_value() {
    local code=$1 value=$2
    case "$code" in
        0e|10|12|16|17|18|19|1a|1b|1c|1d|1e|20|42|46|59|5a|5b|5c|5d|5e|62|6c|6e|70|9a|9b|9c|9d|9e|9f|a0|c1|c2|c3|f6)
            printf "%d" "$value"
            ;;
        c0)
            printf "%d sec" "$value"
            ;;
        87|41|44)
            (( value == 0 )) && echo "Off" || printf "Level %d" "$value"
            ;;
        40)
            (( value == 0 )) && echo "Off" || printf "Level %d" "$value"
            ;;
        d8)
            printf "Level %d" "$value"
            ;;
        39|3b|3d|3e|3f|4b|63|e1|e2)
            case "$value" in
                0) echo "Off" ;;
                1) echo "On" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        8d|f7|43|61)
            case "$value" in
                0|1) echo "Off" ;;
                2) echo "On" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        ca)
            case "$value" in
                1) echo "Off" ;;
                2) echo "On" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        cc)
            case "$value" in
                1) echo "Chinese Traditional" ;;
                2) echo "English" ;;
                3) echo "French" ;;
                4) echo "German" ;;
                5) echo "Italian" ;;
                6) echo "Japanese" ;;
                7) echo "Korean" ;;
                8) echo "Portuguese" ;;
                9) echo "Russian" ;;
                10) echo "Spanish" ;;
                12) echo "Turkish" ;;
                13) echo "Chinese Simplified" ;;
                14) echo "Portuguese Brazil" ;;
                15) echo "Arabic" ;;
                20) echo "Dutch" ;;
                22) echo "Finnish" ;;
                23) echo "Greek" ;;
                25) echo "Hindi" ;;
                30) echo "Polish" ;;
                35) echo "Thai" ;;
                36) echo "Ukrainian" ;;
                37) echo "Vietnamese" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        d6)
            case "$value" in
                1) echo "On" ;;
                4) echo "Standby" ;;
                5) echo "Turn Off Display" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        dc)
            case "$value" in
                0) echo "Standard default" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        df)
            printf "MCCS %d.%d" "$((value >> 8))" "$((value & 0xFF))"
            ;;
        b2)
            case "$value" in
                1) echo "RGB vertical stripe" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        b6)
            case "$value" in
                3) echo "LCD active matrix" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        14)
            case "$value" in
                5) echo "Warm" ;;
                6) echo "Natural" ;;
                8) echo "Cool" ;;
                11) echo "User 1" ;;
                12) echo "User 2" ;;
                13) echo "User 3" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        22)
            case "$value" in
                1) echo "Standard default" ;;
                2) echo "Standard custom" ;;
                3) echo "RTS/RPG default" ;;
                4) echo "RTS/RPG custom" ;;
                5) echo "FPS default" ;;
                6) echo "FPS custom" ;;
                7) echo "MOBA default" ;;
                8) echo "MOBA custom" ;;
                9) echo "Movie default" ;;
                10) echo "Movie custom" ;;
                11) echo "Reading default" ;;
                12) echo "Reading custom" ;;
                13) echo "Night default" ;;
                14) echo "Night custom" ;;
                15) echo "Eye Care default" ;;
                16) echo "Eye Care custom" ;;
                17) echo "MacView default" ;;
                18) echo "MacView custom" ;;
                19) echo "E-book default" ;;
                20) echo "E-book custom" ;;
                21) echo "sRGB default" ;;
                22) echo "sRGB custom" ;;
                23) echo "AdobeRGB default" ;;
                24) echo "AdobeRGB custom" ;;
                25) echo "DCI-P3 default" ;;
                26) echo "DCI-P3 custom" ;;
                27) echo "DyDs/ULL FPS default" ;;
                28) echo "DyDs/ULL FPS custom" ;;
                29) echo "DyDs/LD default" ;;
                30) echo "DyDs/LD custom" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        26)
            case "$value" in
                2) echo "1.8" ;;
                4) echo "2.0" ;;
                6) echo "2.2" ;;
                8) echo "2.4" ;;
                10) echo "2.6" ;;
                12) echo "S Curve" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        45)
            case "$value" in
                0) echo "Off" ;;
                1) echo "Level 1" ;;
                2) echo "Level 2" ;;
                3) echo "Auto-Level 1" ;;
                4) echo "Auto-Level 2" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        49)
            case "$value" in
                0) echo "Off" ;;
                1) echo "Level 1" ;;
                2) echo "Level 2" ;;
                3) echo "Level 3" ;;
                4) echo "Topspeed" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        47)
            case "$value" in
                0|1|2) echo "Off" ;;
                3) echo "Low" ;;
                4) echo "Smooth" ;;
                5) echo "Medium" ;;
                6) echo "High" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        48)
            case "$value" in
                0|1|2) echo "Off" ;;
                3) echo "Low" ;;
                4) echo "Medium" ;;
                5) echo "High" ;;
                6) echo "ULL Level 1" ;;
                7) echo "ULL Level 2" ;;
                8) echo "ULL Level 3" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        4a)
            case "$value" in
                0) echo "Off" ;;
                1) echo "Auto" ;;
                2) echo "Game" ;;
                3) echo "Movie" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        60)
            case "$value" in
                1) echo "RGB Auto" ;;
                2) echo "RGB Limit" ;;
                3) echo "RGB Full" ;;
                15) echo "USB-C" ;;
                16) echo "DisplayPort" ;;
                17) echo "HDMI-1" ;;
                18) echo "HDMI-2" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        c5)
            case "$value" in
                0|1) echo "Off" ;;
                2) echo "Level 1" ;;
                3) echo "Level 2" ;;
                4) echo "Level 3" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        c6)
            case "$value" in
                0) echo "Off" ;;
                1) echo "Level 1" ;;
                2) echo "Level 2" ;;
                3) echo "Level 3" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        31|35|37)
            case "$value" in
                0) echo "Top Right" ;;
                1) echo "Top Left" ;;
                2) echo "Bottom Right" ;;
                3) echo "Bottom Left" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        38)
            case "$value" in
                1) echo "Top Right" ;;
                2) echo "Top Left" ;;
                3) echo "Bottom Right" ;;
                4) echo "Bottom Left" ;;
                5) echo "Central" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        65)
            case "$value" in
                0) echo "Top Right" ;;
                1) echo "Top Left" ;;
                2) echo "Central" ;;
                3) echo "Bottom Right" ;;
                4) echo "Bottom Left" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        32)
            case "$value" in
                0) echo "Red" ;;
                1) echo "Yellow" ;;
                2) echo "Green" ;;
                3) echo "Cyan" ;;
                4) echo "Blue" ;;
                5) echo "Purple" ;;
                6) echo "White" ;;
                7) echo "Auto" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        33|36)
            case "$value" in
                1) echo "15 min" ;;
                2) echo "30 min" ;;
                3) echo "45 min" ;;
                4) echo "60 min" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        4c)
            case "$value" in
                1) echo "Small" ;;
                2) echo "Medium" ;;
                3) echo "Large" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        64)
            case "$value" in
                0) echo "Small" ;;
                1) echo "Medium" ;;
                2) echo "Large" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        4d)
            case "$value" in
                0) echo "x1.5" ;;
                1) echo "x2" ;;
                2) echo "x4" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        66)
            case "$value" in
                0) echo "Level 1" ;;
                1) echo "Level 2" ;;
                2) echo "Level 3" ;;
                3) echo "Level 4" ;;
                4) echo "Level 5" ;;
                *) printf "%d" "$value" ;;
            esac
            ;;
        *)
            printf "%d" "$value"
            ;;
    esac
}

read_vcp() {
    local code=$1 attempt status

    for attempt in 1 2 3; do
        sleep 0.04
        if read_vcp_once "$code"; then
            return 0
        else
            status=$?
            # An explicit unsupported reply is not a transport failure.
            (( status == 2 )) && return 2
        fi
        (( DEBUG_DDC != 0 )) && printf "  attempt %d failed: %s\n" "$attempt" "$REPLY_ERROR" >&2
        (( attempt < 3 )) && sleep 0.10
    done

    return 1
}

read_vcp_once() {
    local code=$1 code_hex cs raw
    code_hex=$(printf "0x%02X" "$code")
    cs=$(calc_cs "$DDC_SRC" 0x82 0x01 "$code")

    REPLY_ERROR=""
    i2ctransfer -y "$BUS" w5@"$ADDR" "$DDC_SRC" 0x82 0x01 "$code_hex" "$cs" 2>/dev/null || {
        REPLY_ERROR="write request"
        return 1
    }
    sleep "$VCP_READ_DELAY"
    raw=$(i2ctransfer -y "$BUS" r11@"$ADDR" 2>/dev/null)
    if [[ -z "$raw" ]]; then
        REPLY_ERROR="empty response"
        return 1
    fi

    local -a b
    read -r -a b <<< "$raw"
    REPLY_RAW="$raw"
    if (( ${#b[@]} >= 3 && ${b[0]} == DDC_DEST && ${b[1]} == 0x80 )); then
        if (( (0x50 ^ ${b[0]} ^ ${b[1]} ^ ${b[2]}) == 0 )); then
            REPLY_ERROR="NULL response (no setting value)"
            return 1
        fi
    fi
    if (( ${#b[@]} < 11 )); then
        REPLY_ERROR="short response"
        return 1
    fi
    if (( ${b[0]} != DDC_DEST )); then
        REPLY_ERROR="bad destination"
        return 1
    fi
    if (( ${b[1]} != 0x88 )); then
        REPLY_ERROR="unexpected response length"
        return 1
    fi
    local checksum=0x50 byte
    for byte in "${b[@]:0:11}"; do checksum=$(( checksum ^ byte )); done
    if (( checksum != 0 )); then
        REPLY_ERROR="bad checksum"
        return 1
    fi
    if (( ${b[2]} != 0x02 )); then
        REPLY_ERROR=$(printf "bad reply type 0x%02X" "$(( ${b[2]} ))")
        return 1
    fi
    if (( ${b[4]} != (code & 0xFF) )); then
        REPLY_ERROR=$(printf "wrong code 0x%02X" "$(( ${b[4]} ))")
        return 1
    fi

    REPLY_STATUS=$(( ${b[3]} ))
    REPLY_MAX=$(( ${b[6]} * 256 + ${b[7]} ))
    REPLY_CUR=$(( ${b[8]} * 256 + ${b[9]} ))
    REPLY_SOURCE="current"
    REPLY_RAW="$raw"

    if (( REPLY_STATUS != 0 )); then
        REPLY_ERROR=$(printf "unsupported status 0x%02X" "$REPLY_STATUS")
        return 2
    fi

    # Apply the vendor interpretation only to a successful, validated reply.
    if (( code == 0x46 && ${b[6]} == 0xFF && ${b[7]} <= 100 )); then
        REPLY_CUR=$(( ${b[7]} ))
        REPLY_SOURCE="vendor-max-low"
    fi

    return 0
}

wake_ddc() {
    local saved_delay=$VCP_READ_DELAY
    VCP_READ_DELAY=0.25
    read_vcp_once "$VCP_DDCCI_INIT" >/dev/null 2>&1 || true
    VCP_READ_DELAY=$saved_delay
    sleep 0.25
}

if ! i2ctransfer -y "$BUS" r1@"$ADDR" >/dev/null 2>&1; then
    echo "No I2C access: bus=$BUS addr=$ADDR"
    exit 1
fi

wake_ddc

for index in "${!codes[@]}"; do
    code_arg=${codes[$index]}
    (( index > 0 )) && sleep "$VALUE_READ_DELAY"

    if [[ ! "$code_arg" =~ ^[0-9a-fA-F]{1,2}$ ]]; then
        echo "Invalid VCP code: $code_arg"
        continue
    fi

    code_lc=$(printf "%02x" "$((16#$code_arg))")
    code_dec=$((16#$code_lc))
    name=$(feature_name "$code_lc")

    REPLY_RAW=""
    fail_retry=0
    read_ok=0
    while true; do
        if read_vcp "$code_dec"; then
            read_ok=1
            break
        else
            status=$?
        fi
        (( fail_retry++ ))
        if (( status == 2 )); then
            printf "%s: UNSUPPORTED (0x%s)\n" "$name" "$code_lc"
            break
        fi
        if (( FAIL_MAX_RETRIES > 0 && fail_retry >= FAIL_MAX_RETRIES )); then
            printf "%s: FAIL (%s)\n" "$name" "$REPLY_ERROR" >&2
            break
        fi
        (( DEBUG_DDC != 0 )) && printf "%s: FAIL retry %d: %s\n" "$name" "$fail_retry" "$REPLY_ERROR" >&2
        sleep "$FAIL_RETRY_DELAY"
    done

    (( read_ok == 0 )) && continue

    if [[ -n "${REPLY_RAW:-}" ]]; then
        decoded=$(decode_value "$code_lc" "$REPLY_CUR")
        if [[ "$code_lc" == "22" ]]; then
            # WMW maps write values to profile names. This interpretation is
            # not independent confirmation of the profile selected in OSD.
            printf "%s: value=%d (0x%04X), WMW=%s\n" \
                "$name" "$REPLY_CUR" "$REPLY_CUR" "$decoded"
        else
            printf "%s: %s\n" "$name" "$decoded"
        fi
        if (( DEBUG_DDC != 0 )); then
            printf "  raw=%d hex=0x%04X max=0x%04X status=0x%02X source=%s\n" \
                "$REPLY_CUR" "$REPLY_CUR" "$REPLY_MAX" "$REPLY_STATUS" "$REPLY_SOURCE"
        fi
        (( DEBUG_DDC != 0 )) && printf "  frame: %s\n" "$REPLY_RAW"
    fi
done

exit 0
