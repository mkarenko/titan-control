# Monitor DDC codes with values

## General

| Setting          | VCP  | Values                                 | Default |
| ---------------- | ---- | -------------------------------------- | ------- |
| Output Range     | 0x60 | rgb auto \| rgb limit \| rgb full      | auto    |
| Quick Boot       | 0x61 | enabled / disabled                     | off     |
| Audio Mute       | 0xf7 | disabled / enabled                     | enabled |
| Audio Volume     | 0xf6 | 0-100                                  | 50      |
| --               | --   | --                                     | --      |
| OSD Show Time    | 0xc0 | 5-60                                   | 10      |
| OSD H-Position   | 0xc1 | 0-100                                  | 50      |
| OSD V-Position   | 0xc2 | 0-100                                  | 50      |
| OSD Transparency | 0xc3 | 0-100                                  | 0       |
| --               | --   | --                                     | --      |
| Power LED        | 0xc5 | 01=disabled, 02=lvl1, 03=lvl2, 04=lvl3 | lvl2    |
| Power LED        | 0xc5 | 01=disabled, 02=lvl1, 03=lvl2, 04=lvl3 | lvl2    |
| Power Saving     | 0xc6 | 0=off, 1=lvl1, 2=lvl2, 3=lvl3          | off     |
| Read LED         | 0x39 | 0=on, 1=off                            | on      |

| Setting             | VCP  |
| ------------------- | ---- |
| Reset Factory       | 0xc7 |
| Reset Colors        | 0x08 |
| Reset Brightness/CR | 0x05 |
| Change Language     | 0xcc |

## Profiles

**VCP:** `0x22`

Oznaczone przechwycenie Fridą z 2026-09-29 (`ddc-output.txt`) przypisuje
wartości `0x22` do nazw Default/Custom. Poprawiono wcześniejsze przypisania
Standard, RTS i FPS na podstawie kroków 4/5, 10/11 i 12/13.
Tabela opisuje argumenty aplikacji producenta; obserwacje `done` nie zawierają
szczegółów efektu na ekranie. Reading ma spójne powtórzenie w krokach 21/22,
ale wcześniejsze etykiety 18–20 są rozbieżne z wysłanymi wartościami.

| VCP  | Nazwa                 | Wartości                                   |
| ---- | --------------------- | ------------------------------------------ |
| 0xe0 | Zmiana profilu obrazu | 0–14 dziesiętnie (`0x00–0x0E`), 15 profili |

> ⚠️ NIGDY NIE PRÓBUJ USTAWIAĆ `0x22 = 0`

Próby na Linuksie po przełączeniu w OSD: dla FPS Custom i następnie FPS
Default odczyt `E0` pozostawał `4`, a `22` pozostawał `6`. W kolejnych
odczytach readera przy Standard występowały wartości `1` i `2`, a po
wyborze FPS Default wartości `5` i `6` (zrekonstruowane ze starego dekodera).
Stary reader błędnie nazywał `2` Standard default, `5` RTS/RPG custom,
`6` FPS default. Poprawiony reader pokazuje surową wartość i nazwę z mapy
zapisów WMW. Nie traktować samej nazwy jako pewnego stanu Default/Custom
z OSD. Przyczyną zmiennych odczytów nie można obarczyć wyłącznie dekodera:
różne etykiety starego readera wskazują też na różne wartości odpowiedzi.

| Profile        | Default | Custom |
| -------------- | ------- | ------ |
| Standard       | 0x01    | 0x02   |
| RTS            | 0x03    | 0x04   |
| FPS            | 0x05    | 0x06   |
| MOBA           | 0x07    | 0x08   |
| Movie          | 0x09    | 0x0a   |
| Reading        | 0x0b    | 0x0c   |
| Night          | 0x0d    | 0x0e   |
| Eye Care       | 0x0f    | 0x10   |
| MacView        | 0x11    | 0x12   |
| E-book         | 0x13    | 0x14   |
| sRGB           | 0x15    | 0x16   |
| AdobeRGB       | 0x17    | 0x18   |
| DCI-P3         | 0x19    | 0x1a   |
| DyDs / ULL FPS | 0x1b    | 0x1c   |
| DyDs / LD      | 0x1d    | 0x1e   |

<br>

## Color Profiles

