#!/usr/bin/env python3
"""Read VCP codes like the manufacturer's program: GET 99 -> GET <code>.

Default (auto): codes 00-1E plain GET; other codes after GET 99, and when that register is an empty placeholder
(00ff/0000) the plain register instead. The register used is always printed: [99] or [plain].
The same code can hold two different values: e.g. C0 [99] = OSD Show Time, C0 [plain] = usage hours.

  ./getter.py 10 c0 e1          # codes in HEX
  ./getter.py c0 e1 --both      # both registers
  ./getter.py 30 --plain        # plain register only
  ./getter.py c3 --99           # register after 99 only
  ./getter.py 13 -n 5           # repeat 5 times
  ./getter.py all -o /tmp/a.tsv # every code 00-FF, both registers, saved to a file (a few minutes)
  ./getter.py --diff /tmp/a.tsv /tmp/b.tsv
Exit code: 0 all read, 1 at least one code without a valid reply.
"""
import argparse, sys
from titan_ddc import Monitor, DdcError, NAMES, TABLE_SELECTORS, describe


def diff(path_a, path_b):
    def load(path):
        rows = {}
        for line in open(path):
            p = line.rstrip("\n").split("\t")
            rows[(p[0], p[1])] = p[2]
        return rows
    a, b = load(path_a), load(path_b)
    changed = [(k, a.get(k, "—"), b.get(k, "—")) for k in sorted(set(a) | set(b)) if a.get(k) != b.get(k)]
    for (code, reg), va, vb in changed:
        print(f"{code} [{reg:5}] {va} -> {vb}  {NAMES.get(int(code, 16), '')}")
    if not changed:
        print("no differences")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("codes", nargs="*", help="HEX codes or 'all'")
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--plain", action="store_true", help="plain GET only (standard MCCS register)")
    mode.add_argument("--99", dest="only99", action="store_true", help="register after GET 99 only")
    mode.add_argument("--both", action="store_true", help="both registers")
    ap.add_argument("-n", type=int, default=1, help="repeat each read")
    ap.add_argument("--tries", type=int, default=6)
    ap.add_argument("-o", "--output", help="also save TSV: code, register, maximum/current")
    ap.add_argument("--diff", nargs=2, metavar=("A", "B"), help="compare two TSV files")
    ap.add_argument("--bus", type=int, default=14)
    a = ap.parse_args()
    if a.diff:
        diff(*a.diff)
        return
    if not a.codes:
        ap.error("give HEX codes or 'all'")
    if a.codes == ["all"]:
        codes = [c for c in range(0x100) if c != 0x99]
        a.both = a.both or not (a.plain or a.only99)
        a.tries = min(a.tries, 3)
    else:
        codes = [int(c, 16) for c in a.codes]

    m = Monitor(bus=a.bus)
    m.settle()
    out = open(a.output, "w") if a.output else None
    failed = False
    for code in codes:
        if a.plain or code in TABLE_SELECTORS:
            paths = [False]
        elif a.only99:
            paths = [True]
        elif a.both:
            paths = [True, False]
        else:
            paths = [None]
        for via99 in paths:
            for _ in range(a.n):
                try:
                    mx, cur, reg = m.get(code, via99, a.tries)
                    print(f"{code:02X} [{reg:5}] {mx:04x}/{cur:04x}  = {describe(mx, cur, code):28} {NAMES.get(code, '')}",
                          flush=True)
                    if out:
                        out.write(f"{code:02X}\t{reg}\t{mx:04x}/{cur:04x}\n")
                except DdcError as e:
                    failed = True
                    reg = {True: "99", False: "plain", None: "auto"}[via99]
                    print(f"{code:02X} [{reg:5}] no reply ({e.kind})  {NAMES.get(code, '')}", flush=True)
                    if out:
                        out.write(f"{code:02X}\t{reg}\tnone\n")
    if out:
        out.close()
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
