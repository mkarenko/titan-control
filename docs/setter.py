#!/usr/bin/env python3
"""Write a VCP code like the manufacturer's program, then verify by reading it back the same way.

The register is chosen like in getter.py (auto): read after GET 99, or the plain register when the 99 one is an
empty placeholder (e.g. E2 Adaptive-Sync). Codes 00-1E are read plain, but written with 99 like VMW does.
Write [99]:    SET 99 = value -> SET <code> = value   (once, as captured with Frida)
Write [plain]: SET <code> = value
Verify: read the same register again and compare current with the value.

  ./setter.py c3 40             # code HEX, value DEC  (C3 = 40 -> OSD Transparency 2)
  ./setter.py e2 0              # auto: E2 is a plain register -> plain write
  ./setter.py 30 50 --plain     # force the plain register
  ./setter.py c0 20 --99        # force the register after 99
  ./setter.py d6 1 --no-verify
When the read-back does not confirm the value, the write is resent once (--retries, 0 = like VMW).
Last line: success (confirmed), sent (written, not confirmed), error (I/O failure / refused).
Exit code: 0 success, 1 error, 2 sent.
"""
import argparse, sys, time
from titan_ddc import Monitor, DdcError, NAMES, TABLE_SELECTORS, PLAIN_CODES, describe

# Writes that are destructive or hard to undo: require --force.
DANGEROUS = {
    (0x22, 0): "0x22 = 0 is forbidden",
    (0xCA, 1): "CA = 1 locks buttons 2 and 3 (undo: CA = 2)",
    (0xD6, 3): "D6 = 3 turns the USB hub off",
    (0xD6, 4): "D6 = 4 blanks the screen (power mode, effect unknown)",
    (0xD6, 5): "D6 = 5 turns the whole monitor off",
}
DANGEROUS_CODES = {0xF0: "F0: writing 1 or 2 turns the monitor off"}
RESET_CODES = {0x04: "factory reset", 0x05: "brightness/contrast reset", 0x06: "geometry reset",
               0x08: "color reset", 0x0A: "TV settings reset", 0xB0: "save/restore settings",
               0xC7: "factory reset"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("code", type=lambda x: int(x, 16))
    ap.add_argument("value", type=int)
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--plain", action="store_true", help="plain register: no SET/GET 99")
    mode.add_argument("--99", dest="only99", action="store_true", help="register after 99")
    ap.add_argument("--no-verify", action="store_true")
    ap.add_argument("--reads", type=int, default=5, help="verification attempts")
    ap.add_argument("--wait", type=float, default=0.3, help="pause after the write")
    ap.add_argument("--retries", type=int, default=1,
                    help="resend the write when the read-back does not confirm it (a single write is sometimes lost)")
    ap.add_argument("--force", action="store_true", help="allow dangerous writes")
    ap.add_argument("--bus", type=int, default=14)
    a = ap.parse_args()
    if not 0 <= a.code <= 0xFF or not 0 <= a.value <= 0xFFFF:
        ap.error("code 00-FF (HEX), value 0-65535 (DEC)")
    if a.code == 0x22 and a.value == 0:
        ap.error(DANGEROUS[(0x22, 0)])
    warn = DANGEROUS.get((a.code, a.value)) or DANGEROUS_CODES.get(a.code) or (RESET_CODES.get(a.code) and f"{a.code:02X} = {RESET_CODES[a.code]}")
    if warn and not a.force:
        ap.error(f"{warn}; add --force if you are sure")
    if warn:
        a.retries = 0  # never repeat a dangerous write

    if a.code in PLAIN_CODES and not a.only99:
        a.plain = True  # e.g. 39: only a plain SET changes the LED
    m = Monitor(bus=a.bus)
    m.settle()
    print(f"{a.code:02X} {NAMES.get(a.code, '')}")
    force = False if a.plain else (True if a.only99 else None)
    try:
        mx, cur, reg = m.get(a.code, False if a.code in TABLE_SELECTORS else force, 3)
        print(f"before [{reg}]: {mx:04x}/{cur:04x} = {describe(mx, cur, a.code)}")
    except DdcError as e:
        reg = "plain" if a.plain or a.code in TABLE_SELECTORS else "99"
        print(f"before [{reg}]: no reply ({e.kind})")
    write99 = not a.plain and (a.only99 or a.code in TABLE_SELECTORS or reg == "99")

    if a.no_verify:
        try:
            m.set(a.code, a.value, via99=write99)
        except DdcError as e:
            print(f"write: {e}")
            print("error")
            sys.exit(1)
        print(f"write: {'99=' + str(a.value) + ' -> ' if write99 else ''}{a.code:02X}={a.value}")
        print("sent")
        sys.exit(2)

    after = None
    for attempt in range(a.retries + 1):
        try:
            m.set(a.code, a.value, via99=write99)
        except DdcError as e:
            print(f"write: {e}")
            print("error")
            sys.exit(1)
        print(f"write{' (resent)' if attempt else ''}: {'99=' + str(a.value) + ' -> ' if write99 else ''}{a.code:02X}={a.value}")
        time.sleep(a.wait)
        for _ in range(a.reads):
            try:
                after = m.get(a.code, reg == "99", 2)
            except DdcError:
                continue
            if after[1] == a.value:
                break
        if after and after[1] == a.value:
            break
    if after:
        print(f"after  [{reg}]: {after[0]:04x}/{after[1]:04x} = {describe(after[0], after[1], a.code)}")
    else:
        print(f"after  [{reg}]: no reply")
    if after and after[1] == a.value:
        print("success")
        return
    print("sent")
    sys.exit(2)

if __name__ == "__main__":
    main()
