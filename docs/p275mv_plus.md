# Titan Army P275MV PLUS: DDC/CI reference

Everything known about controlling the Titan Army P275MV PLUS over DDC/CI: transport, the manufacturer's register
scheme, every VCP code with its values, the picture profile table, and what cannot be controlled from a computer.
It was worked out on Linux (amdgpu, DisplayPort) with captures and static analysis of the manufacturer's Windows tool
(VIEW MORE WIDGET, "VMW"), then confirmed code by code on the monitor.

Conventions: VCP codes are hex (`4A`), values are decimal unless written as `0x…`. A reply is shown as
`maximum/current` in hex (`ff04/0002`). "Confirmed" = seen in the manufacturer's traffic or checked on the monitor;
anything else is marked as unconfirmed.

Monitor: P275MV PLUS (Enhanced), Mstar controller, USB firmware 5.1.1, M001 2025-09-20 (see
[firmware](../firmware/README.md)).

> [!WARNING]
> - **Never write `22 = 0`** (picture profile 0).
> - **Never write `F0`**: values 1 and 2 turn the monitor off.
> - `D6 = 5` turns the monitor off until the power button is pressed; `D6 = 3` turns the USB hub off.
> - `CA = 1` locks front buttons 2 and 3 (unlock with `CA = 2`).
> - Low black levels (`6C`/`6E`/`70`, e.g. `6E = 1`) tint the picture until set back to 50.