| Nazwa              | VCP  |              Zakres / Wartości              | Domyślnie |
| ------------------ | :--: | :-----------------------------------------: | --------: |
| Color Temperature  | 0x14 | Warm, Cold, Natural, User 1, User 2, User 3 |      Warm |
| User 1 Red         | 0x16 |                    0-100                    |        50 |
| User 1 Green       | 0x17 |                    0-100                    |        50 |
| User 1 Blue        | 0x18 |                    0-100                    |        50 |
| User 2 Red         | 0x19 |                    0-100                    |        50 |
| User 2 Green       | 0x1a |                    0-100                    |        50 |
| User 2 Blue        | 0x1b |                    0-100                    |        50 |
| User 3 Red         | 0x1c |                    0-100                    |        50 |
| User 3 Green       | 0x1d |                    0-100                    |        50 |
| User 3 Blue        | 0x1e |                    0-100                    |        50 |
| --                 |  --  |                     --                      |        -- |
| Hue Red            | 0x9b |                    0-100                    |        50 |
| Hue Green          | 0x9d |                    0-100                    |        50 |
| Hue Blue           | 0x9f |                    0-100                    |        50 |
| Hue Cyan           | 0x9e |                    0-100                    |        50 |
| Hue Magenta        | 0xa0 |                    0-100                    |        50 |
| Hue Yellow         | 0x9c |                    0-100                    |        50 |
| --                 |  --  |                     --                      |        -- |
| Saturation Red     | 0x59 |                    0-100                    |        50 |
| Saturation Green   | 0x5b |                    0-100                    |        50 |
| Saturation Blue    | 0x5d |                    0-100                    |        50 |
| Saturation Cyan    | 0x5c |                    0-100                    |        50 |
| Saturation Magenta | 0x5e |                    0-100                    |        50 |
| Saturation Yellow  | 0x5a |                    0-100                    |        50 |

<br>

## Picture Enhancer

| Nazwa            | VCP         | Zakres / Wartości                        | Domyślnie | Uwagi                 |
| ---------------- | ----------- | ---------------------------------------- | --------: | --------------------- |
| Brightness       | 0x10        | 0-100                                    |        25 |
| Contrast         | 0x12        | 0-100                                    |        50 |
| Sharpness        | 0x87        | 0-5                                      |         0 |
| Color Enhance    | 0x40        | 0-10                                     |         0 |
| CR Enhance       | 0x41        | 0-5                                      |         0 |
| Shadow Balance   | 0x42        | 0-100                                    |        50 |
| Super Resolution | 0x44        | 0-5                                      |         0 |
| Halo Control     | 0x46        | 0-100                                    |        25 |
| Low Blue Light   | 0xd8 (0xe1) | 0-4                                      |         0 |
| DCR              | 0xe1        | Off, On                                  |       Off |
| Adaptive-Sync    | 0xe2        | Off, On                                  |       Off | e4 tez wlacza/wylacza |
| HDR              | 0x4a        | Off, Auto, Game, Movie                   |       Off |
| Gamma            | 0x26        | 1.8, 2.0, 2.2, 2.4, 2.6, S-Curve         |       2.2 |
| Local Dimming    | 0x47        | Off, Low, Smooth, Medium, High           |      High |
| DyDs             | 0x48        | Off, Low, Medium, High, ULL1, ULL2, ULL3 |       Off |
| Night Vision     | 0x45        | Off, Lvl1, Lvl2, Auto-Lvl1, Auto-Lvl2    |       Off |
| Dynamic OD       | 0x49        | Off, Lvl1, Lvl2, Lvl3, Topspeed          |       Off |
| Game Rush        | 0x43        | Off, On                                  |       Off |

<br>

## Game Aid

| VCP  | Nazwa                   | Zakres / Wartości                                       |
| ---- | ----------------------- | ------------------------------------------------------- |
| 0x3a | Full Game (DM)          | Wide, 25", sPX                                          |
| 0x30 | Refresh Rate Enable     | Off, On                                                 |
| 0x31 | Refresh Rate Position   | Top Right, Top Left, Bottom Right, Bottom Left          |
| 0x3d | Game Crosshair Enable   | Off, On                                                 |
| 0x34 | Game Crosshair Shape    | Type1, Type2, Type3, Type4, Type5, Type6                |
| 0x32 | Game Crosshair Color    | Red, Yellow, Green, Cyan, Blue, Purple, White, Auto     |
| 0x3e | Stopwatch Enable        | Off, On                                                 |
| 0x33 | Stopwatch Time          | 15, 30, 45, 60                                          |
| 0x35 | Stopwatch Position      | Top Right, Top Left, Bottom Right, Bottom Left          |
| 0x3f | Game Time Enable        | Off, On                                                 |
| 0x36 | Game Time               | 15, 30, 45, 60                                          |
| 0x37 | Game Time Position      | Top Right, Top Left, Bottom Right, Bottom Left          |
| 0x4b | Magnifier Mode Enable   | Off, On                                                 |
| 0x39 | Magnifier Night Vision  | Off, On                                                 |
| 0x4d | Magnification           | x1.5, x2, x4                                            |
| 0x4c | Magnifier Size          | Small, Medium, Large                                    |
| 0x38 | Magnifier Position      | Top Right, Top Left, Central, Bottom Right, Bottom Left |
| 0x3b | Alignment Aid           | Off, On                                                 |
| 0x63 | HawkEye Vision Enable   | Off, On                                                 |
| 0x64 | HawkEye Vision Size     | Small, Medium, Large                                    |
| 0x65 | HawkEye Vision Position | Top Right, Top Left, Central, Bottom Right, Bottom Left |
| 0x66 | HawkEye Vision Level    | Level 1, Level 2, Level 3, Level 4, Level 5             |

