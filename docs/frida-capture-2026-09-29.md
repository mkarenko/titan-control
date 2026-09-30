# Analiza oznaczonego przechwycenia Fridy — 2026-09-29

Źródło: `/home/misiek/Pobrane/ddc-output.txt`, 1582 linie JSON, 58 zamkniętych kroków, jedna sesja x64.
SHA-256: `09af7ab46828c84ade3fce9a49451d59ad61ed949561637f605c9d22f11c3147`.

Pełna chronologia z rozbiciem odczytów na bajty: [CSV](frida-capture-2026-09-29.csv). Numery linii odnoszą się do oryginalnego pliku.

## Wyniki i ograniczenia

298 zapisów SetVCPFeature: wszystkie TRUE. 382 odczyty: 357 TRUE, 25 FALSE. Wszystkie 25 nieudanych odczytów ma błąd 0xC0262589; 22 dotyczą 47, a 3 FE. Brak wywołań SaveCurrentMonitorSettings i brak hook-error.

103 udane odczyty pozostawiają w lastError wcześniejszy kod -1071241847. Nie są błędami: o powodzeniu tego API decyduje TRUE/FALSE; GetLastError odczytujemy jako przyczynę przy FALSE. [Microsoft: GetVCPFeatureAndVCPFeatureReply](https://learn.microsoft.com/en-us/windows/win32/api/lowlevelmonitorconfigurationapi/nf-lowlevelmonitorconfigurationapi-getvcpfeatureandvcpfeaturereply), [GetLastError](https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror).

Wszystkie obserwacje done mają tekst „rzeczywisty efekt”, bez opisu stanu OSD/obrazu. Potwierdzamy etykiety i argumenty programu oraz udane odczyty, lecz nie uznajemy tego tekstu za niezależne potwierdzenie fizycznego efektu. Frida nie zapisuje pełnych ramek z przewodu ani checksum.

## Zapisy i Full Game

Wszystkie 149 par mają postać SET 99=wartość, następnie SET kod_docelowy=ta_sama_wartość. Nie ma SET 03 ani SET 99=246. Znaczenie 99 wewnątrz firmware pozostaje nieustalone.

| Etykieta | VCP | Wartość DEC / HEX | Krok / linia docelowego SET |
| --- | --- | --- | --- |
| Full Game → 25 | 3A | 1 / 01 | 1 / 110 |
| Full Game → Wide | 3A | 0 / 00 | 2 / 116 |
| Full Game → sPX | 3A | 4 / 04 | 3 / 122 |

Nazwy są teraz obecne w oznaczeniu kroków; sPX=4 nie wymaga już wnioskowania z samej kolejności kliknięć. Nadal nie ustala to rozdzielczości sPX ani pozostałych rozmiarów w OSD. Nie ma GET 3A.

## Profile — argumenty SET 22

| Profil | Default DEC (HEX) | Custom DEC (HEX) | Kroki Default / Custom |
| --- | --- | --- | --- |
| Standard | 1 (0x01) | 2 (0x02) | 5 / 4 |
| RTS/RPG | 3 (0x03) | 4 (0x04) | 10 / 11 |
| FPS | 5 (0x05) | 6 (0x06) | 12 / 13 |
| MOBA | 7 (0x07) | 8 (0x08) | 14 / 15 |
| Movie | 9 (0x09) | 10 (0x0A) | 16 / 17 |
| Reading | 11 (0x0B) | 12 (0x0C) | 21 / 22 |
| Night | 13 (0x0D) | 14 (0x0E) | 23 / 24 |
| Eye Care | 15 (0x0F) | 16 (0x10) | 25 / 26 |
| MacView | 17 (0x11) | 18 (0x12) | 27 / 28 |
| E-book | 19 (0x13) | 20 (0x14) | 29 / 30 |
| sRGB | 21 (0x15) | 22 (0x16) | 31 / 32 |
| AdobeRGB | 23 (0x17) | 24 (0x18) | 33 / 34 |
| DCI-P3 | 25 (0x19) | 26 (0x1A) | 35 / 36 |
| DyDs/ULL FPS | 27 (0x1B) | 28 (0x1C) | 6 / 7 |
| DyDs/LD | 29 (0x1D) | 30 (0x1E) | 8 / 9 |

Poprzednia mapa Standard 2/3, RTS 4/5 i FPS 6/brak Custom była sprzeczna z tym przechwyceniem. Dalsze pary od MOBA do DyDs/LD są zgodne. Maksimum odczytu 22 wynosi 38 (0x26), ale nie potwierdza znaczenia ani obsługi wszystkich wartości do 38.

Kroki 18 i 19 mają etykiety Reading, ale wysyłają 9/10 (jak Movie). Krok 20 ma etykietę Reading Default, ale ładuje dane 0C i wysyła 12 (Custom według spójnego powtórzenia kroków 21/22). To rozbieżności etykiety i działania programu; nie zakładamy błędu użytkownika ani nie przypisujemy Reading równocześnie do 9/10. Ostateczna mapa korzysta z powtórzenia 21/22.

## Strumień tabeli profili przez FE

Na przykład w kroku 11 (RTS/RPG Custom):
```text
GET 99 → maximum=FAFA, current=FAFA
GET 22 → maximum=0026, current=0003 (aktywny RTS Default)
GET 04 → maximum=5A32, current=0103
GET FE → maximum=0232, current=0532
GET FE → maximum=3232, current=3232
... kolejne GET FE ...
GET FE → maximum=0000, current=5101
SET 99=4
SET 22=4
```

W czasie tego odczytu pola maximum/current są kontenerem bajtów. Kandydat kolejności zgodny z istniejącą tabelą w docs i kodem Rust: MH, ML, SH, SL, czyli po dwa bajty z maximum i current. `5A32/0103` daje `[90,50,1,3]`, a `0232/0532` daje `[2,50,5,50]`. Po umieszczeniu danych od indeksu 2 pasuje to do: jasność 90, kontrast 50, ostrość 1, Color Enhance 3, CR Enhance 2, Shadow Balance 50, Color Temperature 5, pierwszy kanał RGB 50. To spójny schemat danych, a nie dowód, że każda pojedyncza odpowiedź jest zwykłą wartością funkcji FE.

Pierwsza odpowiedź danych jest na GET kodu Custom (np. 04, 06, 1C), potem standardowo jest dziewięć fragmentów FE z danymi (łącznie 40 bajtów przy danych od indeksu 2), następnie dodatkowy GET FE z 0000/5101. Zakończenie 5101 występuje w wielu strumieniach i zgadza się z wcześniejszym odczytem informacyjnym FE. Nie doliczamy go automatycznie do tabeli.

Przy trzech błędach FE program próbuje FF, a potem wraca do FE. Nie można na podstawie samego API zagwarantować, że nieudany odczyt nie przesunął kursora. Takie strumienie oznaczamy jako wymagające ponownego odczytu; nie składamy ich bezwarunkowo jako kompletnej tabeli.

Podsumowanie kandydatów danych w krokach Custom poniżej. Wartości są dekodowane według dotychczasowego schematu pól, nie niezależnie zmierzone na ekranie.

| Krok | Etykieta | Kod pierwszego GET danych | Liczba udanych fragmentów danych przed końcem | Błędy FE / fallback FF | Pierwsze 8 bajtów danych |
| --- | --- | --- | ---: | --- | --- |
| 7 | Profile -> DyDs/ULL FPS - Cus | 0x1c | 10 | brak / nie | 64 32 00 05 00 32 06 32 |
| 9 | Profile -> DyDs/LD Cus | 0x1e | 11 | 288 / tak | 64 32 00 05 00 32 06 32 |
| 11 | Profile -> RTS/RPG Cus | 0x04 | 10 | brak / nie | 5a 32 01 03 02 32 05 32 |
| 13 | Profile -> FPS Cus | 0x06 | 10 | brak / nie | 5f 32 02 02 02 32 05 32 |
| 15 | Profile -> MOBA Cus | 0x08 | 11 | 488 / tak | 5f 32 01 03 03 3c 05 32 |
| 17 | Profile -> Movie Cus | 0x0a | 10 | brak / nie | 64 3c 00 00 02 32 05 32 |
| 19 | Profile -> Reading Cus | 0x0a | 10 | brak / nie | 64 3c 00 00 02 32 05 32 |
| 20 | Profile -> Reading Def | 0x0c | 10 | brak / nie | 32 32 00 00 00 32 06 32 |
| 22 | Profile -> Reading Cus | 0x0c | 10 | brak / nie | 32 32 00 00 00 32 06 32 |
| 24 | Profile -> Night Cus | 0x0e | 10 | brak / nie | 1e 32 00 00 00 32 06 32 |
| 26 | Profile -> Eye Care Cus | 0x10 | 10 | brak / nie | 1e 32 00 00 00 32 06 32 |
| 28 | Profile -> MacView Cus | 0x12 | 10 | brak / nie | 5a 32 00 01 01 32 06 32 |
| 30 | Profile -> E-Book Cus | 0x14 | 10 | brak / nie | 46 32 00 00 01 32 05 32 |
| 32 | Profile -> sRGB Cus | 0x16 | 10 | brak / nie | 5a 32 00 00 00 32 05 32 |
| 34 | Profile -> Adobe Cus | 0x18 | 10 | brak / nie | 5a 32 00 00 00 32 05 32 |
| 36 | Profile -> DCI-P3 Cus | 0x1a | 10 | brak / nie | 5a 32 00 00 00 32 05 32 |

## Wszystkie oznaczone kroki

Listy SET poniżej pomijają tylko powtarzający się prefiks 99 z tą samą wartością. Każdy krok ma odpowiadający step-end. Wartości są DEC, kody HEX. Jeżeli krok zawiera kilka opcji, przypisanie wartości do nazw wymaga założenia, że klikano w kolejności wpisanej w etykiecie.

| Krok | Etykieta | Docelowe SET: kod=wartość | GET: udane / wszystkie | Linie step-start / step-end |
| --- | --- | --- | --- | --- |
| 1 | Full Game -> 25 | 3A=1 | 0 / 0 | 107 / 112 |
| 2 | Full Game -> Wide | 3A=0 | 0 / 0 | 113 / 118 |
| 3 | Full Game -> sPX | 3A=4 | 0 / 0 | 119 / 124 |
| 4 | Profile -> Standard/Custom | 22=2 | 0 / 0 | 129 / 134 |
| 5 | Profile -> Standard/Default | 22=1 | 0 / 0 | 135 / 140 |
| 6 | Profile -> DyDs/ULL FPS - Def | 22=27 | 5 / 6 | 205 / 222 |
| 7 | Profile -> DyDs/ULL FPS - Cus | 22=28 | 13 / 13 | 223 / 254 |
| 8 | Profile -> DyDs/LD Def | 22=29 | 5 / 6 | 255 / 272 |
| 9 | Profile -> DyDs/LD Cus | 22=30 | 13 / 14 | 273 / 306 |
| 10 | Profile -> RTS/RPG Def | 22=3 | 5 / 6 | 357 / 374 |
| 11 | Profile -> RTS/RPG Cus | 22=4 | 13 / 13 | 375 / 406 |
| 12 | Profile -> FPS Def | 22=5 | 5 / 6 | 407 / 424 |
| 13 | Profile -> FPS Cus | 22=6 | 13 / 13 | 425 / 456 |
| 14 | Profile -> MOBA Def | 22=7 | 5 / 6 | 457 / 474 |
| 15 | Profile -> MOBA Cus | 22=8 | 13 / 14 | 475 / 508 |
| 16 | Profile -> Movie Def | 22=9 | 5 / 6 | 509 / 526 |
| 17 | Profile -> Movie Cus | 22=10 | 13 / 13 | 527 / 558 |
| 18 | Profile -> Reading Def | 22=9 | 0 / 0 | 559 / 564 |
| 19 | Profile -> Reading Cus | 22=10 | 13 / 13 | 565 / 596 |
| 20 | Profile -> Reading Def | 22=12 | 18 / 19 | 597 / 640 |
| 21 | Profile -> Reading Def | 22=11 | 5 / 6 | 693 / 710 |
| 22 | Profile -> Reading Cus | 22=12 | 13 / 13 | 711 / 742 |
| 23 | Profile -> Night Def | 22=13 | 5 / 6 | 743 / 760 |
| 24 | Profile -> Night Cus | 22=14 | 13 / 13 | 761 / 792 |
| 25 | Profile -> Eye Care Def | 22=15 | 5 / 6 | 793 / 810 |
| 26 | Profile -> Eye Care Cus | 22=16 | 13 / 13 | 811 / 842 |
| 27 | Profile -> MacView Def | 22=17 | 5 / 6 | 843 / 860 |
| 28 | Profile -> MacView Cus | 22=18 | 13 / 13 | 861 / 892 |
| 29 | Profile -> E-Book Def | 22=19 | 5 / 6 | 893 / 910 |
| 30 | Profile -> E-Book Cus | 22=20 | 13 / 13 | 911 / 942 |
| 31 | Profile -> sRGB Def | 22=21 | 5 / 6 | 943 / 960 |
| 32 | Profile -> sRGB Cus | 22=22 | 13 / 13 | 961 / 992 |
| 33 | Profile -> Adobe Def | 22=23 | 5 / 6 | 993 / 1010 |
| 34 | Profile -> Adobe Cus | 22=24 | 13 / 13 | 1011 / 1042 |
| 35 | Profile -> DCI-P3 Def | 22=25 | 5 / 6 | 1043 / 1060 |
| 36 | Profile -> DCI-P3 Cus | 22=26 | 13 / 13 | 1061 / 1092 |
| 37 | StandardCusSet -> ColorTemperature -> Natural | 14=6 | 0 / 0 | 1135 / 1140 |
| 38 | StandardCusSet -> ColorTemperature -> Warm | 14=5 | 0 / 0 | 1141 / 1146 |
| 39 | StandardCusSet -> ColorTemperature -> User1 -> R | 14=11, 16=40 | 0 / 0 | 1147 / 1156 |
| 40 | StandardCusSet -> ColorTemperature -> User1 -> G | 17=40 | 0 / 0 | 1157 / 1162 |
| 41 | StandardCusSet -> ColorTemperature -> User1 -> B | 18=40 | 0 / 0 | 1163 / 1168 |
| 42 | StandardCusSet -> Brightness | 10=50 | 0 / 0 | 1173 / 1178 |
| 43 | StandardCusSet -> Contrast | 12=60 | 0 / 0 | 1179 / 1184 |
| 44 | StandardCusSet -> Sharpness | 87=1 | 0 / 0 | 1185 / 1190 |
| 45 | StandardCusSet -> CR Enchance | 41=1 | 0 / 0 | 1191 / 1196 |
| 46 | StandardCusSet -> Color Enchance | 40=1 | 0 / 0 | 1197 / 1202 |
| 47 | StandardCusSet -> Super Resolution | 44=1 | 0 / 0 | 1203 / 1208 |
| 48 | StandardCusSet -> HDR | 4A=0 | 0 / 0 | 1209 / 1214 |
| 49 | StandardCusSet -> HDR -> Auto | 4A=1 | 0 / 0 | 1215 / 1220 |
| 50 | StandardCusSet -> HDR -> Off | 4A=0 | 0 / 0 | 1221 / 1226 |
| 51 | StandardCusSet -> Gamma -> 1.8, 2.0, 2.2, 2.4, 2.6, S.curve | 26=2, 26=4, 26=6, 26=8, 26=10, 26=12 | 0 / 0 | 1227 / 1252 |
| 52 | StandardCusSet -> Night Vision -> Off, Lvl1, Lvl2, Auto-lvl1, Auto-lvl2 | 45=0, 45=1, 45=2, 45=3, 45=4 | 0 / 0 | 1265 / 1286 |
| 53 | StandardCusSet -> Dynamic OD -> Off, Lvl1, Lvl2, Lvl3, Topspeed | 49=0, 49=1, 49=2, 49=3, 49=4 | 0 / 0 | 1291 / 1312 |
| 54 | StandardCusSet -> Low Blue Light | D8=1 | 0 / 0 | 1313 / 1318 |
| 55 | StandardCusSet -> Shadow Balance | 42=40, 42=50, 42=40 | 0 / 0 | 1319 / 1332 |
| 56 | StandardCusSet -> Shadow Balance | 42=30 | 0 / 0 | 1333 / 1338 |
| 57 | StandardCusSet -> Hue -> R, G, B, C, M, Y | 9B=60, 9D=60, 9F=60, 9E=60, A0=60, 9C=60 | 0 / 0 | 1447 / 1472 |
| 58 | StandardCusSet -> Color Saturation -> R, G, B, C, M, Y | 59=60, 5B=60, 5D=60, 5C=60, 5E=60, 5A=60 | 0 / 0 | 1533 / 1558 |

## Wartości list i kolorów

- Gamma 26: 2,4,6,8,10,12 dla 1.8,2.0,2.2,2.4,2.6,S-curve, przy kolejności zgodnej z etykietą kroku 51.
- Night Vision 45: 0,1,2,3,4 dla Off,Lvl1,Lvl2,Auto-Lvl1,Auto-Lvl2, przy kolejności z kroku 52.
- Dynamic OD 49: 0,1,2,3,4 dla Off,Lvl1,Lvl2,Lvl3,Topspeed, przy kolejności z kroku 53.
- Hue (R,G,B,C,M,Y): 9B,9D,9F,9E,A0,9C — krok 57, wszystkie wysłane wartości 60.
- Saturation (R,G,B,C,M,Y): 59,5B,5D,5C,5E,5A — krok 58, wszystkie wysłane wartości 60.
- User1 RGB: 16,17,18; każda wartość 40. User1 wybierane przez 14=11; Natural przez 14=6; Warm przez 14=5.
- HDR 4A: Auto=1, Off=0. Low Blue Light D8=1. Pozostałe suwaki mają konkretne próbki z tabeli kroków, a nie kompletne zakresy.

## Dane poza krokami i dalsza integracja

Wywołania bez stepId również są zachowane w CSV. Nie przypisujemy ich automatycznie do poprzedniej etykiety. Nie występują komendy LED E5–E9, czujnika światła ani odczyt E0. Nie ma uchwyconego zapisu E2=0 przed wyborem DyDs; nie oznacza to, że firmware nie wyłącza Adaptive Sync wewnętrznie.

W istniejącym Titan Control wymagają uwagi mapy pierwszych trzech profili, sPX=4 i sekwencja zapisów (program Rust nadal wysyła wartość docelową, potem stałe 99=00F6). Schemat składania tabeli od indeksu 2 jest zgodny z obserwowanymi strumieniami. Nie zmieniono kodu aplikacji w ramach analizy tego pliku; przed zmianą transportu potrzebne jest porównanie zapisów i odczytów na Linuksie.
