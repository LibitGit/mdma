# Changelog

## MDMA 0.12.0 (2025/02/09)

### Dodano
- `Gracze na mapie`: dodano nowe menu kontekstowe `Dobijaj` pozwalające na obranie danego gracza za cel.
    - Po wybraniu tej opcji bohater podejdzie do celu i będzie go atakować, dopóki m.in. przeciwnik się nie teleportuje lub nie rozpocznie się walka.
- `Gracze na mapie`: dodano wyświetlanie klanu gracza wewnątrz tipa.
- `Kastrat`: dodano możliwość wyłączenia atakowania celu z poziomu okna dodatku.
- NOWOŚĆ `Super Rzemieślnik`: dodatek pozwala wykorzystać przedmioty z łupu lub ekwipunku jako składniki do ulepszania.
    - Automatyczne ulepszanie przedmiotów umieszczonych w oknie dodatku przy użyciu wybranych typów łupów z potworów.
    - Przycisk **Ulepsz** umożliwiający ulepszanie przedmiotami o rzadkości wyższej niż pospolita w dowolnym momencie.
- `Znacznik`: dodano możliwość zmiany rzadkości własnych znaczników.
- `Znacznik`: dodano animacje wewnątrz oraz przy wyborze własnych znaczników.


### Zmieniono
- `Accept Group`: zmieniono nazwę dodatku na `Akceptowanie zaproszeń do drużyny`.
- `Accept Summon`: zmieniono nazwę dodatku na `Akceptowanie przywołań`.
- `Better Group Invites`: zmieniono nazwę dodatku na `Zapraszanie do drużyny`.
- `Better Messages`: zmieniono nazwę dodatku na `Poprawione powiadomienia`.
- `Better Who Is Here`: zmieniono nazwę dodatku na `Gracze na mapie`.
- `Manager`: odświeżono wygląd menadżera.

### Naprawiono
- `Kastrat`: naprawiono niepoprawne wyświetlanie okna dodatku w przypadku wyłączonego dodatku. 
- `Service Worker`: naprawiono niezamierzone wstrzykiwanie zestawu na subdomenach `commons` oraz `dev-commons`.
- `Znacznik`: naprawiono niewyświetlanie własnych znaczników.

### Zmiany wewnętrzne
- Dodano nowy tryb dla komponentu `Input` - `slider` pozwalający na wybór wartości z zakresu.
- Dodano responsywność ramek oraz nakładek przedmiotów wewnątrz komponentu `Input`.
- Dodano śledzenie lokalizacji wewnątrz anonimowych funkcji za pomocą bramki `#![feature(closure_track_caller)]`.
- Naprawiono niezamierzone wywoływanie błędu, gdy zestaw nie był w stanie określić profesji gracza z grupy.
- Poprawiono mechanizm czyszczenia zmiennych globalnych przy zmianie lokacji.
- Poprawiono mechanizm wykrywania czy bohater jest w grupie lub opuszcza grę.

## MDMA 0.11.3 (2025/01/29)

### Dodano

### Zmieniono
- `Better Group Invites`: zapraszanie oraz zapraszanie masowe według profesji:
    - w przypadku, gdy dodatek nie może określić profesji członka grupy aktualna, liczba dostępnych miejsc dla danej profesji pozostaje niezmieniona. 
- `Znacznik`: domyślną ikonę dla aliasów mapy **Grota Caerbannoga** na starą ikonę tytana **Zabójczy królik**.

### Naprawiono
- `Better Group Invites`: sprawdzanie czy kandydat jest już w grupie w przypadku, gdy bohater był poprzednio w grupie, ale aktualnie nie jest.
- `Better Messages`: wyłączanie interakcji z tekstem.
- `Kastrat`: sprawdzanie czy kandydat na cel znajduje się w grupie z bohaterem.

### Zmiany wewnętrzne
- Dodano własny seed używany podczas hashowania zmiennych typu **string**.
- Usunięto wszystkie ścieżki absolutne z modułów WASM.

## MDMA 0.11.2-test (2025/01/27)

### Dodano

### Zmieniono

### Naprawiono

### Zmiany wewnętrzne
- Tymczasowo wyłączono cachowanie modułu `foreground`.
- Zmieniono sposób inicjalizacji zestawu. Wszystkie funkcjonalności zostają wczytane przed wysłaniem pierwszego żądania do serwera gry.

## MDMA 0.11.1-test (2025/01/27)

### Dodano

