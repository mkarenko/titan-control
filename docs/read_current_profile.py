#!/usr/bin/env python3
"""Read current VCP settings or the reported profile's WMW data stream."""

import argparse
import atexit
import ctypes
import math
import os
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass


PROFILES = ["Standard", "RTS/RPG", "FPS", "MOBA", "Movie", "Reading",
            "Night", "Eye Care", "MacView", "E-book", "sRGB", "AdobeRGB",
            "DCI-P3", "DyDs/ULL FPS", "DyDs/LD"]
# Positions follow the documented MH/ML/SH/SL stream starting at offset 2.
FIELDS = [
    (2, 0x10, "Brightness"), (3, 0x12, "Contrast"),
    (4, 0x87, "Sharpness"), (5, 0x40, "Color Enhancement"),
    (6, 0x41, "CR Enhancement"), (7, 0x42, "Shadow Balance"),
    (8, 0x14, "Color Temperature"),
    *[(9 + i, 0x16 + i, f"User {i // 3 + 1} {'RGB'[i % 3]}") for i in range(9)],
    (18, 0x9B, "Hue R"), (19, 0x9D, "Hue G"), (20, 0x9F, "Hue B"),
    (21, 0x9C, "Hue Y"), (22, 0x9E, "Hue C"), (23, 0xA0, "Hue M"),
    (24, 0x59, "Saturation R"), (25, 0x5B, "Saturation G"),
    (26, 0x5D, "Saturation B"), (27, 0x5A, "Saturation Y"),
    (28, 0x5C, "Saturation C"), (29, 0x5E, "Saturation M"),
    (30, 0xD8, "Low Blue Light"), (31, 0x4A, "HDR"),
    (33, 0x26, "Gamma"), (34, 0x44, "Super Resolution"),
    (35, 0x45, "Night Vision"), (36, 0x49, "Dynamic OD"),
]
GLOBAL_FIELDS = [(0x47, "Local Dimming"), (0x48, "DyDs"),
                 (0x46, "Halo Control"), (0xE1, "DCR"),
                 (0xE2, "Adaptive Sync"), (0x43, "Game Rush")]
ENUMS = {
    0x14: {5: "Warm", 6: "Natural", 8: "Cool", 11: "User 1", 12: "User 2", 13: "User 3"},
    0x26: {2: "1.8", 4: "2.0", 6: "2.2", 8: "2.4", 10: "2.6", 12: "S-Curve"},
    0x4A: {0: "Off", 1: "Auto", 2: "Game", 3: "Movie"},
    0x45: {0: "Off", 1: "Level 1", 2: "Level 2", 3: "Auto-Level 1", 4: "Auto-Level 2"},
    0x49: {0: "Off", 1: "Level 1", 2: "Level 2", 3: "Level 3", 4: "Topspeed"},
    0xE1: {0: "Off", 1: "On"}, 0xE2: {0: "Off", 1: "On"},
    0x43: {0: "Off", 1: "On"},
}


class ReadError(Exception):
    pass


class Unsupported(ReadError):
    pass


class NullReply(ReadError):
    pass


@dataclass(frozen=True)
class Reply:
    maximum: int
    current: int

    def payload(self):
        return self.maximum.to_bytes(2, "big") + self.current.to_bytes(2, "big")


