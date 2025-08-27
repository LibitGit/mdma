mod html;

use std::ops::{Deref, Not};

use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::SignalVecExt;
use proc_macros::{ActiveSettings, Settings};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::addons::better_who_is_here::LevelDisplay;
use crate::prelude::*;

const ADDON_NAME: AddonName = AddonName::OnlinePeers;

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum DisplayTab {
    #[default]
    ClanMembers = 0,
    Friends = 1,
}

impl From<DisplayTab> for bool {
    fn from(value: DisplayTab) -> Self {
        match value {
            DisplayTab::ClanMembers => false,
            DisplayTab::Friends => true,
        }
    }
}

impl Not for DisplayTab {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::ClanMembers => Self::Friends,
            Self::Friends => Self::ClanMembers,
        }
    }
}

#[derive(ActiveSettings, Default)]
struct ActiveSettings {
    current_tab: Mutable<DisplayTab>,
    #[setting(skip)]
    scroll_visible: Mutable<Option<Id>>,
}

impl ActiveSettings {
    #[cfg(not(debug_assertions))]
    fn init(&'static self) {
        let future = map_ref! {
            let peers = Peers::get().signal_vec_keys().to_signal_cloned(),
            let scroll_target = self.scroll_visible.signal() => {
                scroll_target
                    .is_some_and(|target| !peers.contains(&target))
            }
        }
        .for_each(|should_remove| {
            if should_remove {
                self.scroll_visible.set(None);
            }
            async {}
        });
        wasm_bindgen_futures::spawn_local(future);
    }

    fn is_scroll_active(&self) -> bool {
        self.scroll_visible.lock_ref().is_some()
    }

    fn is_scroll_target(&self, other_id: Id) -> bool {
        self.scroll_visible
            .lock_ref()
            .is_some_and(|target| target == other_id)
    }