Oznaczone kroki 1–3 z 2026-09-29 potwierdzają argumenty aplikacji:
`0x3A=1` dla 25 cali, `0x3A=0` dla Wide i `0x3A=4` dla sPX.
Nie przypisywać `4` konkretnej rozdzielczości OSD na podstawie tego
przechwycenia. Nie ma odczytu `3A` ani szczegółowej obserwacji efektu.

<br>

## Tablica dla profili

Przechwycenie 2026-09-29 pokazuje odczyt Custom przez zwykłe Get VCP:
`GET 99`, `GET 22`, `GET kod_profilu_Custom`, następnie kolejne `GET FE`.
Pola `maximum/current` w tej sekwencji przenoszą po cztery bajty danych,
w kolejności `MH ML SH SL`, składane od indeksu 2. Na przykład RTS Custom:
`maximum=5A32, current=0103` daje `[90,50,1,3]`, zgodne ze schematem:
jasność 90, kontrast 50, ostrość 1, Color Enhancement 3.
`3232/3232` daje cztery bajty o wartości 50, a nie jedną wartość 12850.
Jest to rekonstrukcja zgodna z dotychczasowym schematem pól; Frida nie
rejestruje checksum ramek. Przy błędzie FE program próbuje FF, ale wpływ
nieudanego odczytu na kursor strumienia wymaga potwierdzenia.
Pełny przebieg: [analiza oznaczonego przechwycenia](frida-capture-2026-09-29.md).

| ID  | Nazwa                   | Zakres / Wartości                                       |
| --- | ----------------------- | ------------------------------------------------------- |
| 00  | null                    | null                                                    |
| 01  | null                    | null                                                    |
| 02  | Brightness              | 0-100                                                   |
| 03  | Contrast                | 0-100                                                   |
| 04  | Sharpness               | 0-5                                                     |
| 05  | Color Enhancement       | 0-10                                                    |
| 06  | CR Enhancement          | 0-5                                                     |
| 07  | Shadow Balance          | 0-100                                                   |
| 08  | Color Temperature       | 5 Warm, 6 Natural, 8 Cool, 11 User1, 12 User2, 13 User3 |
| 09  | Color Temp Red User 1   | 0-100                                                   |
| 10  | Color Temp Green User 1 | 0-100                                                   |
| 11  | Color Temp Blue User 1  | 0-100                                                   |
| 12  | Color Temp Red User 2   | 0-100                                                   |
| 13  | Color Temp Green User 2 | 0-100                                                   |
| 14  | Color Temp Blue User 2  | 0-100                                                   |
| 15  | Color Temp Red User 3   | 0-100                                                   |
| 16  | Color Temp Green User 3 | 0-100                                                   |
| 17  | Color Temp Blue User 3  | 0-100                                                   |
| 18  | Hue Red                 | 0-100                                                   |
| 19  | Hue Green               | 0-100                                                   |
| 20  | Hue Blue                | 0-100                                                   |
| 21  | Hue Yellow              | 0-100                                                   |
| 22  | Hue Cyan                | 0-100                                                   |
| 23  | Hue Magenta             | 0-100                                                   |
| 24  | Saturation Red          | 0-100                                                   |
| 25  | Saturation Green        | 0-100                                                   |
| 26  | Saturation Blue         | 0-100                                                   |
| 27  | Saturation Yellow       | 0-100                                                   |
| 28  | Saturation Cyan         | 0-100                                                   |
| 29  | Saturation Magenta      | 0-100                                                   |
| 30  | Low Blue Light          | 0-100 (4 poziomy co 25)                                 |
| 31  | HDR                     | 0 Off, 1 Auto, 2 Game, 3 Movie                          |
| 33  | Gamma                   | 2=1.8, 4=2.0, 6=2.2, 8=2.4, 10=2.6, 12=S-Curve          |
| 34  | Super Resolution        | 0-5                                                     |
| 35  | Night Vision            | 0 Off, 1 Lvl1, 2 Lvl2, 3 Auto-Lvl1, 4 Auto-Lvl2         |
| 36  | Dynamic OD              | 0 Off, 1 Lvl1, 2 Lvl2, 3 Lvl3, 4 Topspeed               |

