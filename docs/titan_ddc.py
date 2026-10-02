"""DDC/CI access to the Titan Army P275MV PLUS, the way the manufacturer's program (VMW) does it.

Protocol summary (details in p275mv_plus.md):
- /dev/i2c-14 (DP AUX of DP-3), address 0x37; one request per I2C transfer, >= 100 ms between requests.
- GET:  51 82 01 <code> <chk>, wait 40 ms, read 11 bytes; reply 6e 88 02 <status> <code> <type> mh ml sh sl <chk>.
  6e 80 be = NULL (busy). Anything else (wrong header/code/checksum) = invalid, retried after a pause.
- Manufacturer registers are read with GET 99 (reply fafa/fafa) right before GET <code>; the same code read
  without 99 may be a DIFFERENT (standard MCCS) register, e.g. C0: after 99 = OSD Show Time, plain = usage hours.
- Codes 00-1E after GET 99 start a profile table stream, so they are always read plain.
- SET:  51 84 03 <code> hi lo <chk>. VMW sends SET 99 = value, then SET <code> = value (once, no retries).
"""
import ctypes, os, time

BUS = 14
ADDR = 0x37
GAP = 0.100          # minimum start-to-start interval of requests
REPLY_DELAY = 0.040  # write -> read
RETRY_WAIT = 0.324   # pause after a failed reply (as VMW)
WRITE_GAP = 0.060    # SET 99 -> SET code
TABLE_SELECTORS = range(0x00, 0x1F)
# Codes whose working register is the plain one although [99] answers with data (confirmed by tests).
PLAIN_CODES = {0x02, 0x39, 0x04, 0x05, 0x06, 0x08, 0x0A, 0xB0}  # 02: clears the OSD-change flag; 39: LED; MCCS resets work only with a plain SET (C7 works with 99)

# Confirmed or documented names (p275mv_plus.md). "99"/"plain" marks the register when both exist.
NAMES = {
    0x02: "New control value (2 = changed in OSD)", 0x10: "Brightness", 0x12: "Contrast", 0x14: "Color Temperature",
    0x16: "User 1 Red", 0x17: "User 1 Green", 0x18: "User 1 Blue", 0x19: "User 2 Red", 0x1A: "User 2 Green",
    0x1B: "User 2 Blue", 0x1C: "User 3 Red", 0x1D: "User 3 Green", 0x1E: "User 3 Blue", 0x22: "Picture profile",
    0x26: "Gamma", 0x3A: "Full Game: 0 Wide, 1 25\" (DDC accepts only 0/1)", 0x40: "Color Enhance", 0x41: "CR Enhance", 0x42: "Shadow Balance",
    0x43: "Game Rush", 0x44: "Super Resolution", 0x45: "Night Vision", 0x46: "Halo Control", 0x47: "Local Dimming",
    0x48: "DyDs", 0x49: "Dynamic OD", 0x4A: "HDR", 0x60: "Output Range (99) / Input Source (plain, 15 = DP-1)", 0x61: "Quick Boot",
    0x62: "Volume (MCCS)", 0x69: "Input signal active lines", 0x6C: "Black Level Red", 0x6E: "Black Level Green",
    0x70: "Black Level Blue", 0x87: "Sharpness", 0xAC: "Horizontal frequency", 0xAE: "Refresh rate (0.01 Hz)",
    0xC0: "OSD Show Time (99) / usage hours (plain)", 0xC1: "OSD H-Position", 0xC2: "OSD V-Position",
    0xC3: "OSD Transparency (0-100, step 20)", 0xC5: "Power LED", 0xC6: "Power Saving 0-3 (99) / app key (plain)", 0xC8: "Controller type",
    0xCA: "Button lock: 1 = buttons 2/3 locked, 2 = unlocked", 0xCC: "OSD Language", 0xD6: "Power mode (3 = USB hub off, 5 = monitor off)",
    0xDF: "VCP version", 0xE1: "DCR (99) / Low Blue Light 0-4 (plain)", 0xE2: "Adaptive-Sync", 0xE5: "LED color", 0xE6: "LED strength",
    0xE7: "LED mode", 0xE8: "LED front color", 0xE9: "LED rear color", 0xF3: "Usage minutes", 0xF6: "Volume",
    0xF7: "Audio Mute (99) / serial number 4 chars (plain)", 0xD8: "Low Blue Light 0-4", 0xC7: "Factory reset (99, value 0)", 0xDC: "Display Application (MCCS): 0 Standard, 3 Movie, 5 Games", 0xD4: "Eyeshield Reminder 0-8 (0 off, 1 = 30 min ... 8 = 4 h)", 0x24: "Aspect ratio: 1 wide, 2 4:3, 3 1:1, 4 21:9, 5 auto",
    0x31: "Refresh Rate Position 0-3", 0x33: "Stopwatch Time", 0x36: "Game Time 1-4 (15/30/45/60 min)",
    0x37: "Game Time Position", 0x3E: "Stopwatch Enable", 0x68: "OSD Language (0 = English)", 0x8D: "Audio Mute (MCCS)",
    0xE0: "Picture profile change 0-14", 0xE4: "Adaptive-Sync (2nd code)", 0xF0: "!!! writes 1/2 turn the monitor off",
    0x39: "LED (0 = on, 1 = off)", 0xB2: "Sub-pixel layout (read-only)", 0xB6: "Display technology (read-only)", 0x13: "Backlight? (read-only)", 0xFE: "Firmware", 0x30: "Refresh Rate Enable 0/1 (99) / Low Blue Light (plain)",
    0xBA: "plain write 1 raises brightness to 100", 0xD7: "DP/USB-C link rate (VMW, untested)", 0xD9: "CEC (VMW)",
    0xDA: "Light sensor (VMW; not supported by the firmware)", 0x52: "Audio source (99) / Active control (plain)",
    0x50: "PIP/PBP", 0x51: "PIP sub source", 0x53: "PIP position", 0x54: "PIP size", 0x55: "Swap PIP pictures",
    0x56: "Reset PIP", 0x57: "Input select", 0x5F: "USB hub upstream", 0xC4: "OSD lock", 0xD5: "USB power in sleep",
    0x32: "Crosshair color", 0x34: "Crosshair shape", 0x35: "Stopwatch position", 0x38: "Magnifier position",
    0x3B: "Alignment aid", 0x3D: "Crosshair", 0x3F: "Game Time Enable", 0x4B: "Magnifier", 0x4C: "Magnifier size",
    0x4D: "Magnification", 0x63: "HawkEye Vision", 0x64: "HawkEye size", 0x65: "HawkEye position", 0x66: "HawkEye level",
    0x9B: "Hue red", 0x9C: "Hue yellow", 0x9D: "Hue green", 0x9E: "Hue cyan", 0x9F: "Hue blue",
    0xA0: "Hue magenta", 0x59: "Saturation red", 0x5A: "Saturation yellow", 0x5B: "Saturation green",
    0x5C: "Saturation cyan", 0x5D: "Saturation blue", 0x5E: "Saturation magenta",
}