    fn to_map_alias(map_name: &str) -> &'static str {
        match map_name {
            // TITANS:
            "Mroczna Pieczara p.0" | "Migotliwa Pieczara" => "T-51",
            "Grota Caerbannoga" | "Jaskinia Caerbannoga" => "T-70",
            "Bandyckie Chowisko" | "Bandyckie Chowisko - skarbiec" => "T-101",
            "Wulkan Politraki - przedsionek" | "Wulkan Politraki - Piekielne Czeluście" => "T-131",
            "Lokum Złych Goblinów p.4" | "Lokum Złych Goblinów - pracownia" => "T-154",
            "Jaskinia Ulotnych Wspomnień" | "Źródło Wspomnień" => "T-177",
            "Więzienie Demonów" | "Komnata Krwawych Obrzędów" => "T-204",
            "Grota Jaszczurzych Koszmarów p.2" | "Dolina Potoku Śmierci" => "T-231",
            "Teotihuacan - przedsionek" | "Teotihuacan" => "T-258",
            "Sekretne Przejście Kapłanów" | "Sala Zrujnowanej Świątyni" => "T-285",
            "Przejście Władców Mrozu" | "Sala Tronowa" => "T-300",
            // COLOSSUS:
            "Pradawne Wzgórze Przodków" | "Świątynia Mzintlavy" => "K-36",
            "Pieczara Szaleńców - przedsionek"
            | "Pieczara Szaleńców - sala 4"
            | "Pieczara Szaleńców - sala Regulusa Mętnookiego" => "K-63",
            "Zmarzlina Amaimona Soplorękiego - przedsionek"
            | "Skały Mroźnych Śpiewów"
            | "Zmarzlina Amaimona Soplorękiego - sala" => "K-83",
            "Głębia Przeklętych Fal - przedsionek"
            | "Archipelag Bremus An"
            | "Głębia Przeklętych Fal - sala" => "K-114",
            "Przepaść Zadumy - przedsionek" | "Jezioro Ważek" | "Przepaść Zadumy - sala" => {
                "K-144"
            }
            "Czeluść Chimerycznej Natury - przedsionek"
            | "Przełęcz Krwistego Posłańca"
            | "Czeluść Chimerycznej Natury - sala" => "K-167",
            "Grobowiec Przeklętego Krakania - przedsionek"
            | "Krypty Bezsennych p.2 s.2"
            | "Grobowiec Przeklętego Krakania - sala" => "K-198",
            "Grota Przebiegłego Tkacza - przedsionek"
            | "Pajęczy Las"
            | "Grota Przebiegłego Tkacza - sala" => "K-225",
            "Grota Martwodrzewów - przedsionek"
            | "Regiel Zabłąkanych"
            | "Grota Martwodrzewów - sala" => "K-252",
            "Katakumby Antycznego Gniewu - przedsionek"
            | "Katakumby Krwawych Wypraw"
            | "Katakumby Antycznego Gniewu - sala" => "K-279",
            #[cfg(debug_assertions)]
            _ => "T-69",
            #[cfg(not(debug_assertions))]
            _ => "",
        }
    }

    fn to_map_alias_tip(map_name: &str) -> &'static str {
        match map_name {
            // TITANS:
            "Mroczna Pieczara p.0" | "Migotliwa Pieczara" => "Dziewicza Orlica",
            "Grota Caerbannoga" | "Jaskinia Caerbannoga" => "Zabójczy Królik",
            "Bandyckie Chowisko" | "Bandyckie Chowisko - skarbiec" => "Renegat Baulus",
            "Wulkan Politraki - przedsionek" | "Wulkan Politraki - Piekielne Czeluście" => {
                "Piekielny Arcymag"
            }
            "Lokum Złych Goblinów p.4" | "Lokum Złych Goblinów - pracownia" => "Versus Zoons",
            "Jaskinia Ulotnych Wspomnień" | "Źródło Wspomnień" => "Łowczyni Wspomnień",
            "Więzienie Demonów" | "Komnata Krwawych Obrzędów" => "Przyzywacz Demonów",
            "Grota Jaszczurzych Koszmarów p.2" | "Dolina Potoku Śmierci" => "Maddok Magua",
            "Teotihuacan - przedsionek" | "Teotihuacan" => "Tezcatlipoca",
            "Sekretne Przejście Kapłanów" | "Sala Zrujnowanej Świątyni" => {
                "Barbatos Smoczy Strażnik"
            }
            "Przejście Władców Mrozu" | "Sala Tronowa" => "Tanroth",
            // COLOSSUS:
            "Pradawne Wzgórze Przodków" | "Świątynia Mzintlavy" => "Mamlambo",
            "Pieczara Szaleńców - przedsionek"
            | "Pieczara Szaleńców - sala 4"
            | "Pieczara Szaleńców - sala Regulusa Mętnookiego" => "Regulus Mętnooki",
            "Zmarzlina Amaimona Soplorękiego - przedsionek"
            | "Skały Mroźnych Śpiewów"
            | "Zmarzlina Amaimona Soplorękiego - sala" => "Amaimon Soploręki",
            "Głębia Przeklętych Fal - przedsionek"
            | "Archipelag Bremus An"
            | "Głębia Przeklętych Fal - sala" => "Umibozu",
            "Przepaść Zadumy - przedsionek" | "Jezioro Ważek" | "Przepaść Zadumy - sala" => {
                "Vashkar"
            }
            "Czeluść Chimerycznej Natury - przedsionek"
            | "Przełęcz Krwistego Posłańca"
            | "Czeluść Chimerycznej Natury - sala" => "Hydrokora Chimeryczna",
            "Grobowiec Przeklętego Krakania - przedsionek"
            | "Krypty Bezsennych p.2 s.2"
            | "Grobowiec Przeklętego Krakania - sala" => "Lulukav",
            "Grota Przebiegłego Tkacza - przedsionek"
            | "Pajęczy Las"
            | "Grota Przebiegłego Tkacza - sala" => "Arachin Podstępny",
            "Grota Martwodrzewów - przedsionek"
            | "Regiel Zabłąkanych"
            | "Grota Martwodrzewów - sala" => "Reuzen",
            "Katakumby Antycznego Gniewu - przedsionek"
            | "Katakumby Krwawych Wypraw"
            | "Katakumby Antycznego Gniewu - sala" => "Wernoradzki Drakolisz",
            #[cfg(debug_assertions)]
            _ => "tip devowski pozdrr",
            #[cfg(not(debug_assertions))]
            _ => "",
        }
    }

    #[cfg(not(debug_assertions))]
    fn is_scroll_target_signal(&self, other_id: Id) -> impl Signal<Item = bool> {
        map_ref! {
            let peers = Peers::get().signal_vec_keys().to_signal_cloned(),
            let scroll_target = self.scroll_visible.signal() => {
                scroll_target.is_some_and(|target| target == other_id && peers.contains(&target))
            }
        }
        .dedupe()
    }

    #[cfg(debug_assertions)]
    fn is_scroll_target_signal(&self, other_id: Id) -> impl Signal<Item = bool> {
        map_ref! {
            let others = Others::get().signal_vec_keys().to_signal_cloned(),
            let peers = Peers::get().signal_vec_keys().to_signal_cloned(),
            let scroll_target = self.scroll_visible.signal() => {
                scroll_target.is_some_and(|target| target == other_id && (others.contains(&target) || peers.contains(&target)))
            }
        }
        .dedupe()
    }

    #[cfg(not(debug_assertions))]
    fn scroll_target_signal(&self) -> impl Signal<Item = Option<Id>> {
        map_ref! {
            let peers = Peers::get().signal_vec_keys().to_signal_cloned(),
            let scroll_target = self.scroll_visible.signal() => {
                scroll_target
                    .and_then(|target| peers.contains(&target).then_some(target))
            }
        }
        .dedupe()
    }

    #[cfg(debug_assertions)]
    fn scroll_target_signal(&self) -> impl Signal<Item = Option<Id>> {
        map_ref! {
            let others = Others::get().signal_vec_keys().to_signal_cloned(),
            let peers = Peers::get().signal_vec_keys().to_signal_cloned(),
            let scroll_target = self.scroll_visible.signal() => {
                scroll_target
                    .and_then(|target| (others.contains(&target) || peers.contains(&target)).then_some(target))
            }
        }
        .dedupe()
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum SortBy {
    #[default]
    Lvl = 0,
    Nick,
    Prof,
}

impl SortBy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lvl => "Poziomu",
            Self::Nick => "Nicku",
            Self::Prof => "Profesji",
        }
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum Ordering {
    Ascending = 0,
    #[default]
    Descending,
}