# Notatki

- Local Dimming, DyDs i Halo Control są globalne i nie są powiązane z profilami.
- HawkEye Vision: zakładane wartości ON = 01/02 nie działają

# Nowe funkcje

| VCP  | Nazwa                                | Wartosci                                                                                                 | Uwagi                                            |
| ---- | ------------------------------------ | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| 0x24 | ratio z opcjami (zakladam od 0 do 4) | wide screen, 4:3, 1:1, 21:9, auto                                                                        |                                                  |
| 0x57 | wybor inputu                         | 0 to auto,1 to dp,2 to dp,3 to usb-c,5 to hdmi-1,6 to hdmi-2                                             |                                                  |
| 0x5f | wybor gdzie ma dzialac hub usb       | 0 to po usb-c, 1 to po usb-b                                                                             |                                                  |
| 0xc4 | blokada osd                          | 0 = off, 1 = on                                                                                          |                                                  |
| 0xd5 | usb power sleep                      | 0 = off, 1 = on                                                                                          |                                                  |
| 0x50 | pip                                  | 0 = off, 1 = pip mode, 2 = pbp 2 win 1:1, 3 = pbp 2win 2:1, 4 = 2win 1:2                                 | pbp = picture by picture                         |
| 0x51 | sub-signal source                    | 0 = dp, 3 = usb-c , 4 = hdmi-1 , 5 = hdmi-2                                                              |                                                  |
| 0x52 | audio source                         | 0 = auto, 3 = usb-c, 4 = hdmi-1, 5 = hdmi-2                                                              | tylko podlaczone kable                           |
| 0x53 | PIP Position                         | 0 = Top Right, 1 = Top Left, 2 = Bottom Right, 3 = Bottom Left                                           |                                                  |
| 0x54 | PIP Size                             | 0 = Small , 1 = Medium, 2 = Large                                                                        |                                                  |
| 0x55 | Swap Inputs                          | 1 = True                                                                                                 |                                                  |
| 0x56 | Reset PIP Settings                   | 1 = True                                                                                                 |                                                  |
| 0xe5 | Led - Color                          | 1 = Red, 2 = Green, 3 = Blue, 4 = Yellow , 5 = Purple, 6 = Cyan                                          |                                                  |
| 0xe6 | Led - Strength                       | 1 = Highlist, 2 = Standard, 3 = Soft                                                                     |                                                  |
| 0xe7 | Led - Mode                           | 1 = Normal, 2 = Breathe, 3 = Flicker, 4 = Plain Water, 5 = Star, 6 = Colorful Pearls, 7 = Colorful Water | 5 tylko kolor, 6 reszta ustawien zablokowana     |
| 0xe8 | Led - Front Color                    | 1 = Red, 2 = Green, 3 = Blue, 4 = Yellow , 5 = Purple, 6 = Cyan                                          | ustawienie dostepne tylko dla 7 = Colorful Water |
| 0xe9 | Led - Rear Color                     | 1 = Red, 2 = Green, 3 = Blue, 4 = Yellow , 5 = Purple, 6 = Cyan                                          | ustawienie dostepne tylko dla 7 = Colorful Water |
| 0xd6 | Tryby zasilania, nie ogarniam        |                                                                                                          |                                                  |
| 0x0c | temp kolorow                         |                                                                                                          |

dcr, ai mode, game mode, hdr, a-sync, dyds, all game-aid, dynamid od, game rush mode halo control te ustawienia sa wylaczone jesli pip wlaczony

0c, 2a-2c, 72, 61, 85, 6b/6e/6f ed, d6/7 b3, a5, a9, ca/c9, ad, fc - wygasza osd, wiec cos zmienia
62, 67 to cos z zasilaniem
eyeshield mozna zmienic na razie nie wiem gdzie

## Sekwencja producenta — przechwycenie Fridą 2026-09-28

Aktualizacja 2026-09-29: oznaczony log zawiera 149 par zapisów z identyczną
sekwencją 99 → kod docelowy. Są już udane odczyty: 357 z 382; pozostałe 25
ma błąd `0xC0262589`. Nie interpretować starego `lastError` jako niepowodzenia
przy wyniku TRUE. [Nowa analiza](frida-capture-2026-09-29.md).

Analiza `/home/misiek/Pobrane/test.txt`: wszystkie 81 par zapisów ma postać:

