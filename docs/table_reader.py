#!/usr/bin/env python3
"""Read the active picture profile and its settings table, the way the manufacturer's program (VMW) does.

Sequence: GET 99 (fafa/fafa) -> GET 22 (active profile) -> GET <profile> -> GET FE x 9 -> GET FE (0000/5101 = end).
A lost reply inside the stream is recovered: NULL = fragment not consumed, the same request is repeated;
garbage = fragment consumed, GET FF sends it again. Any other error restarts the whole sequence. A table counts only
after two identical reads in a row (see p275mv_plus.md, "Profile table").

Only Custom profiles (even codes) have a readable table; for a Default profile the settings are read one by one.
Read-only: nothing is written to the monitor.

  ./table_reader.py              # active profile and its decoded settings
  ./table_reader.py --raw        # also the 40 raw table bytes
  ./table_reader.py -n 10        # 10 confirmed reads in a row (reliability test)
Exit code: 0 read, 1 failure.
"""
import argparse, sys, time
from titan_ddc import Monitor, DdcError, RETRY_WAIT

PROFILES = ["Standard", "RTS", "FPS", "MOBA", "Movie", "Reading", "Night", "Eye Care", "MacView", "E-book", "sRGB",
            "AdobeRGB", "DCI-P3", "DyDs/ULL FPS", "DyDs/LD"]
END = bytes([0, 0, 0x51, 0x01])
FRAGMENTS = 10
BUDGET = 60.0  # seconds per confirmed table

COLOR_TEMP = {5: "Warm", 6: "Natural", 8: "Cool", 11: "User 1", 12: "User 2", 13: "User 3"}
HDR = ["Off", "Auto", "Game", "Movie"]
GAMMA = {2: "1.8", 4: "2.0", 6: "2.2", 8: "2.4", 10: "2.6", 12: "S-Curve"}
NIGHT_VISION = ["Off", "Lvl 1", "Lvl 2", "Auto Lvl 1", "Auto Lvl 2"]
DYNAMIC_OD = ["Off", "Lvl 1", "Lvl 2", "Lvl 3", "Top Speed"]

# Table index -> (name, VCP code when read one by one, value names). Indices 0, 1 and 32 are empty.
LAYOUT = {
    2: ("Brightness", 0x10, None), 3: ("Contrast", 0x12, None), 4: ("Sharpness", 0x87, None),
    5: ("Color Enhance", 0x40, None), 6: ("CR Enhance", 0x41, None), 7: ("Shadow Balance", 0x42, None),
    8: ("Color temperature", 0x14, COLOR_TEMP),
    9: ("User 1 red", 0x16, None), 10: ("User 1 green", None, None), 11: ("User 1 blue", 0x18, None),
    12: ("User 2 red", None, None), 13: ("User 2 green", 0x1A, None), 14: ("User 2 blue", None, None),
    15: ("User 3 red", None, None), 16: ("User 3 green", None, None), 17: ("User 3 blue", None, None),
    18: ("Hue red", 0x9B, None), 19: ("Hue green", 0x9D, None), 20: ("Hue blue", 0x9F, None),
    21: ("Hue yellow", 0x9C, None), 22: ("Hue cyan", 0x9E, None), 23: ("Hue magenta", 0xA0, None),
    24: ("Saturation red", 0x59, None), 25: ("Saturation green", 0x5B, None), 26: ("Saturation blue", 0x5D, None),
    27: ("Saturation yellow", 0x5A, None), 28: ("Saturation cyan", 0x5C, None), 29: ("Saturation magenta", 0x5E, None),
    30: ("Low Blue Light", 0xD8, None), 31: ("HDR", 0x4A, HDR), 33: ("Gamma", 0x26, GAMMA),
    34: ("Super Resolution", 0x44, None), 35: ("Night Vision", 0x45, NIGHT_VISION), 36: ("Dynamic OD", 0x49, DYNAMIC_OD),
}