class DdcError(Exception):
    def __init__(self, kind, detail=""):
        super().__init__(f"{kind}: {detail}" if detail else kind)
        self.kind = kind


class _Msg(ctypes.Structure):
    _fields_ = [("addr", ctypes.c_uint16), ("flags", ctypes.c_uint16),
                ("length", ctypes.c_uint16), ("buffer", ctypes.POINTER(ctypes.c_uint8))]


class _Xfer(ctypes.Structure):
    _fields_ = [("messages", ctypes.POINTER(_Msg)), ("count", ctypes.c_uint32)]


class Monitor:
    def __init__(self, bus=BUS, addr=ADDR):
        self.addr = addr
        self.fd = os.open(f"/dev/i2c-{bus}", os.O_RDWR | os.O_CLOEXEC)
        self._ioctl = ctypes.CDLL(None, use_errno=True).ioctl
        self._ioctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_void_p]
        self._ioctl.restype = ctypes.c_int
        self._last = 0.0

    def close(self):
        os.close(self.fd)

    # --- raw transport -------------------------------------------------------------------------
    def _xfer(self, data=None, rlen=0):
        n = len(data) if data is not None else rlen
        buf = (ctypes.c_uint8 * n)(*(data or []))
        msg = _Msg(self.addr, 0 if data is not None else 1, n, buf)
        if self._ioctl(self.fd, 0x0707, ctypes.byref(_Xfer(ctypes.pointer(msg), 1))) != 1:  # I2C_RDWR
            raise DdcError("i2c", os.strerror(ctypes.get_errno()))
        return bytes(buf)

    def _pace(self):
        wait = GAP - (time.monotonic() - self._last)
        if wait > 0:
            time.sleep(wait)
        self._last = time.monotonic()

    # --- single frames -------------------------------------------------------------------------
    def get_once(self, code):
        """One GET; returns (maximum, current) or raises DdcError(null|invalid|unsupported|i2c)."""
        self._pace()
        req = [0x51, 0x82, 0x01, code]
        chk = 0x6E
        for b in req:
            chk ^= b
        self._xfer(req + [chk])
        time.sleep(REPLY_DELAY)
        r = self._xfer(rlen=11)
        if r[:3] == b"\x6e\x80\xbe":
            raise DdcError("null")
        c = 0x50
        for b in r:
            c ^= b
        if r[:3] != b"\x6e\x88\x02" or c or r[4] != code:
            raise DdcError("invalid", r.hex(" "))
        if r[3] != 0:
            raise DdcError("unsupported", f"status {r[3]:02x}")
        return int.from_bytes(r[6:8], "big"), int.from_bytes(r[8:10], "big")

    def set_once(self, code, value):
        self._pace()
        hi, lo = value >> 8, value & 0xFF
        self._xfer([0x51, 0x84, 0x03, code, hi, lo, 0x6E ^ 0x51 ^ 0x84 ^ 0x03 ^ code ^ hi ^ lo])

    # --- robust operations ---------------------------------------------------------------------
    def settle(self, tries=30):
        """Plain GET 0xDF until two valid replies in a row (leaves a wedged stream or an outage).
        DF (VCP version) is used because codes 00-1E right after GET 99 -> GET 22 start a table stream."""
        good = 0
        for _ in range(tries):
            try:
                self.get_once(0xDF)
                good += 1
                if good == 2:
                    return True
            except DdcError:
                good = 0
        return False

    def get(self, code, via99=None, tries=6):
        """Reads one register and returns (maximum, current, register) with register "99" or "plain".
        via99=True/False forces the register. via99=None (auto): codes 00-1E plain; others after GET 99,
        and if that register is an empty placeholder (00ff/0000) or unreadable, the plain register instead.
        'unsupported' is not trusted on the first reply (it is sometimes transient), so it is retried too."""
        if via99 is None:
            if code in TABLE_SELECTORS or code in PLAIN_CODES:
                return self.get(code, False, tries)
            placeholder = None
            try:
                r = self.get(code, True, tries)
                if r[:2] != (0x00FF, 0):
                    return r
                placeholder = r
            except DdcError:
                pass
            try:
                return self.get(code, False, tries)
            except DdcError:
                if placeholder:
                    return placeholder
                raise
        if via99 and code in TABLE_SELECTORS:
            raise ValueError("codes 00-1E after GET 99 start a profile table stream")
        last = None
        for _ in range(tries):
            try:
                if via99:
                    if self.get_once(0x99) != (0xFAFA, 0xFAFA):
                        raise DdcError("invalid", "GET 99 != fafa/fafa")
                mx, cur = self.get_once(code)
                if via99 and code == 0x22:
                    # GET 99 -> GET 22 -> GET 00..1E would read a profile table: close the sequence.
                    try:
                        self.get_once(0xDF)
                    except DdcError:
                        pass
                return mx, cur, "99" if via99 else "plain"
            except DdcError as e:
                last = e
                time.sleep(0.2 if e.kind == "unsupported" else RETRY_WAIT)
                if e.kind != "unsupported":
                    self.settle()
        raise last

    def set(self, code, value, via99=True):
        """Writes like VMW: SET 99 = value, then SET code = value. No retries (as VMW)."""
        self.settle()
        if via99:
            self.set_once(0x99, value)
            time.sleep(WRITE_GAP)
        self.set_once(code, value)


