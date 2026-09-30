# Mapowanie View More Widget do Titan Control

Celem jest powiązanie nazw opcji z pełnymi sekwencjami API i obserwowanym
efektem na monitorze. Zwykły sukces SetVCPFeature nie potwierdza efektu.
Oznaczamy osobno wartość wysłaną przez aplikację, poprawny odczyt i obserwację
użytkownika. Nazwy funkcji bierzemy z widocznego interfejsu producenta.

## Uruchomienie w Windows

Skopiuj najnowszy `frida-ddc.js` do `C:\TitanArmy\frida-ddc.js`.
Uruchom producenta, ustal aktualny PID. Zamknij poprzednią sesję Fridy,
aby nie mieć dwóch skryptów przechwytujących jednocześnie.
W PowerShell uruchom, zastępując 1234 właściwym PID:

```powershell
frida -p 1234 -l C:\TitanArmy\frida-ddc.js -o C:\TitanArmy\mapowanie-01.txt
```

Na początku powinno być `scriptVersion: 2` oraz zdarzenia `hook`.
Wszystkie dalsze `mark`, `done` i `note` wpisujesz przy znaku zachęty Fridy.
Nie uruchamiaj pliku JS dwuklikiem. Nie zmieniaj pliku JS podczas sesji:
automatyczne przeładowanie skryptu rozpoczyna nową sesję i zeruje etykiety.
Każdy plik zapisuj pod nową nazwą; nie nadpisuj wcześniejszych pomiarów.

## Kontekst sesji

Przed klikaniem wpisz notatkę z rzeczywistymi ustawieniami, np.:

```javascript
note("P275MV PLUS; polaczenie DP; profil Standard domyslny; HDR off; Adaptive Sync off; PIP off; 2560x1440 160 Hz")
```

To tylko przykład — nie zmieniaj ustawień, aby pasowały do tego tekstu.
Zanotuj też nazwę/wersję producenta, jeśli jest widoczna, oraz początkowy
stan badanej opcji. Zamknij inne aplikacje odpytywania/sterowania monitorem.

## Jeden krok = jedna opcja

1. Otwórz właściwą zakładkę i poczekaj na zakończenie jej odczytów.
2. Wpisz `mark("dokladna nazwa funkcji -> wybrana opcja", "stan przed zmiana")`.
3. Kliknij jedną opcję w programie producenta.
4. Poczekaj, aż skończą się wywołania i ponowienia. Zwykle co najmniej 3 s;
   jeśli program dalej odpytuje, poczekaj dłużej.
5. Sprawdź OSD/obraz i wpisz `done("rzeczywisty wynik")`.
6. Rozpocznij kolejny krok dopiero po zakończeniu poprzedniego.

Zmiany robione dodatkowo w OSD opisz jako osobny krok z prefiksem `OSD`,
żeby nie pomylić ich ze sterowaniem programem. Taki krok może nie mieć
żadnego wywołania API. Jeśli już wybrana opcja nie wywołuje komendy,
wybierz inną opcję w osobnym oznaczonym kroku i potem wróć.

Każdy krok może zawierać kilka zapisów, w tym 99 i kod docelowy oraz
automatyczne zmiany zależnych ustawień. Zachowujemy je wszystkie;
nie przypisujemy opcji na podstawie samego ostatniego zapisu.
Etykieta określa przedział obserwacji, nie dowodzi, że każde odpytywanie
w tym przedziale zostało wywołane kliknięciem.

## Pierwszy blok: Full Game

Wpisz etykietę, kliknij wskazaną opcję, potem opisz wynik w `done`.
Nie wklejaj wszystkich etykiet naraz.

```javascript
mark("Full Game -> Wide")
```

Po kliknięciu i obserwacji, przykładowo:

```javascript
done("OSD: Wide; obraz wypelnia ekran")
```

Następne etykiety, każda w osobnym kroku zakończonym `done`:

```javascript
mark('Full Game -> 25"')
mark("Full Game -> sPX")
mark("Full Game -> Wide (powrot)")
```

Jeżeli sPX otwiera dodatkowy wybór, samo otwarcie oznacz osobno, a każdą
rozdzielczość lub pozycję okna nazwij w kolejnym kroku. Przeklikuj tylko
opcje rzeczywiście widoczne w aplikacji.

## Kolejne bloki

| Blok | Co oznaczać |
| --- | --- |
| Profile | Każda nazwa profilu oraz osobno Default/Custom, jeśli dostępne. Zanotuj profil początkowy i końcowy w OSD. |
| Obraz | Jasność, kontrast, ostrość, gamma, temperatura barwowa. Dla każdej listy wszystkie widoczne opcje. |
| Kolory | User 1/2/3, kanały RGB, Hue i Saturation; etykieta zawiera zestaw oraz kolor. |
| Ulepszenia obrazu | Local Dimming, DyDs, HDR, Adaptive Sync, DCR, Low Blue Light, Night Vision, OD, Game Rush, Halo Control. |
| Game Aid | FPS, celownik, timery, lupa, HawkEye, Alignment Aid; osobno włączenie, kolor, rozmiar, pozycja i tryb. |
| Oświetlenie / czujnik | Tylko jeśli aplikacja udostępnia: Game Illumination, kolory, siła, tryb, front/rear, Light Sensor. |
| Ogólne / OSD | Głośność, mute, zakres RGB, Quick Boot, czas OSD, pozycja OSD, przezroczystość, język. |
| PIP/PBP i wejścia | Jeśli dostępne, osobny blok z zanotowanym wejściem, układem i źródłami. Zmiana wejścia może przerwać przechwytywanie tego monitora. |

Dla przełącznika wykonaj Off → On → Off, każdy stan osobno. Dla suwaka
wybierz trzy rozróżnialne wartości w zakresie widocznym w aplikacji i wróć
do początkowej. W etykiecie podaj liczbę z UI; zapisz również wartość końcową
w `done`. Przeciąganie może wysłać wartości pośrednie — pozostają w logu.

Nie trzeba wykonywać resetów fabrycznych, aby zmapować zwykłe ustawienia.
Opcję zablokowaną opisz `done("opcja niedostepna przy ...")`; nie traktuj
braku wywołań jako dowodu, że monitor nie obsługuje funkcji.
Zależności badaj później, zmieniając jeden warunek naraz (np. HDR).

## Wprowadzenie do Titan Control

Po każdym bloku przekazujemy plik do analizy. Powstaje mapa:
etykieta UI → kolejność kodów i wartości → wynik API → efekt na monitorze.
Z niej aktualizujemy `p275mv_plus.md`, definicje kodów, mapowania opcji
i zależności UI w Titan Control. Potwierdzamy działanie na Linuksie dla
danego ustawienia; nie stosujemy automatycznie sekwencji 99 do wszystkich
kodów na podstawie samego logu Windows. Błędy odczytu pozostają jawne.

Źródła mechanizmu konsoli i przechwytywania:
[Frida CLI](https://frida.re/docs/frida-cli/) oraz
[Frida JavaScript API](https://frida.re/docs/javascript-api/).