class Monitor:
    def __init__(self, bus, address, delay, debug=False):
        self.bus, self.address, self.delay, self.debug = bus, address, delay, debug
        self.last_request = None

    def transfer(self, *args):
        try:
            result = subprocess.run(
                ["i2ctransfer", "-y", str(self.bus), *args],
                capture_output=True, text=True, timeout=5, check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise ReadError(f"transmisja: {error}") from error
        if result.returncode:
            raise ReadError(result.stderr.strip() or "błąd transmisji I2C")
        return result.stdout.strip()

    def get(self, code):
        # Match the observed ~60-65 ms start-to-start cadence where possible.
        # Process/driver latency may make the actual interval longer.
        if self.last_request is not None:
            time.sleep(max(0, 0.065 - (time.monotonic() - self.last_request)))
        request = [0x51, 0x82, 0x01, code]
        checksum = 0x6E
        for byte in request:
            checksum ^= byte
        self.last_request = time.monotonic()
        self.transfer(f"w5@0x{self.address:02x}",
                      *(f"0x{x:02x}" for x in request + [checksum]))
        time.sleep(self.delay)
        raw = self.transfer(f"r11@0x{self.address:02x}")
        if self.debug:
            print(f"GET {code:02X}: {raw}", file=sys.stderr)
        tokens = raw.split()
        if any(len(t) != 4 or t[:2].lower() != "0x" or
               any(c not in "0123456789abcdefABCDEF" for c in t[2:]) for t in tokens):
            raise ReadError("niepoprawny format bajtów")
        data = bytes(int(t, 16) for t in tokens)
        if data[:3] == bytes([0x6E, 0x80, 0xBE]):
            # NULL is a complete three-byte message; the remainder of r11
            # is outside that message and must not be decoded as VCP data.
            raise NullReply("NULL — brak wartości")
        if len(data) != 11 or data[:3] != bytes([0x6E, 0x88, 0x02]):
            raise ReadError("niepoprawny nagłówek lub długość odpowiedzi")
        checksum = 0x50
        for byte in data:
            checksum ^= byte
        if checksum:
            raise ReadError("niepoprawny checksum")
        if data[4] != code:
            raise ReadError(f"odpowiedź dla {data[4]:02X}, oczekiwano {code:02X}")
        if data[3] == 1:
            raise Unsupported("odczyt nieobsługiwany")
        if data[3] != 0:
            raise ReadError(f"niepoprawny status {data[3]:02X}")
        return Reply(int.from_bytes(data[6:8], "big"), int.from_bytes(data[8:10], "big"))

    def prime(self):
        try:
            self.get(0x99)
        except (NullReply, Unsupported) as error:
            # 99 supplies no setting here. A valid empty/rejected reply
            # permits attempting the target, whose data must pass validation.
            if self.debug:
                print(f"GET 99: {error}; kontynuuję odczyt docelowy", file=sys.stderr)

    def primed_get(self, code):
        self.prime()
        return self.get(code)


def stable_profile(monitor):
    first = monitor.primed_get(0x22).current
    second = monitor.primed_get(0x22).current
    if first != second:
        raise ReadError(f"niestabilny numer profilu: {first} → {second}; ponów odczyt")
    if not 1 <= first <= 30:
        raise ReadError(f"nieznany numer profilu {first} (0x{first:04X})")
    return first


class I2CMessage(ctypes.Structure):
    _fields_ = [("addr", ctypes.c_uint16), ("flags", ctypes.c_uint16),
                ("length", ctypes.c_uint16), ("buffer", ctypes.POINTER(ctypes.c_uint8))]


class I2CTransfer(ctypes.Structure):
    _fields_ = [("messages", ctypes.POINTER(I2CMessage)), ("count", ctypes.c_uint32)]


class NativeMonitor(Monitor):
    """Keep one i2c-dev descriptor open; no process between stream fragments."""

    def __init__(self, bus, address, delay, debug=False):
        super().__init__(bus, address, delay, debug)
        try:
            self.fd = os.open(f"/dev/i2c-{bus}", os.O_RDWR | os.O_CLOEXEC)
        except OSError as error:
            raise ReadError(f"otwarcie /dev/i2c-{bus}: {error}") from error
        atexit.register(os.close, self.fd)
        self.ioctl = ctypes.CDLL(None, use_errno=True).ioctl
        self.ioctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_void_p]
        self.ioctl.restype = ctypes.c_int

    def transfer(self, *args):
        spec, *values = args
        operation, address = spec.split("@")
        count = int(operation[1:])
        reading = operation[0] == "r"
        if (operation[0] not in ("r", "w") or count not in (5, 11) or
                int(address, 16) != self.address or
                len(values) != (0 if reading else count)):
            raise ReadError("niepoprawne parametry transmisji")
        buffer = (ctypes.c_uint8 * count)(*(int(value, 16) for value in values))
        message = I2CMessage(self.address, 1 if reading else 0, count, buffer)
        transfer = I2CTransfer(ctypes.pointer(message), 1)
        # Separate single-message transfers preserve the STOP and delay
        # between the DDC request and reply; no combined repeated START.
        result = self.ioctl(self.fd, 0x0707, ctypes.byref(transfer))  # I2C_RDWR
        if result < 0:
            error = ctypes.get_errno()
            raise ReadError(f"I2C_RDWR: {os.strerror(error)} (errno={error})")
        if result != 1:
            raise ReadError(f"niepełna transmisja I2C: {result}/1")
        return " ".join(f"0x{value:02x}" for value in buffer) if reading else ""