def describe(maximum, current, code=None):
    """Human readable value. Manufacturer registers answer maximum = 0xFF<max>."""
    if code == 0xDC:  # Display Application: 00ff/0000 is a real value here (0 = Standard)
        return f"{current} (0 Standard, 3 Movie, 5 Games)"
    if code == 0x52 and maximum == 0x00FF:  # plain register: code of the last setting changed in the OSD
        if current == 0:
            return "no change recorded (cleared)"
        return f"last changed in the OSD: code 0x{current & 0xFF:02X} ({NAMES.get(current & 0xFF, 'unknown')})"
    if maximum == 0x00FF and current == 0:
        return "placeholder (no data)"
    if code == 0xAC:  # horizontal frequency: 24 bits = ml, sh, sl
        return f"{((maximum & 0xFF) << 16) | current} Hz"
    if code == 0xAE:
        return f"{current / 100:.2f} Hz"
    if maximum >> 8 == 0xFF and maximum != 0xFFFF:
        if current > 0xFF:  # e.g. 51/52/57: ff05/69xx -> low byte is the value
            return f"{current & 0xFF} (high byte 0x{current >> 8:02x}, max {maximum & 0xFF})"
        return f"{current} (max {maximum & 0xFF})"
    return f"{current} (max {maximum})"
