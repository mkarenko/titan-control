# Odczyt raportowanego profilu i jego ustawień

```fish
sudo ./docs/read_current_profile.sh
```

Pokazuje nazwę i Default/Custom z mapy WMW, surowy numer 22, bieżące
ustawienia obrazu, RGB User 1–3, Hue, Saturation oraz ustawienia globalne.
Nieobsługiwane i uszkodzone odpowiedzi są jawnie oznaczone. Próby odczytów
są poprzedzane GET 99. Sprawdzane są długość, bajty, nagłówek, kod VCP,
status i checksum. Nie ma Set VCP ani automatycznego przełączania profili.
Poprawna pusta odpowiedź DDC (`6e 80 be`) lub status „nieobsługiwany”
na przygotowawcze GET 99 nie przerywa sekwencji. Następny odczyt musi
zwrócić poprawną odpowiedź z wartością; NULL na GET 22 lub fragmencie
tabeli nadal oznacza błąd. Bajty po trzybajtowej ramce NULL są pomijane.
Program wymaga Python 3 i dostępu do /dev/i2c-N. Domyślny transport
`native` otwiera urządzenie raz i wykonuje transmisje przez I2C_RDWR.
Zapis zapytania i odczyt odpowiedzi są oddzielnymi transmisjami z przerwą;
nie są łączone przez repeated START. Nie uruchamia procesów i2ctransfer.
Poprzednia metoda jest dostępna przez `--transport i2ctransfer` i wymaga
i2c-tools. Zmiana transportu nie wyklucza dostępu innych programów do I2C.
Jeśli użytkownik ma uprawnienia do I2C,
sudo można pominąć. Domyślna magistrala to 14, adres 0x37.

```fish
sudo ./docs/read_current_profile.sh --bus 14 --addr 0x37 --debug
```

W trybie bieżących VCP skrypt wymaga dwóch zgodnych odczytów numeru profilu
przed i po pobraniu ustawień. Jeśli numer jest niestabilny lub zmienił się w trakcie, odrzuca
wyniki ustawień. Nie zastępuje to atomowego snapshotu ani potwierdzenia OSD.
Zamknij inne programy odpytujące monitor i nie zmieniaj ustawień w trakcie.
Pełny odczyt może potrwać kilkadziesiąt sekund, szczególnie przy błędach.

Wariant raportowany przez 22 nie zawsze zgadzał się z OSD w dotychczasowych
próbach. Dlatego wynik ma etykietę „Profil raportowany” i wskazuje mapę WMW.
Nie należy przedstawiać go jako gwarantowanego stanu Default/Custom z OSD.

## Opcjonalna tabela WMW

```fish
sudo ./docs/read_current_profile.sh --table --debug
```

Odczytuje tabelę dla numeru aktualnie raportowanego przez 22: GET 99,
GET 22, GET numer_profilu, dziewięć GET FE i dodatkowy GET FE kończący.
Przed tabelą jest tylko jeden odczyt GET 99 → GET 22, natychmiast
kontynuowany przez GET numer_profilu, zgodnie z logiem WMW. Nie ma dodatkowych
zapytań kontrolnych między selektorem a danymi. Po zakończeniu tabeli
numer jest sprawdzany jednokrotnie przez GET 99 → GET 22.
Końcowe sprawdzenie ma maksymalnie trzy próby. Poprawna odpowiedź z innym
numerem odrzuca wynik tabeli. Jeśli wszystkie trzy odpowiedzi są uszkodzone
lub puste, kompletna poprawna tabela jest wyświetlana z ostrzeżeniem:
numer profilu dotyczy początku odczytu, a stan końcowy jest niepotwierdzony.
Pobiera 40 bajtów danych i rozkłada je według schematu od indeksu 2.
Ten przebieg zaobserwowano dla numerów Custom; możliwość odczytu tabeli
Default wymaga sprawdzenia na monitorze. Nie przełącza Default na Custom
ani nie podstawia tabeli Custom w miejsce Default. Po pierwszym błędzie FE
czeka 260 ms i próbuje GET FF, a następnie kontynuuje GET FE do zakończenia
(maksymalnie 14 zapytań o fragmenty). To ścieżka odzyskiwania z logu WMW.
Odzyskany strumień nie jest dekodowany: przesunięcie kursora i rola bajtów
FF nie są potwierdzone. Po zakończeniu uruchamiana jest nowa pełna próba.
Po błędzie odrzucana jest cała tabela i ponawiana sekwencja od GET 99.
Domyślnie są cztery pełne próby; żadne fragmenty z różnych prób nie są
łączone. Nie ma potwierdzenia, że GET 99 zawsze resetuje kursor firmware:
powtórzenie też może się nie udać i musi przejść wszystkie kontrole.
Domyślnie skrypt czeka 40 ms od wysłania zapytania do odczytu odpowiedzi
i utrzymuje co najmniej 65 ms między początkami zapytań. Czas działania
sterownika (oraz uruchamiania i2ctransfer przy alternatywnym transporcie)
może wydłużyć ten odstęp. Jest to
przybliżenie tempa WMW, a nie pomiar jego opóźnień na magistrali.
Nie ma już dwóch długich przerw na zapytanie. `--delay` oznacza teraz tylko
czas od wysłania zapytania do odczytu odpowiedzi:

```fish
sudo ./docs/read_current_profile.sh --table --debug --delay 0.04 --attempts 6
```

Zakończenie 0000/5101 i podstawowe pola tabeli są kontrolowane. Nieznane
formaty kończą się błędem zamiast zgadywania. Read-only sekwencja tabeli
może mieć skutki uboczne w firmware (wcześniejsze notatki wspominają preview);
skrypt sprawdza raportowany numer po odczycie, ale nie wysyła zapisu naprawczego.
Tabela banku profilu i bieżące odczyty pojedynczych VCP to dwa źródła danych;
ich zgodność należy porównać, zwłaszcza po edycji ustawień.

Kod wyjścia: 0 pełny odczyt, 1 błąd profilu/tabeli lub zmiana profilu,
2 częściowy odczyt ustawień lub brak końcowego potwierdzenia profilu,
130 przerwanie przez użytkownika.
