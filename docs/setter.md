# Setter: kontrola odpowiedzi

```sh
./setter.sh e0 0
./setter.sh e0 14
```

Kod VCP jest w HEX, wartość w DEC. Setter zapisuje raz, czeka 250 ms,
wysyła Get VCP dla tego samego kodu, czeka 250 ms i odczytuje 11 bajtów.
Zachowuje wybór dodatkowej ramki `99=00F6`; dla E0 jej nie wysyła.
Wybór można wymusić przez `DO_COMMIT=0` lub `DO_COMMIT=1`.

Wynik to wyłącznie jedna linia:

- `success` (kod wyjścia 0): transmisje zakończyły się poprawnie, odpowiedź
  ma 11 bajtów, nagłówek `6e 88 02`, żądany kod VCP, poprawny checksum
  status protokołu `00` i odczytaną wartość zgodną z wysłaną.
- `error` (kod wyjścia 1): niepoprawny argument, błąd transmisji, uszkodzona
  lub niepełna odpowiedź, niepoprawny kod/status albo odpowiedź NULL.

Przy błędach odpowiedzi, statusie `01` (unsupported) lub rozbieżności wartości
setter ponawia tylko odczyt, maksymalnie pięć razy. Nie ponawia zapisu.
`success` potwierdza stan raportowany przez monitor, nie wykonanie konkretnej
ramki ani zachowanie ustawienia po odłączeniu zasilania.

Zmienne konfiguracyjne: `SETTLE_DELAY=0.250`, `VERIFY_READ_DELAY=0.250`,
`COMMIT_DELAY=0.125`, `DO_COMMIT=auto`, `VERIFY_MAX_READS=5`,
`RETRY_DELAY=0.250`. Argumenty `bus:N` i `addr:0xNN`
nadpisują domyślne bus 14 i adres 0x37. Opcje poprzedniej wersji
`VERIFY` i `SET_MAX_ATTEMPTS` nie są używane.