def profile_name(code):
    name = PROFILES[(code - 1) // 2] if 1 <= code <= 2 * len(PROFILES) else "unknown"
    return f"0x{code:02X} {name} ({'Custom' if code % 2 == 0 else 'Default'})"


def fragment(m, code, received):
    """Next 4 table bytes (GET <code>), with the NULL / garbage recovery described above."""
    request = code
    for _ in range(5):
        try:
            maximum, current = m.get_once(request)
        except DdcError as e:
            if e.kind in ("i2c", "unsupported"):
                raise
            time.sleep(RETRY_WAIT)
            if e.kind == "invalid":
                request = 0xFF
            continue
        if request == 0xFF and (maximum, current) == (0, 0x0200):
            # FF outside the stream: the stream had ended, so the lost reply was the terminator.
            if received == FRAGMENTS:
                return END
            raise DdcError("stream", f"FF outside the stream after {received} fragments")
        return maximum.to_bytes(2, "big") + current.to_bytes(2, "big")
    raise DdcError("stream", f"fragment {received + 1} lost")


def read_once(m):
    """One table read: (profile, 40 bytes) for a Custom profile, (profile, None) for a Default one."""
    if not m.settle():
        raise DdcError("settle", "the monitor does not answer")
    for _ in range(8):
        try:
            if m.get_once(0x99) == (0xFAFA, 0xFAFA):
                break
        except DdcError:
            pass
        time.sleep(RETRY_WAIT)
        m.settle()
    else:
        raise DdcError("99", "no fafa/fafa")
    maximum, profile = m.get_once(0x22)
    if maximum != 0x26 or not 1 <= profile <= 30:
        raise DdcError("22", f"{maximum:04x}/{profile:04x}")
    if profile % 2:
        # Default profile: no table. Close the sequence so the next request is not taken as a table read.
        try:
            m.get_once(0xDF)
        except DdcError:
            pass
        return profile, None
    chunks = [fragment(m, profile, 0)]
    while True:
        chunk = fragment(m, 0xFE, len(chunks))
        if chunk == END:
            break
        chunks.append(chunk)
        if len(chunks) > FRAGMENTS:
            raise DdcError("stream", "no end marker")
    if len(chunks) != FRAGMENTS:
        raise DdcError("stream", f"{len(chunks)} fragments instead of {FRAGMENTS}")
    return profile, bytes(2) + b"".join(chunks)


def read_confirmed(m):
    """Repeats table reads until two in a row agree. Returns (profile, data or None, attempts)."""
    previous, attempts, last = None, 0, None
    deadline = time.monotonic() + BUDGET
    while time.monotonic() < deadline:
        attempts += 1
        try:
            current = read_once(m)
        except DdcError as e:
            previous, last = None, e
            time.sleep(0.5)
            continue
        if current == previous:
            return current[0], current[1], attempts
        previous = current
    raise DdcError("budget", f"no confirmed table in {BUDGET:.0f} s ({attempts} attempts, last: {last})")


def show(index, value):
    name, _, names = LAYOUT[index]
    if index == 30:
        text = f"{value} (level {(value + 12) // 25})"
    elif isinstance(names, dict):
        text = f"{value} ({names.get(value, '?')})"
    elif isinstance(names, list):
        text = f"{value} ({names[value] if value < len(names) else '?'})"
    else:
        text = str(value)
    print(f"  {name:20} {text}")


def read_one_by_one(m):
    """Settings of a Default profile, one GET each (the User RGB gains not readable this way are skipped)."""
    for index, (name, code, _) in LAYOUT.items():
        if code is None:
            continue
        try:
            _, value, _ = m.get(code, None, 4)
        except DdcError as e:
            print(f"  {name:20} no reply ({e.kind})")
            continue
        if index == 30:
            value *= 25  # 0xD8 is the level 0-4; show it like the table does
        show(index, value)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("-n", type=int, default=1, help="number of confirmed reads")
    ap.add_argument("--raw", action="store_true", help="print the raw table bytes")
    ap.add_argument("--bus", type=int, default=14)
    a = ap.parse_args()
    m = Monitor(bus=a.bus)
    failed = 0
    for run in range(a.n):
        started = time.monotonic()
        try:
            profile, data, attempts = read_confirmed(m)
        except DdcError as e:
            failed += 1
            print(f"{run + 1}: failed: {e}" if a.n > 1 else f"failed: {e}")
            continue
        took = time.monotonic() - started
        prefix = f"{run + 1}: " if a.n > 1 else ""
        print(f"{prefix}Active profile: {profile_name(profile)}, confirmed after {attempts} reads, {took:.1f} s")
        if data is None:
            if a.n == 1:
                print("  Default profiles have no readable table; settings read one by one:")
                read_one_by_one(m)
            continue
        if a.raw:
            print("  raw: " + data.hex(" "))
        if a.n == 1:
            for index in LAYOUT:
                show(index, data[index])
    if a.n > 1:
        print(f"\n{a.n - failed}/{a.n} confirmed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
