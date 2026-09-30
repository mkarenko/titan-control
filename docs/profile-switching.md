# Przełączanie profili i ustawień ekranu

## Tryby obrazu monitora

Wybór trybu wysyła VCP `0x22`. Każdy tryb ma osobny kod wariantu
„Domyślny” oraz (poza FPS Mode) „Własny”. Mapy kodów są w
`PICTURE_MODE_DEFAULT_VALUES` i `PICTURE_MODE_CUSTOM_VALUES` w `src/monitor.rs`.

Przed przejściem do innego trybu aplikacja zapisuje stan poprzedniego profilu.
Wczytuje zapamiętany wariant docelowego trybu; dla „Własny” wysyła też zapisane
wartości. Opuszczenie wariantu domyślnego nie nadpisuje zapisanych ustawień
własnych. Dane są w `~/.config/titan_control/app_settings.json`
(lub odpowiednim katalogu konfiguracji użytkownika).

Dla DyDs/ULL FPS oraz DyDs/LD, zarówno domyślnych, jak i własnych, kolejka
sprzętowa wysyła najpierw `0xE2 = 0` (Adaptive Sync wyłączony), a następnie
kod trybu `0x22`. Powrót do innego trybu nie włącza Adaptive Sync automatycznie.
Zapis I2C bez błędu nie stanowi potwierdzenia przyjęcia ustawienia przez firmware.

Lista zachowuje kolejność monitora: E-Book → sRGB → AdobeRGB. Ulubione są
skrótami w trayu i nie zmieniają tej kolejności.

### Ograniczenie wariantu „Domyślny”

Kliknięcie przycisku „Domyślny” wybiera domyślny kod trybu i dodatkowo wysyła
`basic_profile_values_for_mode`. Obecnie funkcja zwraca ten sam zestaw wartości
dla wszystkich trybów. Nie jest to kompletna kopia fabrycznej tabeli każdego
profilu. Sam wybór domyślnego trybu z listy wysyła kod trybu i uzupełnia UI
wartościami aplikacji; nie odczytuje wtedy ponownie całej tabeli z monitora.
