# Przechwycenie wywołań DDC producenta

Wersja 2 skryptu dodaje etykiety kroków (`mark`), obserwacje (`done`),
notatki (`note`), czas wywołania i kod błędu w HEX.
Każde wywołanie i jego wynik zawierają identyfikator kroku.
Procedura mapowania całego interfejsu: [przeklikiwanie krok po kroku](frida-mapping.md).

Pierwszy etap: podgląd SetVCPFeature, GetVCPFeatureAndVCPFeatureReply
i SaveCurrentMonitorSettings. Skrypt nie zmienia argumentów ani wyników.
Rejestruje także biblioteki, aby pomóc znaleźć inną ścieżkę sterowania.

## Natywny Windows lub Windows w maszynie wirtualnej

Uruchom narzędzia wewnątrz tego samego Windows co program producenta.
Zainstaluj Python i w PowerShell:

```powershell
py -m pip install --upgrade frida-tools
frida-ps
```

Skopiuj `frida-ddc.js` z tego katalogu do Windows, np. do `C:\ddc`.
Uruchom program producenta, odszukaj PID w wyniku `frida-ps` i podłącz:

```powershell
frida -p PID -l C:\ddc\frida-ddc.js -o C:\ddc\ddc-capture.txt
```

Zastąp PID numerem procesu. Jeśli polecenia nie są dostępne, dodaj katalog
Scripts używanej instalacji Pythona do PATH. Przy odmowie dostępu uruchom
PowerShell jako administrator, zwłaszcza jeśli program producenta też jest
uruchomiony jako administrator.

Z podłączonym monitorem, w konsoli Fridy (nie w drugim PowerShell), wpisz:

```javascript
mark("Full Game -> Wide", "przed zmiana: 25 cali")
```

Kliknij Wide w programie producenta. Poczekaj na koniec wywołań/ponowień,
sprawdź efekt i wpisz w konsoli Fridy:

```javascript
done("OSD: Wide; obraz wypelnia ekran")
```

Opis w `done` ma odzwierciedlać to, co faktycznie widzisz. Jeśli brak zmiany
albo opcja jest niedostępna, napisz to. Nie kopiuj przykładowego wyniku
bez sprawdzenia. Kolejne ustawienie rozpocznij nowym `mark`.
Zakończenie Fridy: Ctrl+D.

Przykładowe pola wywołania, nie wynik pomiaru:

```json
{"event":"call","stepId":1,"label":"Full Game -> Wide","function":"SetVCPFeature","code":"0x3a","valueDecimal":0,"valueHex":"0x00"}
```

`return.ok` oznacza wynik API, nie niezależne potwierdzenie fizycznego efektu.
Nie zobaczymy tu checksum ani pełnych ramek z przewodu. Jeśli podczas zmian
nie wystąpi żadne `call`, przeanalizujemy listę bibliotek i interfejs sterownika
używany przez producenta. Brak wywołań tych funkcji nie oznacza braku DDC.

Windows w VM musi mieć dostęp do monitora przez odpowiedni interfejs;
sam wirtualny ekran albo przekazanie huba USB nie zapewnia DDC monitora.
Wine wymaga osobnego ustalenia procesu i bibliotek; instrukcja Windows
nie jest automatycznie właściwa dla tej konfiguracji.

Źródła: https://frida.re/docs/javascript-api/ oraz
https://frida.re/docs/frida-cli/.