def read_table(monitor, profile):
    # The caller has just executed GET 99 -> GET 22. Continue that exact
    # sequence immediately: extra selector probes may alter firmware context.
    # On FE failure follow WMW's FF -> FE path to finish the stream, but
    # never decode that recovered stream: cursor advancement is unproven.
    chunks = [monitor.get(profile).payload()]
    recovered = False
    ended = False
    for _ in range(14):
        try:
            chunk = monitor.get(0xFE).payload()
        except ReadError as error:
            if recovered:
                raise
            recovered = True
            print(f"GET FE: {error}; odzyskiwanie WMW przez GET FF → GET FE.", file=sys.stderr)
            time.sleep(0.260)
            chunk = monitor.get(0xFF).payload()
        if chunk == bytes([0, 0, 0x51, 1]):
            ended = True
            break
        chunks.append(chunk)
    if not ended:
        raise ReadError("nieoczekiwany koniec strumienia tabeli (oczekiwano 0000/5101)")
    if recovered:
        raise ReadError("strumień zakończony po FF, ale pozycja odzyskanych bajtów jest niepotwierdzona; wymagany nowy odczyt")
    if len(chunks) != 10:
        raise ReadError(f"niepoprawna długość tabeli: {len(chunks) * 4} zamiast 40 bajtów")
    data = bytes([0, 0]) + b"".join(chunks)
    if (data[2] > 100 or data[3] > 100 or data[4] > 5 or
            data[5] > 10 or data[6] > 5 or data[7] > 100 or data[8] not in ENUMS[0x14]):
        raise ReadError("dane nie pasują do przechwyconego schematu tabeli; brak dekodowania")
    return data


def read_live(monitor, fields):
    result = []
    for code, name in fields:
        for attempt in range(3):
            try:
                reply = monitor.primed_get(code)
                value = reply.current
                source = ""
                if code == 0x46 and reply.maximum >> 8 == 0xFF and reply.maximum & 255 <= 100:
                    value = reply.maximum & 255
                    source = " [vendor ML]"
                result.append((code, name, value, source))
                break
            except Unsupported as error:
                result.append((code, name, None, f"nieobsługiwany odczyt: {error}"))
                break
            except ReadError as error:
                if attempt == 2:
                    result.append((code, name, None, f"BŁĄD: {error}"))
                else:
                    time.sleep(monitor.delay)
    return result


def read_table_snapshot(monitor, attempts):
    for attempt in range(1, attempts + 1):
        try:
            profile = monitor.primed_get(0x22).current
            if not 1 <= profile <= 30:
                raise ReadError(f"nieznany numer profilu {profile} (0x{profile:04X})")
            data = read_table(monitor, profile)
            # The stream is complete. Retry only the independent final
            # selector query; never retry or join individual FE fragments.
            final_profile = None
            final_error = None
            for check in range(3):
                try:
                    final_profile = monitor.primed_get(0x22).current
                    break
                except ReadError as error:
                    final_error = error
                    if check == 2:
                        break
                    time.sleep(0.260)
            if final_profile is None:
                print(f"UWAGA: tabela poprawna, ale brak potwierdzenia profilu po odczycie: {final_error}",
                      file=sys.stderr)
                return profile, data, False
            if final_profile != profile:
                raise ReadError(f"numer profilu zmienił się podczas odczytu: {profile} → {final_profile}")
            return profile, data, True
        except ReadError as error:
            if attempt == attempts:
                raise ReadError(f"odczyt tabeli nieudany po {attempts} próbach: {error}") from error
            print(f"Próba {attempt}/{attempts}: {error}. Odrzucam całą tabelę; ponawiam od początku.",
                  file=sys.stderr)
            time.sleep(max(0.260, monitor.delay))


