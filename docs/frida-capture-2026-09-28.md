# Analiza Fridy — View More Widget, 2026-09-28

Źródło: `/home/misiek/Pobrane/test.txt`, 662 linie JSON. SHA-256:
`a35f80284f01b009b8974fd2e11a5d654f89b0fe956222ee233e1521cf6e185d`.

Czas wywołań: 20:51:30.276–20:54:30.731 UTC (ostatni powrót po ostatnim wywołaniu). Program x64, PID 12720.
Przechwycone funkcje pochodzą z `dxva2.dll`. Wszystkie 278 wywołań ma sparowany wynik. Brak `hook-error`.
Pełna chronologia: [CSV](frida-capture-2026-09-28.csv). Numery linii poniżej odnoszą się do pliku źródłowego.

## Wyniki API

| Operacja | Liczba | Wynik |
| --- | ---: | --- |
| SetVCPFeature | 162 | Wszystkie TRUE; 81 par zapisów |
| GetVCPFeatureAndVCPFeatureReply | 116 | Wszystkie FALSE, błąd 0xC0262589 |
| SaveCurrentMonitorSettings | 0 | Funkcja przechwycona, brak wywołań w tym zapisie |

`-1071241847`, zapisane jako 32-bitowa liczba bez znaku, to `0xC0262589`: `ERROR_GRAPHICS_DDCCI_INVALID_MESSAGE_COMMAND`. Oznacza nieprawidłowe pole komendy komunikatu DDC/CI według Windows. Nie jest to kod błędu checksum (`0xC026258B`) ani nieobsługiwanej funkcji VCP (`0xC0262584`). [Definicje Microsoft](https://learn.microsoft.com/en-us/windows/win32/com/com-error-codes-5).

Log API nie zawiera surowych ramek, więc nie ustala, jakie bajty spowodowały błąd. Nie rozstrzyga przyczyny po stronie monitora, adaptera, sterownika lub aplikacji. `TRUE` po zapisie nie jest niezależnym odczytem ustawionej wartości. W tym przechwyceniu nie ma ani jednego poprawnego odczytu wartości.

Wszystkie wywołania mają `monitor="0x0"`. To wymaga porównania z wynikiem enumeracji `GetPhysicalMonitorsFromHMONITOR`/`GetPhysicalMonitorsFromIDirect3DDevice9`; bieżący skrypt ich nie przechwytuje. Sam zapis uchwytu jako zera nie pozwala tutaj orzec, który monitor obsługiwano lub że uchwyt jest błędny.

## Sekwencja producenta

We wszystkich 81 parach zachodzi:
```text
SetVCPFeature(monitor, 0x99, wartość)
SetVCPFeature(monitor, kod_docelowy, ta_sama_wartość)
```

Nie ma wywołań ustawiających VCP `0x03`. Nie ma też zapisu `0x99 = 0x00F6`. Przykład: linie 475–478 to `0x99=4`, następnie `0x3A=4`. Zapis do 0x99 poprzedza ustawienie, a jego wartość zależy od docelowego ustawienia. Log pokazuje zachowanie programu, lecz nie dowodzi, czy firmware potrzebuje pierwszego zapisu ani co on robi wewnętrznie.

Drugi zapis zaczyna się w tej samej milisekundzie, w której wraca pierwszy. Wywołanie pierwszego zapisu trwa 51–68 ms (mediana 60 ms). Nie należy interpretować tego jako dowodu na dodatkowy sleep 60 ms w aplikacji.

We wszystkich 58 parach odczytów program wykonuje:
```text
GetVCPFeatureAndVCPFeatureReply(monitor, 0x99, ...)
GetVCPFeatureAndVCPFeatureReply(monitor, kod_docelowy, ...)
```

Kody docelowe to `0x22` (18 prób), `0x47` (20 prób), `0x48` (20 prób). Nie ma odczytu VCP `0x01`. Wyniki obu odczytów każdej pary są błędami. W połączeniu z wcześniejszymi logami producenta wyjaśnia to `03 99 "22"` jako operację zapisu z etapami 99 i 22, a `01 99 "22"` jako odczyt z etapami 99 i 22. 03/01 odpowiadają rodzajowi operacji, nie dodatkowym kodom VCP.

Przykład odtworzenia pary zapisów w ddcutil (nie wykonano podczas analizy):
```sh
ddcutil --bus 14 --noverify setvcp 99 4 3A 4
```

## Full Game / rozmiar ekranu

| Czas UTC | Linie SET 99 / SET 3A | Wartość DEC | Znaczenie |
| --- | --- | ---: | --- |
| 20:53:12.397 / 20:53:12.460 | 467 / 469 | 0 | Wide według wcześniejszych prób użytkownika |
| 20:53:12.926 / 20:53:12.992 | 471 / 473 | 1 | 25 cali według wcześniejszych prób użytkownika |
| 20:53:21.003 / 20:53:21.060 | 475 / 477 | 4 | Kandydat sPX, jeżeli kliknięcia były Wide → 25 cali → sPX |

Wartość 4 jest potwierdzona jako argument programu. Nazwa sPX jest wnioskiem z kolejności kliknięć, nie polem tego logu. Nie można przypisać jej konkretnej rozdzielczości OSD. W przechwyceniu nie występuje zapis 3A=2 ani odczyt 3A.

## Profile

Kolejność wartości wysłanych do `0x22`: `1, 2, 1, 3, 4, 5, 6, 1, 1` (DEC). Każdą poprzedza zapis tej samej wartości do 0x99. Po niektórych zapisach 1, 3 i 5 występują próby odczytu 47 i 48. Nie ma nazw klikniętych profili ani udanych odczytów; nie da się przypisać pełnej tabeli Default/Custom. Obecna tabela w p275mv_plus.md wymaga porównania z celowo opisanymi kliknięciami, zwłaszcza wobec występowania wartości 1.

W pliku nie ma udanego transferu tabeli profili ani danych tabeli; odczyty 47 i 48 są wywołaniami zwykłego Get VCP.

## Wszystkie docelowe kody zapisu

Nazwy poniżej pochodzą z dotychczasowego `p275mv_plus.md`; przechwycenie potwierdza argumenty wywołań, nie prawidłowość wszystkich przypisań nazw ani efekt sprzętowy. Lista wartości pokazuje tylko kliknięte wartości, nie minimum/maksimum obsługiwane przez monitor. Do każdego zapisu poniżej należy doliczyć poprzedzający SET 99 z tą samą wartością.

| VCP | Nazwa z notatek | Wartości zaobserwowane DEC (HEX) | Liczba zapisów | Linie źródłowe SET |
| --- | --- | --- | ---: | --- |
| 0x12 | Contrast | 10 (0x0A) | 1 | 317 |
| 0x14 | Color Temperature | 5 (0x05), 6 (0x06), 8 (0x08), 11 (0x0B) | 4 | 185, 189, 193, 197 |
| 0x16 | User 1 Red | 10 (0x0A) | 1 | 201 |
| 0x17 | AdobeRGB / User 1 Green | 10 (0x0A) | 1 | 205 |
| 0x18 | User 1 Blue | 10 (0x0A) | 1 | 209 |
| 0x22 | Profile (nazwy opcji nieobecne w logu) | 1 (0x01), 2 (0x02), 3 (0x03), 4 (0x04), 5 (0x05), 6 (0x06) | 9 | 113, 181, 321, 325, 393, 397, 465, 617, 661 |
| 0x26 | Gamma | 2 (0x02), 4 (0x04), 6 (0x06) | 3 | 277, 281, 285 |
| 0x30 | Refresh Rate Enable | 1 (0x01) | 1 | 481 |
| 0x31 | Refresh Rate Position | 1 (0x01) | 1 | 485 |
| 0x34 | Game Crosshair Shape | 0 (0x00) | 1 | 493 |
| 0x3A | Full Game (DM) | 0 (0x00), 1 (0x01), 4 (0x04) | 3 | 469, 473, 477 |
| 0x3D | Game Crosshair Enable | 1 (0x01) | 1 | 489 |
| 0x3E | Stopwatch Enable | 1 (0x01) | 1 | 497 |
| 0x3F | Game Time Enable | 1 (0x01) | 1 | 501 |
| 0x40 | Color Enhance | 1 (0x01) | 2 | 305, 537 |
| 0x41 | CR Enhance | 1 (0x01) | 2 | 309, 541 |
| 0x42 | Shadow Balance | 10 (0x0A), 20 (0x14) | 3 | 269, 545, 549 |
| 0x43 | Game Rush | 1 (0x01) | 1 | 513 |
| 0x44 | Super Resolution | 1 (0x01), 2 (0x02) | 3 | 301, 553, 557 |
| 0x45 | Night Vision | 1 (0x01), 2 (0x02) | 2 | 289, 521 |
| 0x46 | Halo Control | 10 (0x0A), 20 (0x14) | 2 | 561, 565 |
| 0x47 | Local Dimming | 3 (0x03) | 1 | 517 |
| 0x49 | Dynamic OD | 1 (0x01), 2 (0x02) | 2 | 293, 525 |
| 0x4A | HDR | 1 (0x01) | 3 | 297, 529, 533 |
| 0x4B | Magnifier Mode Enable | 1 (0x01) | 1 | 505 |
| 0x59 | Saturation Red | 10 (0x0A) | 1 | 237 |
| 0x5A | Saturation Yellow | 10 (0x0A), 20 (0x14) | 2 | 257, 265 |
| 0x5B | Saturation Green | 10 (0x0A) | 1 | 241 |
| 0x5C | Saturation Cyan | 10 (0x0A) | 1 | 249 |
| 0x5D | Saturation Blue | 10 (0x0A) | 1 | 245 |
| 0x5E | Saturation Magenta | 10 (0x0A), 20 (0x14) | 2 | 253, 261 |
| 0x60 | Output Range | 0 (0x00), 1 (0x01), 2 (0x02) | 3 | 581, 585, 589 |
| 0x61 | Quick Boot | 0 (0x00), 1 (0x01) | 2 | 593, 597 |
| 0x63 | HawkEye Vision Enable | 1 (0x01) | 1 | 509 |
| 0x87 | Sharpness | 1 (0x01) | 1 | 313 |
| 0x9B | Hue Red | 10 (0x0A) | 1 | 213 |
| 0x9C | Hue Yellow | 10 (0x0A) | 1 | 233 |
| 0x9D | Hue Green | 10 (0x0A) | 1 | 217 |
| 0x9E | Hue Cyan | 10 (0x0A) | 1 | 225 |
| 0x9F | Hue Blue | 10 (0x0A) | 1 | 221 |
| 0xA0 | Hue Magenta | 10 (0x0A) | 1 | 229 |
| 0xC0 | OSD Show Time | 15 (0x0F) | 1 | 601 |
| 0xC1 | OSD H-Position | 10 (0x0A) | 1 | 605 |
| 0xC2 | OSD V-Position | 10 (0x0A) | 1 | 609 |
| 0xC3 | OSD Transparency | 10 (0x0A) | 1 | 613 |
| 0xD8 | Low Blue Light | 1 (0x01) | 1 | 273 |
| 0xF6 | Audio Volume | 10 (0x0A), 20 (0x14) | 2 | 573, 577 |
| 0xF7 | Audio Mute | 0 (0x00), 1 (0x01) | 2 | 109, 569 |

## Granice i dalszy pomiar

Brak zapisów E0, 39 oraz E5–E9: ten zapis nie ustala dodatkowo profili E0, tylnego LED, kolorów oświetlenia ani czujnika światła. Brak wywołania SaveCurrentMonitorSettings nie dowodzi braku automatycznego zapisu w firmware.

Do przypisania opcji potrzebne są krótkie przechwycenia z podaną kolejnością kliknięć i obserwacją efektu na ekranie. Do wyjaśnienia błędów odczytu przyda się niezależny odczyt jasności 0x10 w tym samym Windows i na tym samym połączeniu; w bieżącym zapisie nie ma GET 0x10. To rozdzieli problem ogólny od problemu z funkcjami producenta. Surowe ramki wymagają przechwycenia na niższym poziomie.