impl Ordering {
    fn as_str(self) -> &'static str {
        match self {
            Self::Descending => "Malejąco",
            Self::Ascending => "Rosnąco",
        }
    }
}

#[derive(Settings)]
struct Settings {
    sort_by: Mutable<SortBy>,
    ordering: Mutable<Ordering>,
    clear_target: Mutable<bool>,
    // false - show tip on text overflow only
    always_show_tip: Mutable<bool>,
    show_location: Mutable<bool>,
    show_alias: Mutable<bool>,
    level_display: Mutable<LevelDisplay>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sort_by: Mutable::default(),
            ordering: Mutable::default(),
            clear_target: Mutable::default(),
            always_show_tip: Mutable::new(true),
            show_location: Mutable::new(true),
            show_alias: Mutable::new(true),
            level_display: Mutable::default(),
        }
    }
}

/// Map a character to its position in the Polish alphabet
fn polish_char_value(c: char) -> u32 {
    match c {
        'a' => 1,
        'ą' => 2,
        'b' => 3,
        'c' => 4,
        'ć' => 5,
        'd' => 6,
        'e' => 7,
        'ę' => 8,
        'f' => 9,
        'g' => 10,
        'h' => 11,
        'i' => 12,
        'j' => 13,
        'k' => 14,
        'l' => 15,
        'ł' => 16,
        'm' => 17,
        'n' => 18,
        'ń' => 19,
        'o' => 20,
        'ó' => 21,
        'p' => 22,
        'q' => 23, // Not in Polish alphabet but included for completeness
        'r' => 24,
        's' => 25,
        'ś' => 26,
        't' => 27,
        'u' => 28,
        'v' => 29, // Not in native Polish alphabet but included for completeness
        'w' => 30,
        'x' => 31, // Not in Polish alphabet but included for completeness
        'y' => 32,
        'z' => 33,
        'ź' => 34,
        'ż' => 35,
        // For characters not in the Polish alphabet, assign a higher value
        _ => 1000 + (c as u32),
    }
}

