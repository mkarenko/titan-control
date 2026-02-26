# Titan Control 🖥️🛡️

**Titan Control** to nowoczesne oprogramowanie na system Linux, służące do bezpośredniego, sprzętowego sterowania parametrami monitorów zewnętrznych. Aplikacja została zbudowana z myślą o nowoczesnych środowiskach graficznych (Wayland/X11) i oferuje interfejs w pełni zgodny z wytycznymi GNOME (Libadwaita).

W przeciwieństwie do narzędzi programowych, które nakładają filtry na obraz, Titan Control komunikuje się z monitorem przez magistralę I2C (DDC/CI), zmieniając rzeczywiste ustawienia podświetlenia i procesora obrazu wewnątrz monitora.

---

## ✨ Funkcje

* **Pełna kontrola obrazu:** Jasność, Kontrast, Ostrość
* **Zaawansowane parametry:** Low Blue Light, DCR (Dynamic Contrast), Czujnik światła
* **Zarządzanie kolorem:** Wybór gammy, proporcji obrazu oraz temperatury barwowej
* **Obsługa Audio:** Głośność oraz szybkie wyciszanie (Mute)
* **Informacje o sprzęcie:** Odczyt nazwy modelu i aktualnego języka menu OSD
* **Integracja z systemem:**

  * Ikona w zasobniku systemowym (Tray)
  * Nowoczesny interfejs GTK4/Libadwaita
  * Pełna kompatybilność z Waylandem (brak zależności od X11)

---

## 🚀 Technologia

Aplikacja została napisana w języku **Rust**, co zapewnia maksymalną wydajność i bezpieczeństwo pamięci.

* **GUI:** GTK4 + Libadwaita
* **Hardware:** ddc-i2c + i2c-linux (bezpośrednia komunikacja z urządzeniami `/dev/i2c-*`)
* **Asynchroniczność:** async-channel + glib::MainContext (płynny interfejs podczas komunikacji ze sprzętem)

---

## 🛠️ Wymagania i instalacja

### 1. Zależności systemowe

Upewnij się, że masz zainstalowane biblioteki deweloperskie dla GTK4 i Libadwaita:

```bash
# Ubuntu/Debian/Pop!_OS
sudo apt install libgtk-4-dev libadwaita-1-dev libi2c-dev

# Fedora
sudo dnf install gtk4-devel libadwaita-devel i2c-tools-devel

# Arch Linux
sudo pacman -S gtk4 libadwaita i2c-tools
```

### 2. Uprawnienia I2C (Bardzo ważne)

Aplikacja wymaga dostępu do magistrali I2C. Aby uniknąć uruchamiania programu przez `sudo`, dodaj swojego użytkownika do grupy `i2c`:

```bash
# Załaduj moduł jądra
sudo modprobe i2c-dev

# Dodaj użytkownika do grupy
sudo usermod -aG i2c $USER
```

> Po wykonaniu powyższych komend należy się wylogować i zalogować ponownie.

### 3. Kompilacja

```bash
git clone https://github.com/twoj-uzytkownik/titan_control.git
cd titan_control
cargo build --release
```

---

## 🖥️ Uruchomienie

W trybie deweloperskim:

```bash
cargo run
```

---

## 🏗️ Struktura projektu

* `src/main.rs` – Główny punkt wejścia, łączenie sygnałów i pętla zdarzeń
* `src/monitor.rs` – Logika komunikacji DDC/CI, wątek roboczy (Worker)
* `src/ui.rs` – Definicja nowoczesnego interfejsu użytkownika
* `src/tray.rs` – Obsługa ikony w zasobniku systemowym

---

## 📝 Roadmap

* Obsługa wielu monitorów jednocześnie
* Tworzenie i zapisywanie profili ustawień (np. "Praca", "Noc", "Gaming")
* Synchronizacja jasności monitora z jasnością systemową laptopa
* Automatyczne przełączanie jasności w zależności od pory dnia

---

## 📄 Licencja

Projekt udostępniany na licencji **MIT**. Szczegóły w pliku `LICENSE`.

---

Stworzone z pasją do wydajnego oprogramowania na Linuksa 🐧