```text
SetVCPFeature(monitor, 0x99, wartość)
SetVCPFeature(monitor, kod_docelowy, ta_sama_wartość)
```

Wszystkie 58 par odczytów ma postać:

```text
GetVCPFeatureAndVCPFeatureReply(monitor, 0x99, ...)
GetVCPFeatureAndVCPFeatureReply(monitor, kod_docelowy, ...)
```

`03`/`01` z wcześniejszych logów odpowiadają rodzajowi operacji
(zapis/odczyt). W przechwyceniu nie ma operacji na kodach VCP `0x03`
ani `0x01`. Przed zapisem do `0x22` lub `0x3A` program zapisuje do
`0x99` tę samą wartość, którą następnie wysyła do kodu docelowego.
To nie jest stały zapis `03=99` ani późniejsze `99=00F6`.
Wewnętrzne znaczenie `0x99` i konieczność używania go przez firmware
pozostają nieustalone.

Wyniki: 162 zapisy zwracają TRUE, ale wszystkie 116 odczytów zwraca FALSE
z błędem `0xC0262589` (nieprawidłowe pole komendy komunikatu DDC/CI).
Przechwycenie nie potwierdza odczytem skuteczności żadnego ustawienia.
Pełna tabela wartości i numery linii: [analiza Fridy](frida-capture-2026-09-28.md).

## Komenda do czytania wartości — obserwacja użytkownika

```sh
ddcutil --noverify getvcp 01 99 22
```

Użytkownik wskazuje tę komendę do czytania wartości. W składni ddcutil
oznacza ona trzy osobne odczyty kodów VCP: `0x01`, `0x99` i `0x22`,
nie jedną komendę z parametrami `01 99 22`. Kod `0x22` jest powiązany
z wyborem profilu; znaczenie odczytów `0x01` i `0x99` w tym monitorze
wymaga ustalenia. Nie potwierdzono, że poprzedzające odczyty są konieczne
do odczytania `0x22`.

Opcja `--noverify` wyłącza potwierdzanie zapisów; nie zmienia tej operacji
odczytu. Późniejsze przechwycenie Fridą pokazało dwa odczyty producenta:
`0x99`, następnie `0x22`, bez odczytu VCP `0x01`. Lista kodów odpowiadająca
tym wywołaniom w ddcutil to `getvcp 99 22`; nie potwierdzono, że daje ona
poprawne odpowiedzi na tym monitorze.

Źródło składni: [ddcutil getvcp](https://www.ddcutil.com/command_getvcp/).

## Odczyty informacyjne monitora

Pola odpowiedzi łączymy jako `maximum = (mh << 8) | ml` oraz
`current = (sh << 8) | sl`. Dla kodów producenta mogą zawierać dane
niestandardowe; znaczenia nieustalone oznaczono poniżej.

| VCP  | Nazwa                                                    | Odczyt / interpretacja                                                                 | Status                                                                              |
| ---- | -------------------------------------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| 0xae | Vertical Frequency — częstotliwość pionowa / odświeżanie | W TXT odczyt 16005, czyli 160,05 Hz przy jednostce 0,01 Hz.                            | Przypisanie odświeżania; odczyt informacyjny.                                       |
| 0xf3 | Czas pracy monitora                                      | `maximum=0x0003`, `current=0xF4DF` = 62687. Jeśli jednostką są minuty: 1044 h 47 min.  | Przypisanie czasu pracy z notatek; jednostka i pole maximum wymagają potwierdzenia. |
| 0xfe | Firmware / fragmenty strumienia profilu                  | Poza strumieniem: `maximum=0x0000`, `current=0x5101` = 20737, interpretacja aplikacji 5.1.1. Podczas odczytu profilu FE przenosi kolejne cztery bajty tabeli. | Znaczenie zależy od stanu protokołu; nie dekodować każdego FE jako wersji firmware. |
| 0xff | Dane producenta — znaczenie nieustalone                  | `maximum=0x0000`, `current=0x0200` = 512.                                              | Nie interpretować automatycznie jako wersji 2.0.                                    |
| 0xdf | VCP / MCCS Version                                       | Odczyt `ddcutil`: 2.1.                                                                 | Wersja protokołu, nie firmware.                                                     |

## Tymczasowe wykluczenia z pętli setter

Na prośbę użytkownika pomijać na razie: `00`, `01`, `02`, `03`, `04`,
`05`, `06`, `07`, `09`, `DF` (wszystkie zapisane w HEX).
To lista wykluczeń z prób zapisu, nie potwierdzenie znaczenia każdego kodu.
`DF` jest dodatkowo opisany powyżej jako wersja VCP.