/// Compare two strings according to Polish alphabetical order
pub fn compare_polish_strings(a: &str, b: &str) -> std::cmp::Ordering {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let len = std::cmp::min(a_chars.len(), b_chars.len());

    for i in 0..len {
        let a_val = polish_char_value(a_chars[i]);
        let b_val = polish_char_value(b_chars[i]);

        match a_val.cmp(&b_val) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    // If we've compared all characters up to the minimum length and they're equal,
    // then the shorter string comes first
    a_chars.len().cmp(&b_chars.len())
}

impl Settings {
    fn sort_signal(&'static self) -> impl Signal<Item = SortBy> {
        self.ordering
            .signal()
            .dedupe()
            .switch(|_| self.sort_by.signal().dedupe())
    }

    fn sort_ordering_signal(&self) -> impl Signal<Item = &'static str> {
        self.ordering.signal().map(|ordering| ordering.as_str())
    }

    fn level_display_signal(&self) -> impl Signal<Item = &'static str> {
        self.level_display
            .signal()
            .map(|level_display| level_display.as_str())
    }

    fn sort_by_signal(&self) -> impl Signal<Item = &'static str> {
        self.sort_by.signal().map(|sort| sort.as_str())
    }

    fn compare(&self, a: &Peer, b: &Peer) -> std::cmp::Ordering {
        //common::debug_log!("START CMP");
        let sort_by = self.sort_by.get();
        let ordering = self.ordering.get();

        match sort_by {
            SortBy::Nick => match ordering {
                Ordering::Ascending => Self::compare_nick(a, b),
                Ordering::Descending => Self::compare_nick(a, b).reverse(),
            },
            SortBy::Lvl => {
                let lvl_order = match ordering {
                    Ordering::Ascending => Self::compare_lvl(a, b),
                    Ordering::Descending => Self::compare_lvl(a, b).reverse(),
                };

                lvl_order.then_with(|| Self::compare_nick(a, b))
            }
            SortBy::Prof => {
                let prof_order = match ordering {
                    Ordering::Ascending => Self::compare_prof(a, b),
                    Ordering::Descending => Self::compare_prof(a, b).reverse(),
                };

                prof_order
                    .then_with(|| Self::compare_lvl(a, b).reverse())
                    .then_with(|| Self::compare_nick(a, b))
            }
        }

        //common::debug_log!(
        //    format!("{ord:?}"),
        //    serde_wasm_bindgen::to_value(a).unwrap_js(),
        //    serde_wasm_bindgen::to_value(b).unwrap_js(),
        //);
        //common::debug_log!("END CMP");
    }

    /// Implemented for ascending order, where
    /// the topmost value is the first rendered cell.
    fn compare_nick(a: &Peer, b: &Peer) -> std::cmp::Ordering {
        let nick_a = &a.nick;
        let nick_b = &b.nick;

        compare_polish_strings(
            nick_a.lock_ref().to_lowercase().as_str(),
            nick_b.lock_ref().to_lowercase().as_str(),
        )
        //nick_a
        //    .lock_ref()
        //    .to_lowercase()
        //    .cmp(&nick_b.lock_ref().to_lowercase())
    }

    /// Implemented for ascending order, where
    /// the topmost value is the first rendered cell.
    fn compare_lvl(a: &Peer, b: &Peer) -> std::cmp::Ordering {
        let lvl_a = &a.lvl;
        let lvl_b = &b.lvl;

        lvl_a.lock_ref().cmp(lvl_b.lock_ref().deref())
    }

    // TODO: Store the profession counts globally instead of recounting on every update.
    // FIXME: Errogenous search.
    /// Implemented for ascending order, where
    /// the topmost value is the first rendered cell.
    fn compare_prof(a: &Peer, b: &Peer) -> std::cmp::Ordering {
        // TODO: Rename the lock.
        let others_lock = Peers::get().lock_ref();
        //common::debug_log!(@f "others_length {}", others_lock.len());

        let prof_a = &a.prof;
        let prof_b = &b.prof;
        let prof_a = prof_a.get();
        let prof_b = prof_b.get();

        if prof_a == prof_b {
            //common::debug_log!("SAME PROF COUNT");
            return std::cmp::Ordering::Equal;
        }

        let mut prof_a_count = 1;
        let mut prof_b_count = 1;

        //if !others_lock.contains_key(&a.char_id) || !others_lock.contains_key(&b.char_id) {
        //    common::debug_log!(match others_lock.contains_key(&a.char_id) {
        //        true => "B HAS BEEN REMOVED ALREADY",
        //        false => "A HAS BEEN REMOVED ALREADY",
        //    });
        //}
        //
        others_lock
            .iter()
            .filter(|(id, _)| **id != a.char_id && **id != b.char_id)
            .for_each(|(_, other_data)| match other_data.prof.get() {
                prof if prof == prof_a => prof_a_count += 1,
                prof if prof == prof_b => prof_b_count += 1,
                _ => {}
            });
        //common::debug_log!(@f "prof_a_count: {prof_a_count} prof_b_count: {prof_b_count}");

        prof_a_count
            .cmp(&prof_b_count)
            .then_with(|| prof_a.cmp(&prof_b))
    }
}

pub(crate) fn init() -> JsResult<()> {
    let active_settings = ActiveSettings::new(ADDON_NAME);

    #[cfg(not(debug_assertions))]
    active_settings.init();

    let settings = Settings::new(ADDON_NAME);

    html::init(active_settings, settings)
}