def show_fields(rows):
    print(f"{'VCP':<5} {'Ustawienie':<23} {'DEC / HEX':<18} Interpretacja")
    for code, name, value, note in rows:
        if value is None:
            print(f"{code:02X}    {name:<23} {'—':<18} {note}")
        else:
            label = ENUMS.get(code, {}).get(value, "")
            print(f"{code:02X}    {name:<23} {f'{value} / 0x{value:04X}':<18} {label}{note}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bus", type=int, default=14)
    parser.add_argument("--addr", type=lambda s: int(s, 0), default=0x37)
    parser.add_argument("--delay", type=float, default=0.040,
                        help="czas od wysłania zapytania do odczytu odpowiedzi (domyślnie 0.04 s)")
    parser.add_argument("--attempts", type=int, default=4,
                        help="maksymalna liczba pełnych prób tabeli (domyślnie 4)")
    parser.add_argument("--table", action="store_true", help="odczyt tabeli WMW zamiast bieżących VCP")
    parser.add_argument("--debug", action="store_true", help="surowe ramki na stderr")
    parser.add_argument("--transport", choices=("native", "i2ctransfer"), default="native",
                        help="native: jeden otwarty deskryptor I2C (domyślnie); i2ctransfer: poprzednia metoda")
    args = parser.parse_args()
    if args.bus < 0 or not 0x03 <= args.addr <= 0x77 or not math.isfinite(args.delay) or not 0 <= args.delay <= 10:
        parser.error("niepoprawny bus/addr/delay")
    if not 1 <= args.attempts <= 20:
        parser.error("attempts musi być od 1 do 20")
    if args.transport == "i2ctransfer" and not shutil.which("i2ctransfer"):
        raise ReadError("brak i2ctransfer — zainstaluj i2c-tools")
    monitor_class = NativeMonitor if args.transport == "native" else Monitor
    monitor = monitor_class(args.bus, args.addr, args.delay, args.debug)
    if args.debug:
        print(f"Transport: {args.transport}; bus={args.bus}, delay={args.delay:.3f}s", file=sys.stderr)
    profile_confirmed = True
    if args.table:
        profile, data, profile_confirmed = read_table_snapshot(monitor, args.attempts)
    else:
        profile = stable_profile(monitor)
    family = PROFILES[(profile - 1) // 2]
    variant = "Default" if profile % 2 else "Custom"
    if args.table:
        rows = [(code, name, data[pos], "") for pos, code, name in FIELDS]
        globals_rows = []
        source = "Tabela WMW dla numeru odczytanego z 22 (schemat z przechwycenia)"
    else:
        rows = read_live(monitor, [(code, name) for _, code, name in FIELDS])
        globals_rows = read_live(monitor, GLOBAL_FIELDS)
        data = None
        source = "Bieżące wartości Get VCP; pola nieczytelne pozostają oznaczone"
    if not args.table and stable_profile(monitor) != profile:
        raise ReadError("profil zmienił się podczas odczytu; wyniki ustawień odrzucone")
    print(f"Profil raportowany: {family} / {variant} (22={profile}, 0x{profile:02X})")
    print("Nazwa i wariant: mapa WMW; zgodność Default/Custom z OSD nie jest gwarantowana.")
    if not profile_confirmed:
        print("UWAGA: profil odczytano przed tabelą; nie potwierdzono jego stanu po odczycie.")
    print(f"Bus: {args.bus}, adres: 0x{args.addr:02X}. Źródło: {source}.\n")
    show_fields(rows)
    if globals_rows:
        print("\nUstawienia globalne monitora:")
        show_fields(globals_rows)
    if data is not None:
        print("\nBajty tabeli (od pozycji 2): " + data[2:].hex(" "))
        known = {pos for pos, _, _ in FIELDS}
        print("Pola nieprzypisane: " + ", ".join(f"[{pos}]={data[pos]}" for pos in range(2, len(data)) if pos not in known))
    incomplete = any(value is None for _, _, value, _ in rows + globals_rows)
    if incomplete or not profile_confirmed:
        print("\nOdczyt częściowy: nie wszystkie pola lub końcowy stan profilu zostały potwierdzone.")
        return 2
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except ReadError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        sys.exit(130)