**Contents:** [Transport](#transport) · [Registers](#registers-plain-and-manufacturer) ·
[Change notifications](#change-notifications) · [Picture profiles](#picture-profiles) ·
[Profile table](#profile-table) · [Settings](#settings) · [Rules between settings](#rules-between-settings) ·
[Read-only values](#read-only-values) · [Not available over DDC](#not-available-over-ddc) ·
[Code index](#code-index-00ff) · [Manufacturer software](#manufacturer-software-vmw) ·
[Hardware notes](#hardware-notes) · [Tools](#tools)

## Transport

- **Bus.** The monitor answers at I2C address `0x37`. On amdgpu with DisplayPort the working bus is the DP AUX
  channel (`AMDGPU DM aux hw bus N`, I2C over AUX); here `/dev/i2c-14` for connector DP-3. Find yours with
  `ddcutil detect`. The user needs access to `/dev/i2c-*` (group `i2c`, module `i2c-dev`).
- **GET:** write `51 82 01 <code> <chk>`, wait 40 ms, read 11 bytes. Reply:
  `6e 88 02 <status> <code> <type> <mh> <ml> <sh> <sl> <chk>`; `maximum = mh<<8 | ml`, `current = sh<<8 | sl`.
  The request checksum is the XOR of `0x6E` and all bytes; the reply checksum XORs to 0 together with `0x50`.
  Status 0 = OK, 1 = unsupported code.
- **SET:** write `51 84 03 <code> <hi> <lo> <chk>`. There is no reply.
- **Busy and garbage.** `6e 80 be` is a NULL reply (monitor busy). Anything with a wrong header, code or checksum is
  garbage: often a repeating buffer (`2f 9a 2d 3a b5 ff 9d 37 b3 63 23`) or the tail of an older reply. Never
  interpret either; retry after a pause.
- **Periodic outages.** The monitor stops answering for about 0.5–1 s, in pairs roughly every 7.7 s (about 14 % of
  the time). It is not caused by other bus users (stopping powerdevil and ddccontrol did not help), and Windows sees
  the same errors (25 of 382 reads in a capture, `0xC0262589`) and retries.
- **Timing.**
  - Keep **at least 100 ms between the starts of two requests**. At 65 ms only about 60 % of table reads succeeded;
    at 100 ms, 90–95 %.
  - The write→read delay is uncritical between 30 and 80 ms (120 ms is worse).
  - After a failed reply wait ~324 ms (as VMW does).
- **Settling.** After an outage or an interrupted table stream, send plain `GET DF` until two valid replies come in
  a row. `DF` is used because a code `00–1E` right after `GET 99 → GET 22` would start a table stream.
- **Writes.**
  - A written value can be read back at once (brightness: ~145 ms including the read; `C1` via `99`: ~245 ms).
  - About 1 in 6 single writes was lost in a test, so read back and resend once when the value does not match.

## Registers: plain and manufacturer

The monitor has two register sets under the same codes.

- **Plain register:** a normal `GET <code>`. Standard MCCS codes and some manufacturer codes live here. This is
  what ddcutil reads.
- **Manufacturer register `[99]`:** read with `GET 99` (reply `fafa/fafa`) immediately followed by `GET <code>`.
  Most manufacturer settings answer only this way; a plain read gives "unsupported" (ddcutil: `ERR`). The reply is
  `maximum = 0xFF<max>`, `current = value`, e.g. `4A = ff04/0002` (HDR, max 4, value 2). **`00ff/0000` is an empty
  placeholder**, not data.
- **Writes** follow the manufacturer: `SET 99 = v`, then `SET <code> = v` with the same value (VMW sends the pair
  once, ~60 ms apart, without retries). The meaning of `99` itself in the firmware is unknown; a lone `SET 99` is
  harmless because it precedes every manufacturer write. Settings that live in a plain register are written without
  `99`.
- **Plain-only codes** (always written without `99`):
  - `39` LED, `CA` button lock, `D6` power, `E2` Adaptive-Sync, `02` notifications.
  - `62` volume, `68` OSD language, `8D` mute, `DC` display application.
  - MCCS resets `04 05 06 08 0A B0`.
- **Profile selectors `00–1E`.** After `GET 99`, these codes start a profile table stream (see
  [Profile table](#profile-table)). Read them plain; write them with `99` like VMW. Trap: `GET 99 → GET 22 →
  GET <00–1E>` is a table read even when only a plain value was wanted (contrast `12` returned `5a32/0001`, a table
  fragment). Close the sequence after reading `22` with another request (the tools use `DF`).

**Codes with two different registers** (full scan with both paths):

| Code | `[99]` | Plain |
| --- | --- | --- |
| `22` | picture profile (max `0x26`) | `ffff/0001`, meaning unknown |
| `24` | aspect ratio | other encoding (`[99]` 1 ↔ plain 4, 5 ↔ 5) |
| `26` | gamma (2–12) | gamma in another encoding (4 at 2.2) |
| `30` | refresh rate counter 0/1 | Low Blue Light 0–100 (steps of 25) |
| `31` | refresh rate counter position | `ffff/0010` |
| `36` | game time 1–4 | other encoding |
| `3E` | stopwatch on/off | `0064/0032` |
| `51` | PIP sub source | unknown |
| `52` | audio source | MCCS Active control, see [notifications](#change-notifications) |
| `60` | Output Range 0–2 | MCCS Input Source (`0x0F` DP-1, `0x10` DP-2, `0x11` HDMI-1, `0x12` HDMI-2) |
| `C0` | OSD show time 5–60 s | usage hours (≈ `F3`/60) |
| `C6` | power saving 0–3 | MCCS application enable key (`45cc`) |
| `CC` | OSD language (max 24) | OSD language (MCCS, max 255) |
| `E1` | DCR 0/1 | Low Blue Light 0–4 |
| `E2` | placeholder | Adaptive-Sync 0/1 (the working register) |
| `F7` | audio mute 0/1 | first 4 characters of the serial number (ASCII) |

## Change notifications

- **`02` New control value** (plain): 1 = nothing new, **2 = a setting was changed in the OSD**. Writing `02 = 1`
  (plain) clears the flag and `52`.
- **`52` Active control** (plain, low byte): the code of the last changed setting, e.g. `10` (brightness changed by
  the light sensor), `60` (input), `14` (color temperature), `04` (after a factory reset). It is not a queue: it keeps
  the last code until `02 = 1`. Manufacturer settings (e.g. `D4`) may not show up here.
- Titan Control polls `02` every 3 seconds and re-reads settings when it is 2. Settings it wrote itself in the last
  5 s are not re-read, because some registers lag behind or (Halo) never show the written value.

## Picture profiles

**`22`** selects the picture profile; Default and Custom variants have separate codes. Read via `[99]`; the reported
maximum is 38 (`0x26`), which does not mean all values up to 38 exist.

| Profile | Default | Custom | | Profile | Default | Custom |
| --- | --- | --- | --- | --- | --- | --- |
| Standard | `01` | `02` | | E-book | `13` | `14` |
| RTS | `03` | `04` | | sRGB | `15` | `16` |
| FPS | `05` | `06` | | AdobeRGB | `17` | `18` |
| MOBA | `07` | `08` | | DCI-P3 | `19` | `1A` |
| Movie | `09` | `0A` | | DyDs/ULL FPS | `1B` | `1C` |
| Reading | `0B` | `0C` | | DyDs/LD | `1D` | `1E` |
| Night | `0D` | `0E` | | | | |
| Eye Care | `0F` | `10` | | | | |
| MacView | `11` | `12` | | | | |

- Custom = even code, Default = odd code (confirmed with Frida captures, 2026-09-29).
- The picture settings of a profile (brightness, contrast, colors, gamma, HDR, …) can only be changed in its Custom
  variant; a Default variant shows the factory values.
- `E0` (plain) is a separate "picture profile change" code with values 0–14 (15 profiles, confirmed). It is not used.
- On the old firmware DyDs profiles need Adaptive-Sync off first (the monitor refuses otherwise). Firmware
  2025-09-20 allows DyDs together with Adaptive-Sync.
- VMW 1.0.2.8 groups the two DyDs profiles in a "DyDs Pulse" menu. On this monitor it holds just DyDs/ULL FPS and
  DyDs/LD; the new "DyDs/ULL Pulse" and "DyDs/LD Pulse" modes from the VMW release notes do not exist in this firmware.

## Profile table

All picture settings of a Custom profile can be read in one go as the profile table. This is how VMW reads them,
and how Titan Control reads them on start and after a profile change.

**Sequence:** `GET 99` (= `fafa/fafa`) → `GET 22` (max `0x26`, current = active profile 1–30) → `GET <profile code>`
→ `GET FE` × 9 → `GET FE` = `0000/5101` (end). Each reply carries 4 data bytes `mh ml sh sl`; the 10 fragments are
joined from index 2. Example (RTS Custom): first fragment `5a32/0103` = `[90, 50, 1, 3]` = brightness 90,
contrast 50, sharpness 1, Color Enhance 3.

**Reliable reading (100 % within the budget in tests).** The sequence is stateful, so any error restarts it:

1. Settle (`GET DF` until two valid replies in a row), then `GET 99` until `fafa/fafa` (up to 8 tries).
2. `GET 22`; it must report max `0x26` and a profile 1–30. Take the profile number from this read only: an earlier,
   unvalidated read of `22` once returned another code's reply (brightness `0x64`) as the "profile".
3. `GET <profile>`, then `GET FE` until `0000/5101`:
   - NULL reply: the fragment was not consumed, so repeat the same request.
   - Garbage: the fragment was consumed, so `GET FF` sends it again.
   - `FF` outside the stream answers `0000/0200`; if 10 fragments are already in, the lost reply was the
     `0000/5101` terminator.
4. Require exactly 10 fragments. Accept the table only after **two identical reads in a row** (budget 60 s). Never
   combine fragments from different attempts.

An interrupted stream leaves the monitor answering NULL or stale replies until it is settled again. Results: 20/20
confirmed tables with the Python reader, 10/10 in Rust (4–13 s each); changing brightness in the OSD between reads
changes the first byte as expected.

**Custom profiles only.** VMW reads only Custom tables. For a Default profile (e.g. Standard Default `01`) the stream
ends after 9 fragments instead of 10 (25 tries); the Default layout is unknown. For Default profiles Titan Control
reads the settings one by one. All of them answer except the User 1–3 RGB gains (`17 19 1B 1C 1D` = unsupported,
`1E` plain = max 2), which come only from a table.

| Index | Setting | Values |
| --- | --- | --- |
| 0–1 | — | empty |
| 2 | Brightness | 0–100 |
| 3 | Contrast | 0–100 |
| 4 | Sharpness | 0–5 |
| 5 | Color Enhance | 0–10 |
| 6 | CR Enhance | 0–5 |
| 7 | Shadow Balance | 0–100 |
| 8 | Color temperature | 5 Warm, 6 Natural, 8 Cool, 11 User 1, 12 User 2, 13 User 3 |
| 9–11 | User 1 red, green, blue | 0–100 |
| 12–14 | User 2 red, green, blue | 0–100 |
| 15–17 | User 3 red, green, blue | 0–100 |
| 18–23 | Hue red, green, blue, yellow, cyan, magenta | 0–100 |
| 24–29 | Saturation red, green, blue, yellow, cyan, magenta | 0–100 |
| 30 | Low Blue Light | 0–100 in steps of 25 (= level 0–4 of `D8`) |
| 31 | HDR | 0 Off, 1 Auto, 2 Game, 3 Movie |
| 32 | — | empty |
| 33 | Gamma | 2 = 1.8, 4 = 2.0, 6 = 2.2, 8 = 2.4, 10 = 2.6, 12 = S-Curve |
| 34 | Super Resolution | 0–5 |
| 35 | Night Vision | 0 Off, 1 Lvl 1, 2 Lvl 2, 3 Auto Lvl 1, 4 Auto Lvl 2 |
| 36 | Dynamic OD | 0 Off, 1–3 Lvl 1–3, 4 Top Speed |

## Settings

Register: `[99]` = manufacturer register (read after `GET 99`, written as `99` + code), plain = normal register,
table = profile selector (read plain, written with `99`). "Default" is the factory value.

### Picture mode settings (saved in the Custom profile)

The OSD (Profile → Custom) keeps these per profile: brightness, contrast, saturation, hue, color temperature,
gamma, sharpness, Low Blue Light, Color Enhance, CR Enhance, Shadow Balance, Night Vision, Super Resolution, Dynamic
OD, Local Dimming, Halo Control and DyDs (confirmed by the OSD, 2026-10-02). All of them except the last three are in
the readable profile table; Local Dimming, Halo Control and DyDs are read and written one by one (see the next
section). HDR is in the table but not in the OSD's Custom list.

| Code | Register | Setting | Values | Default |
| --- | --- | --- | --- | --- |
| `10` | table | Brightness | 0–100 | 90 |
| `12` | table | Contrast | 0–100 | 50 |
| `87` | `[99]` | Sharpness | 0–5 | 0 |
| `D8` | `[99]` | Low Blue Light | 0–4 (plain `E1` max 4 and plain `30` 0–100 mirror it) | 0 |
| `14` | table | Color temperature | 5 Warm, 6 Natural, 8 Cool, 11–13 User 1–3 | Warm |
| `16 17 18` | table | User 1 red, green, blue | 0–100 | 50 |
| `19 1A 1B` | table | User 2 red, green, blue | 0–100 | 50 |
| `1C 1D 1E` | table | User 3 red, green, blue | 0–100 | 50 |
| `26` | `[99]` | Gamma | 2 = 1.8, 4 = 2.0, 6 = 2.2, 8 = 2.4, 10 = 2.6, 12 = S-Curve | 2.2 |
| `4A` | `[99]` | HDR | 0 Off, 1 Auto, 2 Game, 3 Movie | Off |
| `40` | `[99]` | Color Enhance | 0–10 | 0 |
| `41` | `[99]` | CR Enhance | 0–5 | 0 |
| `42` | `[99]` | Shadow Balance | 0–100 | 50 |
| `44` | `[99]` | Super Resolution | 0–5 | 0 |
| `45` | `[99]` | Night Vision | 0 Off, 1 Lvl 1, 2 Lvl 2, 3 Auto Lvl 1, 4 Auto Lvl 2 | Off |
| `49` | `[99]` | Dynamic OD | 0 Off, 1–3 Lvl 1–3, 4 Top Speed | Off |
| `9B 9D 9F` | `[99]` | Hue red, green, blue | 0–100 | 50 |
| `9C 9E A0` | `[99]` | Hue yellow, cyan, magenta | 0–100 | 50 |
| `59 5B 5D` | `[99]` | Saturation red, green, blue | 0–100 | 50 |
| `5A 5C 5E` | `[99]` | Saturation yellow, cyan, magenta | 0–100 | 50 |

Notes:

- **RGB gains.** Only `16`, `18` and `1A` can be read one by one; the other gains come only from the profile table.
  A gain write was accepted only while that User temperature was active (seen on `16`).
- **Color temperature.** ddcutil translates `14` with the MCCS names: 5 ≈ 6500 K, 6 ≈ 7500 K, 8 ≈ 9300 K
  (not confirmed by the manufacturer). Writing an invalid value (`14 = 1`, `26 = 1`) switched to another valid value
  instead of keeping the old one.
- **Brightness** follows the light sensor while the sensor is on (e.g. 10 in the dark, 51 under a flashlight).

### Backlight, ratio and other settings (not in the profile table)

| Code | Register | Setting | Values | Default |
| --- | --- | --- | --- | --- |
| `E1` | `[99]` | DCR (dynamic contrast) | 0/1 | Off |
| `47` | `[99]` | Local Dimming (in the OSD under Profile → Custom) | **2 Off, 3 Low, 4 Smooth, 5 Medium, 6 High** (only 2–6 accepted) | High |
| `46` | `[99]` | Halo Control (OSD: Profile → Custom) | 0–100 | 25 |
| `48` | `[99]` | DyDs (OSD: Profile → Custom) | 2 Off, 3 Low, 4 Medium, 5 High, 6 ULL 1, 7 ULL 2, 8 ULL 3 | Off |
| `43` | `[99]` | Game Rush | 0/1 | Off |
| `24` | `[99]` | Aspect ratio | **1 Wide, 2 4:3, 3 1:1, 4 21:9, 5 Auto**; 0 is refused | Auto |
| `DC` | plain | Display Application (MCCS) | 0 Standard, 3 Movie, 5 Games | 0 |

Notes:

- **Halo Control is write-only.** Writes with `99` work (70 and 10 showed in the OSD), but `46 [99]` always reads
  `ff64/0001` and the plain read is unsupported. Titan Control does not read it and shows the last value it wrote.
- **Aspect ratio.** 1, 2, 3 and 5 were checked in the OSD (2026-10-02). 21:9 = 4 is the remaining value. In the OSD
  21:9 is locked until a wide Full Game screen mode (such as 3K Wide) is set; over DDC only Wide and 25" are accepted,
  so Titan Control shows only the 21:9 option greyed out.
- **Display Application** accepts only 0, 3 and 5. Writing 3 or 5 changes contrast and Local Dimming. A plain read of
  `00ff/0000` means 0 here, not a placeholder. It is in neither the OSD nor VMW, so Titan Control does not offer it.

### Game Aid

| Code | Register | Setting | Values |
| --- | --- | --- | --- |
| `E2` | plain | Adaptive-Sync | 0/1 (the `[99]` register is a placeholder) |
| `E4` | `[99]` | Adaptive-Sync, second switch | 0/1 (also works) |
| `3A` | `[99]` | Full Game (screen size) | **0 Wide, 1 25"**; 2–13 ignored |
| `30` | `[99]` | Refresh rate counter | 0/1 (reported max 12 is wrong) |
| `31` | `[99]` | Refresh rate counter position | 0 top right, 1 top left, 2 bottom right, 3 bottom left |
| `3D` | `[99]` | Crosshair | 0/1 |
| `34` | `[99]` | Crosshair shape | 1–6 |
| `32` | `[99]` | Crosshair color | 0 red, 1 yellow, 2 green, 3 cyan, 4 blue, 5 purple, 6 white, 7 auto |
| `3E` | `[99]` | Stopwatch | 0/1 |
| `33` | `[99]` | Stopwatch time | 1–4 = 15, 30, 45, 60 min |
| `35` | `[99]` | Stopwatch position | as `31` |
| `3F` | `[99]` | Game time | 0/1 |
| `36` | `[99]` | Game time length | 1–4 = 15, 30, 45, 60 min |
| `37` | `[99]` | Game time position | as `31` |
| `4B` | `[99]` | Magnifier | 0/1 |
| `4D` | `[99]` | Magnification | 0 ×1.5, 1 ×2, 2 ×4 |
| `4C` | `[99]` | Magnifier size | 1 small, 2 medium, 3 large |
| `38` | `[99]` | Magnifier position | 1 top right, 2 top left, 3 bottom right, 4 bottom left, 5 center |
| `3B` | `[99]` | Alignment aid | 0/1 |
| `63` | `[99]` | HawkEye Vision | 0/1 |
| `64` | `[99]` | HawkEye size | 0 small, 1 medium, 2 large |
| `65` | `[99]` | HawkEye position | 0 top right, 1 top left, 2 center, 3 bottom right, 4 bottom left |
| `66` | `[99]` | HawkEye level | 0–4 = level 1–5 |

VMW also sent `3A = 4` ("sPX"); the monitor does not accept it over DDC.

### Inputs, audio, PIP/PBP, USB

| Code | Register | Setting | Values |
| --- | --- | --- | --- |
| `57` | `[99]` | Input | low byte: 0 auto, 1/2 DP, 3 USB-C, 5 HDMI 1, 6 HDMI 2 (reply `ff05/69xx`) |
| `60` | `[99]` | Output Range | 0 auto, 1 limited, 2 full; set 0 (auto) before another range |
| `5F` | `[99]` | USB hub upstream | 0 USB-C, 1 USB-B |
| `D5` | `[99]` | USB power in sleep | 0/1 |
| `F6` | `[99]` | Volume | 0–100 (plain `62` is the same volume); default 50 |
| `F7` | `[99]` | Mute | 0 sound, 1 muted (plain `8D` mirrors it inverted: 0 muted, 1 sound) |
| `52` | `[99]` | Audio source | low byte: 0 auto, 3 USB-C, 4 HDMI 1, 5 HDMI 2 (only connected inputs) |
| `50` | `[99]` | PIP/PBP | 0 off, 1 PIP, 2 PBP 1:1, 3 PBP 2:1, 4 PBP 1:2 |
| `51` | `[99]` | PIP sub source | low byte: 0 DP, 3 USB-C, 4 HDMI 1, 5 HDMI 2 |
| `53` | `[99]` | PIP position | 0 top right, 1 top left, 2 bottom right, 3 bottom left (PIP only) |
| `54` | `[99]` | PIP size | 0 small, 1 medium, 2 large (PIP only) |
| `55` | `[99]` | Swap pictures | write 1 |
| `56` | `[99]` | Reset PIP settings | write 1 |

`51`, `52` and `57` answer `ff05/69xx`: the low byte is the selected source. The high byte `0x69` = `0110 1001` may
be a mask of available options (bits 0, 3, 5, 6 = auto, USB-C, HDMI 1, HDMI 2 for `57`, with DP unplugged); this is
unconfirmed. Writes to `51`/`52` do nothing while PIP is off.

### Lighting

| Code | Register | Setting | Values |
| --- | --- | --- | --- |
| `39` | plain | LED lighting | **0 on, 1 off**; ≥ 2 ignored. Only a plain SET works |
| `E7` | `[99]` | LED mode | 1 Normal, 2 Breathe, 3 Flicker, 4 Plain Water, 5 Star, 6 Colorful Pearls, 7 Colorful Water |
| `E5` | `[99]` | LED color | 1 red, 2 green, 3 blue, 4 yellow, 5 purple, 6 cyan, 7 colorful (only in Breathe and Plain Water) |
| `E6` | `[99]` | LED strength | 1 highest, 2 standard, 3 soft |
| `E8` | `[99]` | Front color | 1–6 as `E5`, only in Colorful Water |
| `E9` | `[99]` | Rear color | 1–6 as `E5`, only in Colorful Water |
| `C5` | `[99]` | Power LED | 1 off, 2 level 1, 3 level 2, 4 level 3 (default level 2) |

In mode 5 (Star) only the color can be changed; mode 6 (Colorful Pearls) locks the other LED settings.

### OSD and system

| Code | Register | Setting | Values | Default |
| --- | --- | --- | --- | --- |
| `C0` | `[99]` | OSD show time | 5–60 s | 10 |
| `C1` | `[99]` | OSD horizontal position | 0–100 | 50 |
| `C2` | `[99]` | OSD vertical position | 0–100 | 50 |
| `C3` | `[99]` | OSD transparency | 0–100 in steps of 20 = OSD levels 0–5 (only writes with `99` work) | 0 |
| `C4` | `[99]` | OSD lock | 0/1 | 0 |
| `CA` | plain | Button lock | **1 = buttons 2 and 3 locked, 2 = unlocked**; Menu, Back and Power keep working | 2 |
| `CC` | `[99]` | OSD language | MCCS language codes (reported max 24) | — |
| `68` | plain | OSD language (second code) | 0 = English (max 5) | — |
| `D4` | `[99]` | Eyeshield (break) reminder | 0 off, 1 = 30 min, 2 = 1 h … 8 = 4 h; reported max 2 is wrong; **writes only 0/1** | 0 |
| `C6` | `[99]` | Power saving | 0 off, 1–3 level 1–3 | 0 |
| `61` | `[99]` | Quick Boot | 0/1 | 1 |

### Power and actions

| Code | Register | Write | Effect |
| --- | --- | --- | --- |
| `D6` | plain | 1 / 3 / 4 / 5 | MCCS power mode: 1 on, **3 turns the USB hub off**, 4 blanks the screen briefly (rest unknown), **5 turns the monitor off** (only the power button turns it on); 0 no effect, 2 untested. The screen blanks briefly on every change and the value cannot be read back |
| `D6` | `[99]` | 1 / 0 | DP Alt Mode on/off according to VMW (untested) |
| `04` | plain | 1 | Factory reset (brightness → 90). With `99` it does nothing |
| `05` | plain | 1 | Reset brightness/contrast (→ 90 / 50) |
| `08` | plain | 1 | Reset colors (color temperature → Warm; User RGB unchanged) |
| `C7` | `[99]` | **0** | Manufacturer factory reset (`SET 99 = 0`, `SET C7 = 0`); value 1 does nothing |
| `06 0A B0` | plain | 0–2 | Reset geometry / TV / save-restore settings: no visible effect |
| `BA` | plain | 1 | **Raises brightness to 100**; 0 does not undo it, 2 and writes with `99` do nothing. Possibly the Highlight brightness mode or a brightness reset (not checked in the OSD) |
| `F0` | plain | — | **Never write: 1 and 2 turn the monitor off** |

## Rules between settings

The monitor greys these out in the OSD and refuses or resets them over DDC. Titan Control follows the same rules.

- **PIP/PBP on:** DCR, HDR, Adaptive-Sync, DyDs, Dynamic OD, Game Rush, Halo Control, Full Game and all Game Aid
  overlays (refresh rate, crosshair, stopwatch, game time, magnifier, alignment, HawkEye) are off. PIP position and
  size apply only in PIP mode (not PBP).
- **Full Game 25" (`3A = 1`):** Adaptive-Sync, DyDs, Dynamic OD, Magnifier and HawkEye are off; Halo needs Wide.
- **Adaptive-Sync:** the Magnifier works only with Adaptive-Sync off; HawkEye works with it on. On firmware older
  than 2025-09-20, DyDs (`48` and the DyDs profiles `1B–1E`) also needs Adaptive-Sync off.
- **HDR on, firmware 2025-09-20:** locked are brightness, contrast, DCR, Low Blue Light, Color Enhance, CR Enhance,
  Shadow Balance, Output Range, the picture mode, and turning Local Dimming off (it switches to High). DyDs, Night
  Vision and the Magnifier are locked while Adaptive-Sync is off.
- **Magnifier and HawkEye** exclude each other.
- **DCR** excludes Local Dimming and DyDs (except Off), and locks brightness, contrast and Shadow Balance.
- **Local Dimming** is unavailable with DCR or DyDs ULL 1–3; **Halo Control** needs Local Dimming on, Wide, no DCR
  and no DyDs ULL.
- **Color Enhance > 0** locks the six saturation axes and sets them itself. Read from the monitor at each level
  (red, green, blue, yellow, cyan, magenta; at level 0 all are 50, the axes are free):

  | Level | R | G | B | Y | C | M |
  | --- | --- | --- | --- | --- | --- | --- |
  | 1 | 53 | 53 | 53 | 52 | 52 | 52 |
  | 2 | 57 | 57 | 56 | 53 | 53 | 53 |
  | 3 | 58 | 58 | 58 | 54 | 55 | 55 |
  | 4 | 59 | 59 | 60 | 55 | 56 | 56 |
  | 5 | 61 | 61 | 62 | 57 | 57 | 57 |
  | 6 | 63 | 63 | 64 | 58 | 59 | 58 |
  | 7 | 65 | 66 | 67 | 61 | 61 | 60 |
  | 8 | 68 | 69 | 70 | 63 | 63 | 62 |
  | 9 | 71 | 73 | 74 | 65 | 65 | 64 |
  | 10 | 74 | 76 | 77 | 67 | 67 | 66 |

  Titan Control shows these values on the locked saturation sliders.
- **LED:** with the LED off (`39 = 1`) the LED settings are locked; see [Lighting](#lighting) for per-mode limits.
- After writing one of these settings, Titan Control re-reads the ones that depend on it about 0.8 s later.

## Read-only values

| Code | Register | Meaning | Example |
| --- | --- | --- | --- |
| `AE` | plain | Refresh rate of the input signal, 0.01 Hz | `3e85` = 160.05 Hz |
| `AC` | plain | Horizontal frequency, 24 bits (`ml sh sl`; ddcutil shows only the low 16) | `0005/6284` = 352.9 kHz |
| `69` | plain | Active lines of the input signal | `0870` = 2160 |
| `F3` | plain | Usage time in minutes | `0a66` = 2662 min |
| `C0` | plain | Usage time in hours (≈ `F3`/60) | 44 |
| `FE` | plain | Firmware `0000/5101` = 5.1.1 (USB firmware; the same for every package). Inside a table stream it carries table fragments | |
| `FF` | plain | `0000/0200`; inside a table stream it repeats the last fragment | |
| `DF` | plain | MCCS version 2.1 (protocol, not firmware) | `0201` |
| `C8` | plain | Controller type: Mstar (`sl = 05`), controller number `sh = 0x56` | `0000/5605` |
| `F7` | plain | First 4 characters of the serial number (ASCII) | |
| `B2` | plain | Sub-pixel layout (MCCS) | 1 |
| `B6` | plain | Display technology (MCCS) | 3 |
| `0C` | plain | Color temperature request; a write of 1 is accepted, nothing changes | `00ff/0001` |

`AE`, `AC` and `69` describe the signal from the computer, not a monitor setting. Example: system mode
1920×1080 @ 120 Hz ↔ `69` = 1080, `AE` = 119.82 Hz, `AC` = 134.8 kHz.

The monitor's MCCS capabilities string (the same in both firmware packages) lists only standard codes:
`vcp(02 04 05 08 10 12 14(05 06 08 0B 0C 0D) 16 18 1A 60(11 12 0F 10) 62 AC AE B2 B6 C6 C8 C9 CC(…) D6(01 04 05) DF
FD E0 E1 E2 F0 F3 F7 FE FF)`, `mccs_ver(2.1)`. Manufacturer codes (via `99`) are not listed.

## Not available over DDC

- **Light sensor.**
  - VMW 1.0.2.8 has a "Light Sensor" page that writes `DA` via `99` (1 on, 0 off); see
    [Manufacturer software](#manufacturer-software-vmw).
  - On this firmware `DA [99]` is a placeholder and writes do nothing, which is why VMW hides the page here (also on
    Windows).
  - Scan of 2026-10-02: dark room, sensor off, brightness 40, brightness watched for 3 s after every write.
    - Writes tried: `DA` = 0, 1, 2, 3, 255, `0x0100`, `0x0101` (with `99` and plain), and 1 and 2 on ~120 codes
      without a known function.
    - No write turned the sensor on.
  - The sensor switch itself is not reported (`02`/`52` show only the brightness change it causes).
  - `13` (MCCS backlight, `0064/64xx`) changes its low byte independently of the light, so it is not a sensor reading.
  - None of the Enhanced firmware versions mention sensor control over DDC.
- **Brightness mode** (Normal / Highlight): code unknown. `BA = 1` (see [Power and actions](#power-and-actions)) is
  the only candidate. Plain `55` is not it (it returns leftovers like `FA`).
- **Screen mode** (Wide, 2K, FHD, 3K, 3K Wide; changes the resolution and physical picture size): no register found.
  Only the signal codes `69/AC/AE` and the mode list offered to the computer change.
- **DyDs/ULL Pulse, DyDs/LD Pulse, 3D mode, scene follow, shortcuts** from VMW 1.0.2.8: not in this firmware.
- **CEC (`D9`) and DP/USB-C link rate 5.4G/8.1G (`D7`):** listed by VMW. `D9` writes did nothing. `D7` was not tested
  on purpose, because changing the link rate can drop the picture.

## Code index 00–FF

Codes with a function are described above. The rest were tested in 11 sessions (2026-09-30 to 2026-10-02):

- reads of both registers in full scans, compared before and after every write session;
- writes of 1 and 2 via `99`;
- writes of 0–5 plain;
- writes of 0 and 1 via `99`;
- watching the picture, the OSD and the other registers.

**Known:**
`02 04 05 08 0C 10 12 14 16–1E 22 24 26 30–3B 3D–4D 50–57 59–66 68 69 6C 6E 70 87 8D 99 9B–A0 AC AE B2 B6 C0–C8 CA
CC D4–D6 D8 DC DF E0–E2 E4–E9 F0 F3 F6 F7 FE FF`, plus `BA` (brightness to 100), `DA` (light sensor, unsupported),
`D7` and `D9` (VMW, see above).

**Read-only values without a known meaning** (writes do nothing; `[99]` is a placeholder unless noted):

| Code | Plain reply | Note |
| --- | --- | --- |
| `0B` | `5ddc/0000` | value jumps after writes and comes back |
| `0E` | `0064/0032` | |
| `13` | `0064/64xx` | MCCS backlight; low byte 0x13/0x7A independent of the light |
| `20` | `0064/0000` | |
| `25` | `ffff/0006` | accepts 2, 4, 6 (other values → 6); no visible effect |
| `27` | `ffff/0007` | |
| `29` | `ffff/0000` | |
| `74` | `[99]` `2000/0b90` | constant |
| `A8` | `0003/0000` | |
| `B4` | `0002/0001` | |
| `C9` | `ffff/0000` | |
| `FA` | `ffff/0000` | after a write it returns leftovers of the previous reply (e.g. brightness) |
| `FD` | `ffff/0074` | 116 |

**No function found** (no reply or placeholder in both registers, no effect of any write):
`00 01 03 07 09 0D 0F 11 15 1F 21 23 28 2A–2F 3C 4E 4F 58 67 6A 6B 6D 6F 71–73 75–86 88–8C 8E–98 9A A1–A7 A9–AB AD
AF B1 B3 B5 B7–B9 BB–BF CB CD–D3 DB DD DE E3 EA–EF F1 F2 F4 F5 F8 F9 FB FC`.

## Manufacturer software (VMW)

VIEW MORE WIDGET is Titan Army's Windows tool (also used for INNOCN/MPCS models). It shows pages per detected model.

- **Captures** (Frida hooks on `dxva2`, 2026-09-29):
  - Every write is `SetVCPFeature(0x99, v)` → `SetVCPFeature(code, v)` (149 pairs, no other pattern).
  - Every read is `Get(0x99)` → `Get(code)`. Of 382 reads, 25 failed with `0xC0262589` (22 on `47`, 3 on `FE`).
  - After an error VMW waits ~324 ms and retries; for `FE` it switches to `GET FF`.
  - The capture contained no operations on `E0`, `E5–E9`, reads of `3A`, or the light sensor.
- **Static analysis of 1.0.2.8** (no need to run it):
  - The command table at `0x140122e20` holds 14-byte frame templates `6e 51 84 99 <op> <code> …` (op 03 = write,
    01 = read).
  - `HKC_SetCommandVal` (`0x1400090b0`, `ecx` = command id, `dx` = value) calls `0x140009650` twice:
    `dxva2!SetVCPFeature(handle, code, value)` first with byte 3 of the template (`99`), then with byte 5 (the code).
    So these are two standard VCP writes, not one non-standard frame. `setter.py da 1 --99` and
    `ddcutil setvcp 99 1 da 1` send exactly the same.
  - The table maps:
    - Light Sensor (command `0x118`) → `DA`;
    - Output Range → `60`, Quick Boot → `61`, USB Power → `D5`, USB Switch → `5F`, Input Signal → `57`;
    - picture mode → `22`, CEC → `D9`, DP Alt Mode → `D6` via `99`, DP link rate → `D7`.
  - Under Wine the tool has no DDC/CI (`hPhysicalMonitor 0x0`, model "Default") and exits with "DDC/CI erorr, App
    exit!".
- **1.0.2.8 release notes** (translated):
  - New: DyDs/ULL Pulse and DyDs/LD Pulse modes, grouped with DyDs/ULL FPS and DyDs/LD in a "DyDs Pulse" menu.
    - With FreeSync = 1, DyDs and Adaptive-Sync do not exclude each other; with FreeSync = 0 they do.
    - ULL Pulse disables Adaptive-Sync, DyDs Tech and local dimming.
    - LD Pulse disables Adaptive-Sync and DyDs Tech and turns local dimming on.
  - New: a 3D scene mode (sharpness, super resolution and dynamic OD greyed out), the light sensor (Basic info →
    Input/Output), shortcuts and scene follow. The Audio menu moved to Basic settings → System settings.
  - Fixes:
    - DyDs Tech greyed out in the MacView/sRGB/Adobe/DCI-P3 calibrated modes, and the DyDs Tech exclusion logic.
    - Color temperature and saturation values, Low Blue Light conversion, OSD transparency 0–5.
    - New exclusions between e-book mode, hue, saturation and color enhance.

## Hardware notes

- **Buttons** from the left: 1 Menu · 2 Custom 1 / left / down · 3 Custom 2 / right / up · 4 Back · 5 Power.
  Holding 1 and 4 until the monitor turns off restarts it.
- **Internal USB serial port:** a CH340 (`1a86:7523`, `/dev/ttyUSB0`) behind the monitor's USB hub. VMW ships
  `Qt6SerialPort.dll` and `hidapi.dll` but does not import their functions, and its logs show only DDC. Probably a
  service channel. It was never opened: opening sets DTR/RTS, which may reset the microcontroller.

## Tools

Python 3 scripts in this directory, no dependencies. They talk to `/dev/i2c-14` by default (`--bus N` for another
bus) and follow all the rules above (pacing, validation, `99` handling, retries). `titan_ddc.py` is the shared module.

```bash
./getter.py 10 c0 e1            # read codes (hex); register chosen automatically, printed as [99] or [plain]
./getter.py c0 e1 --both        # both registers
./getter.py 30 --plain          # plain register only (--99: manufacturer register only)
./getter.py 13 -n 5             # 5 reads
./getter.py all -o before.tsv   # every code, both registers (a few minutes)
./getter.py --diff before.tsv after.tsv

./setter.py c3 40               # write (code hex, value decimal), then verify
./setter.py e2 0                # E2 has only a plain register: written without 99
./setter.py c0 20 --99          # force the manufacturer register (--plain: plain)
./setter.py d6 1 --no-verify    # no read-back

./table_reader.py               # active profile and its confirmed table, decoded
./table_reader.py --raw -n 10   # 10 confirmed reads, raw bytes
```

- **Register choice (auto):** codes `00–1E` are read plain. Other codes are read after `GET 99`, falling back to the
  plain register when the manufacturer one is a placeholder. Writes use `99` when the value lives in `[99]` (and
  always for `00–1E`), plain otherwise; plain-only codes are handled automatically.
- **Setter result** (last line): `success` (read-back confirms), `sent` (written, not confirmed), `error` (I/O error).
  Exit codes 0 / 2 / 1. An unconfirmed write is resent once (`--retries`).
- **Guarded writes:** `22 = 0` is refused. Resets (`04 05 06 08 0A B0 C7`), `F0`, `CA = 1` and `D6 = 3/4/5` need
  `--force`.
- ddcutil equivalents:
  - manufacturer write: `ddcutil --bus 14 --noverify --permit-unknown-feature setvcp 99 1 47 1`
  - manufacturer read: `ddcutil --bus 14 -U getvcp 99 47`
- **Profile table:**
  - Rust (hardware test): `TITAN_I2C=/dev/i2c-14 cargo test profile_table -- --ignored --nocapture`.
  - App implementation: `MsiDdc::read_active_profile` in `src/monitor.rs`.