### Zmieniono
- `Manager`: implementacje ustawień postaci przystosowując je do obecnej wersji gry.
- `Manager`: tymczasowo usunięto wyświetlanie błędu w sytuacji, gdy emocja gracza powinna zostać usunięta z listy emocji, ale lista została już wyczyszczona, np. wskutek opuszczenia mapy przez gracza.

### Naprawiono

### Zmiany wewnętrzne
- Dodano generowanie kodów błedu w przypadku wystąpienia błędu wewnątrz funkcji `onMessageWebSocket`.

## MDMA 0.11.0-test (2025/01/24)

### Dodano
- NOWOŚĆ `Znacznik`: dodatek umożliwia konfigurowanie ikon oraz podpisów przedmiotów z gry
    - aliasy lokacji oraz ikony potworów nad każdym przedmiotem z kategorii `custom_teleport`,
    - ikony typów obrażeń broni,
    - edytowanie ikon, podpisów oraz rzadkości przedmiotów bohatera.
- `Kastrat`: możliwość atakowania graczy poszukiwanych listem gończym w lokacjach z warunkowym PvP.
- `Service Worker`: zmiany w ustawieniach dodatków są teraz wysyłane do serwera w pakietach co 150ms.

### Zmieniono
- `Better Group Invites`: klanowicze oraz przyjaciele z tej samej lokacji są od teraz zapraszani niezależnie od ich odległości od bohatera.
- `Signed Custom Teleports`: usunięto dodatek.

### Naprawiono
- `Kastrat`: niepoprawne wykrywanie trybu PvP obecnej lokacji.
- `Manager`: błąd podczas tworzenia nowej grupy.
- `Manager`: niepoprawne pozycjonowanie okien w przypadku zbyt małego viewportu.
- `Manager`: niepoprawne wyświetlanie tipu po usunięciu elementu odpowiającego za jego renderowanie.
- `Service Worker`: rozbudzanie workera poprzez zmianę ustawień któregokolwiek dodatku.

### Zmiany wewnętrzne
- Dodano minifikowanie kodu plików z rozszerzeniami `.js`.
- `Service Worker`: zaimplementowano kolejkowanie wiadomości wysyłanych do serwera w przypadku zbyt częstego aktualizowania np. ustawień dodatku.
- Tymczasowo zrezygnowano z enkodowania plików z rozszerzeniami `.wasm`.
- Dodano nowy tryb dla komponentu `Input` - `game-item` pozwalający na modyfikowanie przedmiotów z gry z poziomu shadow DOM tree.
- `Manager`: okna dodatków są teraz renderowane po wysłaniu eventu `AFTER_INTERFACE_START` przez obiekt `API`.
- Wskaźnik odnoszący się do danych globalnych zestawu jest teraz celowo wyciekany, znacząco zwiększając ogólną wydajność.
- Dodano funkcjonalność pozwalającą na przechowywanie zmiennych `BTreeMap` jako obiekt `JSON` do makr proceduralnych odpowiadających za automatyczne komunikowanie zmian w dodatkach.

## MDMA 0.10.0-test (2024/12/30)

### Dodano
- NOWOŚĆ `Kastrat`: kamil odpalaj kastrata bo mi cwele na expowisko wbiły.
- `Widget`: możliwość otwarcia menu rozszerzenia za pomocą prawego przycisku myszy (PPM), niezależnie od statusu zalogowania użytkownika.

### Zmieniono
- Komunikat w przypadku błędu podczas inicjalizacji zestawu.

### Naprawiono
- `Manager`: opcję wyłączania widżetu.

### Zmiany wewnętrzne
- Dodano funkcjonalność znacząco usprawniającą wczytywanie oraz zapisywanie ustawień dodatków.
- Arkusze styli zostały przeniesione na serwer.

## MDMA 0.9.1-test (2024/12/17)

### Dodano
- `Manager`: komunikat w przypadku nieudanej inicjalizacji zestawu.

### Zmieniono
- `Manager`: komunikat w przypadku zbyt niskiego poziomu dostępu.

### Naprawiono
- `Manager`: sprawdzanie stanu aktywności dodatku.

### Zmiany wewnętrzne
- Poprawiono wewnętrzną funkcjonalność odpowiadającą za inicjalizacje zestawu.

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
- Zmieniono odczytywanie właściwości obiektów gry z parsowania przy użyciu `serde` na korzystanie z powiązań wygenerowanych przez `wasm-bindgen`. Rezultatem tej zmiany jest znaczna poprawa ogólnej wydajności.

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
