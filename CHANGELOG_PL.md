# Changelog

## MDMA 0.9.0-test (2024/12/16)

### Dodano
- Funkcjonalność pozwalającą na zapisywanie ustawień dodatków przy każdej zmianie.
- `Popup`: system logowania za pomocą konta Discord.

### Zmieniono
- `Better Group Invites`: walidacja kandydata przebiega teraz przy każdej iteracji pętli wywołującej zaproszenia, zamiast jednokrotnej walidacji wszystkich kandydatów przy kliknięciu przycisku odpowiedzialnego za rozsyłanie zaproszeń.

### Naprawiono

### Zmiany wewnętrzne
- Poprawiono obsługę błędów oraz zastąpiono wiadomości kodami błędu.
- Poprawiono wewnętrzną funkcjonalność odpowiadającą za inicjalizacje zestawu.
- Wraz z wprowadzeniem mechanizmu logowania zablokowano dostęp do zestawu dla użytkowników nieposiadających uprawnień.

## MDMA 0.8.0-test (2024/11/22)

### Dodano

### Zmieniono
- `Signed Custom Teleports`: pozycje w aliasach po aktualizacji przedziału 120 - 175.

### Naprawiono
- `Better Group Invites`: odznaczanie w polach wyboru.
- Żądania nie zostają wysyłane podczas wylogowywania.

### Zmiany wewnętrzne
- Moduł zawierający dodatki jest od teraz instancjowany przez anonimową funkcję z poziomu Rust(🚀).
- Dane o rówieśnikach aktualizują się natychmiastowo po wejściu/wyjściu z gry rówieśnika.
- Zaimplementowano kompresję Brotli w module WASM.
- `Future` odpowiedzialny za usunięcie emocji zostaje przerwany w przypadku, gdy serwer gry zwróci `task: "reload"`.
- Zmieniono odczytywanie właściwości obiektów gry z parsowania  przy użyciu `serde` na korzystanie z powiązań wygenerowanych przez `wasm-bindgen`. Rezultatem tej zmiany jest znaczna poprawa ogólnej wydajności.

## MDMA 0.7.0-test (2024/11/17)

### Dodano

### Zmieniono
- `Console`: Kopiowanie logów zapewnia teraz do 500 ostatnich odpowiedzi z serwera gry razem z logami zestawu.

### Naprawiono
- `Accept Group`: układ okna.
- `Accept Summon`: układ okna.
- `Auto-X`: układ okna.
- `Better Group Invites`: wielkość liter nie ma znaczenia w przypadku zapraszania graczy według nicku.
- `Better Group Invites`: układ okna.
- `Better Messages`: układ okna.
- `Better Who Is Here`: układ okna.
- `Signed Custom Teleports`: układ okna.

### Zmiany wewnętrzne
- Zrefaktoryzowano moduł tworzenia okien konfiguracji oraz ustawień.

## MDMA 0.6.0-test (2024/11/13)

### Dodano
- NOWOŚĆ `Accept Group`: dodatek obsługuje przychodzące zaproszenia do drużyn.
- NOWOŚĆ `Accept Summon`: dodatek pozwala na automatyczne akceptowanie określonych przywołań.
- NOWOŚĆ `Better Group Invites`: dodatek obsługuje wychodzące zaproszenia do drużyny.
- `Better Messages`: okno konfiguracji.
- `Popup`: dodano okienko rozszerzenia.
- `UI`: responsywność na wielkość czatu z gry.
- `UI`: obsługa zmian w komponentach za pomocą sygnałów FRP.

### Zmieniono
- `Auto Group`: dodatek został podzielony na dwa dodatki, `Accept Group` oraz `Better Group Invites` ze względu na zbyt dużą złożoność.
- `UI`: okna dodatków nie mogą być przenoszone poprzez przeciąganie elementów znajdujących się w nagłówku.
- `UI`: obramowanie tipów jest teraz ograniczone do rozmiaru viewportu.

### Naprawiono
- `Service Worker`: poprawne wybudzanie Service Workera po otrzymaniu zdarzenia.
- `UI`: ułożenie okna po jego otwarciu.

### Zmiany wewnętrzne
- Poprawiono obsługę oraz wiadomości błędów.
- Usunięto wsparcie dla wielowątkowości, zmniejszając rozmiar modułu WASM o ~60%.
- Wprowadzono obfuskację oraz cachowanie zmiennych typu string wewnątrz modułu WASM.
- Dodano aktualizowanie danych rówieśników przy zmianie lokacji.
- Inicjalizacja zestawu nie blokuje inicjalizacji innych zestawów dodatków.
- Stworzono framework do zarządzania obiektami DOM.
- Stworzono bibliotekę do powiązań z API WebExtension w Rust(🚀).
- Stworzono framework do komunikacji pomiędzy kontekstami rozszerzenia.

## MDMA 0.5.0-test (2024/07/29)

### Dodano
- `Auto Group`: okno ustawień.
- NOWOŚĆ `Auto-X`: wersja stworzona do testowania nowych okien dodatków.
- NOWOŚĆ `Better Messages`: dodatek pozwala na konfigurowanie "żółtych" wiadomości z gry.
- `Console`: dodano konsolę wraz z przyciskiem do kopiowania logów wewnątrz głównego okna zestawu.
- NOWOŚĆ `Signed Custom Teleports`: dodatek tworzy aliasy lokacji nad każdym przedmiotem z kategorii `custom_teleport`.

### Zmieniono
- `Widget`: zmieniono domyślną pozycję widźetu.

### Naprawiono
- `Better Who Is Here`: emotions update if the server responds with the same emotion before the previous one stopped displaying. 
- `Better Who Is Here`: updated `noemo` handling.

### Zmiany wewnętrzne
- Moduły odpowiedzialne za ładowanie zestawu zatrzymują ładowanie gry do momentu zakończenia jego inicjalizacji.
- Każdy element zestawu jest renderowany wewnątrz shadow DOM 🥷. Wewnętrzna struktura drzewa będącego częścią shadow DOM jest ukryta przed działającym na stronie JS i CSS.
- Dodano komunikację między grą a rozszerzeniem.
- `Auto Group`: zaimplementowano nowy algorytm do obsługi zaproszeń do grupy, zwiększając jego prędkość do 100 µs/zaproszenie.

## MDMA 0.4.0-test (2024/06/16)

### Dodano

### Zmieniono

### Naprawiono

### Zmiany wewnętrzne
- Przeniesiono część funkcjonalności na serwer.

## MDMA 0.3.0-test (2024/06/03)

### Dodano
- `UI`: dodano interfejs graficzny.

### Zmieniono

### Naprawiono

### Zmiany wewnętrzne

## MDMA 0.2.0-test (2024/05/27)

### Dodano
- Pierwsza testowa wersja zestawu w Rust 🦀 

### Zmieniono

### Naprawiono
- `Auto Group`: zmienna `ask` zostaje usunięta z odpowiedzi serwera tylko w przypadku wysłania zaproszenia do grupy dla bohatera.
- `Auto Group`: poprawiono akceptowanie zaproszenia do grupy w przypadku zaproszenia przychodzącego od gracza znajdującego się na tej samej mapie co bohater.
- `Better Who Is Here`: emocje aktualizują swoje pozycje po zniknięciu jednej z nich. 
- `Better Who Is Here`: poprawiono czas wyświetlania emocji.

### Zmiany wewnętrzne

## MDMA 0.1.0-test (2023/11/30)

### Dodano
- Pierwsza publicznie udostępniona wersja zestawu!

### Zmieniono

### Naprawiono

### Zmiany wewnętrzne
