mod html;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Deref;

use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::{SignalVecExt, VecDiff};
use futures_signals::{map_ref, signal};
use js_sys::JsString;
use proc_macros::Settings;
use serde_repr::{Deserialize_repr, Serialize_repr};
use wasm_bindgen::{intern, prelude::*};
use wasm_bindgen_futures::JsFuture;

use crate::addons::kastrat::{MIN_DIFF, TargetData};
use crate::bindings::engine::types::MapMode;
#[cfg(feature = "antyduch")]
use crate::pathfinder::{Pos, pathfind_to};
use crate::prelude::*;

use super::kastrat::Target;

thread_local! {
    static EMOTION_SOURCES: RefCell<HashMap<EmotionName, JsString>> = RefCell::new(HashMap::with_capacity(20));
}

const ADDON_NAME: AddonName = AddonName::BetterWhoIsHere;

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

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum LevelDisplay {
    First = 0,
    Last,
    None,
    #[default]
    Only,
}

impl LevelDisplay {
    pub fn as_str(self) -> &'static str {
        use LevelDisplay::*;

        match self {
            First => "Poziom | Poziom operacyjny",
            Last => "Poziom operacyjny | Poziom",
            None => "Tylko poziom operacyjny",
            Only => "Tylko poziom",
        }
    }
}

#[derive(Settings)]
struct Settings {
    sort_by: Mutable<SortBy>,
    ordering: Mutable<Ordering>,
    clear_target: Mutable<bool>,
    level_display: Mutable<LevelDisplay>,
    replace_widget: Mutable<bool>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sort_by: Mutable::default(),
            ordering: Mutable::default(),
            clear_target: Mutable::default(),
            level_display: Mutable::default(),
            replace_widget: Mutable::new(true),
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
    #[cfg(feature = "ni")]
    fn init(&'static self) -> JsResult<()> {
        let who_is_here = get_engine().who_is_here().ok_or_else(|| err_code!())?;

        let original_manage_panel_visible = who_is_here
            .get_manage_panel_visible()
            .ok_or_else(|| err_code!())?;
        let new_manage_panel_visible = closure!(
            { let who_is_here = who_is_here.clone() },
            move || -> JsResult<()> {
                match self.replace_widget.get() && Addons::is_active(ADDON_NAME) {
                    true => {
                        if who_is_here.is_show()? {
                            who_is_here.close_panel()?;
                        }
                        Addons::toggle_addon_active_state(ADDON_NAME, WindowType::AddonWindow)
                    }
                    false => original_manage_panel_visible.call0(&who_is_here).map(|_| ())
                }
            },
        );
        who_is_here.set_manage_panel_visible(&new_manage_panel_visible);

        Ok(())
    }

    fn sort_signal(&'static self) -> impl Signal<Item = SortBy> {
        self.ordering
            .signal()
            .dedupe()
            .switch(|_| self.sort_by.signal().dedupe())
    }

    fn sort_ordering_signal(&self) -> impl Signal<Item = &'static str> {
        self.ordering.signal().map(|ordering| ordering.as_str())
    }

    fn sort_by_signal(&self) -> impl Signal<Item = &'static str> {
        self.sort_by.signal().map(|sort| sort.as_str())
    }

    fn level_display_signal(&self) -> impl Signal<Item = &'static str> {
        self.level_display
            .signal()
            .map(|level_display| level_display.as_str())
    }

    fn compare(&self, a: &Other, b: &Other) -> std::cmp::Ordering {
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
    fn compare_nick(a: &Other, b: &Other) -> std::cmp::Ordering {
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
    fn compare_lvl(a: &Other, b: &Other) -> std::cmp::Ordering {
        let lvl_a = &a.lvl;
        let lvl_b = &b.lvl;

        lvl_a.lock_ref().cmp(lvl_b.lock_ref().deref())
    }

    // TODO: Store the profession counts globally instead of recounting on every update.
    // FIXME: Errogenous search.
    /// Implemented for ascending order, where
    /// the topmost value is the first rendered cell.
    fn compare_prof(a: &Other, b: &Other) -> std::cmp::Ordering {
        let others_lock = Others::get().lock_ref();
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

#[derive(Default)]
struct ActiveSettings {
    // TODO: Rename to `scroll_target` ?
    scroll_visible: Mutable<Option<Id>>,
    search_text: Mutable<String>,
    //#[setting(skip)]
    target: Target,
    after_follow: Cell<bool>,
}

impl ActiveSettings {
    const WINDOW_TYPE: WindowType = WindowType::AddonWindow;

    fn init(&'static self) -> JsResult<()> {
        self.target.init();

        // TODO: Add a signal for contains.
        let future = map_ref! {
            let others = Others::get().signal_vec_keys().to_signal_cloned(),
            let scroll_target = self.scroll_visible.signal() => {
                Premium::active()
                    .then(|| scroll_target.is_some_and(|target| !others.contains(&target)))
                    .unwrap_or(true)
            }
        }
        .dedupe()
        .for_each(|should_remove| {
            if should_remove {
                self.scroll_visible.set_neq(None);
            }

            async {}
        });
        wasm_bindgen_futures::spawn_local(future);

        // Clear the target if it becomes part of hero's party.
        let future = Party::get().signal_vec_keys().for_each(|diff| {
            match diff {
                VecDiff::Push { value }
                | VecDiff::InsertAt { value, .. }
                | VecDiff::UpdateAt { value, .. } => {
                    if self.is_target(value) {
                        self.clear_target();
                    }
                }
                VecDiff::Replace { values } => {
                    if values.into_iter().any(|other_id| self.is_target(other_id)) {
                        self.clear_target();
                    }
                }
                _ => {}
            };

            async {}
        });
        wasm_bindgen_futures::spawn_local(future);

        Ok(())
    }

    fn clear_target(&self) {
        let Some(_old_target) = self.target.clear(ADDON_NAME) else {
            return;
        };

        self.after_follow.set(false);
    }

    fn is_scroll_active(&self) -> bool {
        self.scroll_visible.lock_ref().is_some()
    }

    fn is_scroll_target(&self, other_id: Id) -> bool {
        self.scroll_visible
            .lock_ref()
            .is_some_and(|target| target == other_id)
    }

    fn is_scroll_target_signal(&self, other_id: Id) -> impl Signal<Item = bool> {
        map_ref! {
            let others = Others::get().signal_vec_keys().to_signal_cloned(),
            let scroll_target = self.scroll_visible.signal() => {
                scroll_target.is_some_and(|target| target == other_id && others.contains(&target))
            }
        }
        .dedupe()
    }

    fn scroll_target_signal(&self) -> impl Signal<Item = Option<i32>> {
        map_ref! {
            let others = Others::get().signal_vec_keys().to_signal_cloned(),
            let scroll_target = self.scroll_visible.signal() => {
                scroll_target
                    .and_then(|target| others.contains(&target).then_some(target))
            }
        }
        .dedupe()
    }

    fn is_target(&self, other_id: OtherId) -> bool {
        self.target
            .lock_ref()
            .as_ref()
            .is_some_and(|target_data| target_data.char_id == other_id)
    }

    fn is_target_signal(&self, other_id: OtherId) -> impl Signal<Item = bool> {
        self.target
            .signal_ref(move |target_data_opt| {
                target_data_opt
                    .as_ref()
                    .is_some_and(|target_data| target_data.char_id == other_id)
            })
            .dedupe()
    }

    fn can_attack_on_map(target: &Target, map_mode: MapMode) -> bool {
        #[cfg(debug_assertions)]
        if map_mode == MapMode::Arena {
            return true;
        }
        if target.is_wanted() && map_mode == MapMode::AgreePvp {
            return true;
        }

        matches!(
            map_mode,
            MapMode::Pvp | MapMode::InstanceSolo | MapMode::InstanceGrp
        )
    }
}

pub(crate) fn init() -> JsResult<()> {
    use futures_signals::signal_map::{MapDiff, SignalMapExt};

    let settings = Settings::new(ADDON_NAME);
    #[cfg(feature = "ni")]
    settings.init()?;
    let active_settings: &'static ActiveSettings = Box::leak(Box::new(ActiveSettings::default()));
    active_settings.init()?;

    let on_other = Others::get().signal_map_cloned().for_each(move |change| {
        match change {
            MapDiff::Remove { key } => {
                if (!Premium::active() || settings.clear_target.get())
                    && active_settings.is_target(key)
                {
                    active_settings.clear_target();
                }
            }
            MapDiff::Clear {} => {
                if !Premium::active() || settings.clear_target.get() {
                    active_settings.clear_target();
                }
            }
            _ => {}
        }

        async {}
    });

    wasm_bindgen_futures::spawn_local(on_other);

    let future = async move {
        loop {
            let future = active_settings
                .target
                .signal_ref(move |value| {
                    signal::option(value.as_ref().map(TargetData::coords_signal)).map(|coords| {
                        // If no target or coords
                        if coords.flatten().is_none() {
                            return false;
                        }
                        let map_mode = get_engine()
                            .map()
                            .unwrap_js()
                            .map_data()
                            .unwrap_js()
                            .get_pvp()
                            .unwrap_or(MapMode::NonPvp);
                        if !ActiveSettings::can_attack_on_map(&active_settings.target, map_mode) {
                            return false;
                        }
                        if Hero::in_battle() {
                            if !Premium::active() || settings.clear_target.get() {
                                active_settings.clear_target();
                            }
                            return false;
                        }

                        true
                    })
                })
                .flatten()
                .dedupe()
                .wait_for(true);
            future.await;

            let target_lock = active_settings.target.lock_ref();
            if let Some(target) = target_lock.as_ref() {
                #[cfg(feature = "antyduch")]
                if let Some(target_x) = target.x.get()
                    && let Some(target_y) = target.y.get()
                {
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Err(err_code) =
                            pathfind_to(Pos::new(target_x as usize, target_y as usize)).await
                        {
                            console_error!(err_code)
                        }
                    });
                }

                if target.in_battle() {
                    // debug_log!("in battle");
                    drop(target_lock);

                    delay(MIN_DIFF as u32).await;
                    continue;
                }
                if Party::get().lock_ref().contains_key(&target.char_id) {
                    // debug_log!("in party");
                    drop(target_lock);
                    delay(MIN_DIFF as u32).await;
                    continue;
                }
                if !target.in_attack_range() {
                    // debug_log!("not in range");
                    get_engine().idle_json().unwrap_js().set_default_diff();
                    drop(target_lock);
                    if active_settings.after_follow.get() && settings.clear_target.get() {
                        active_settings.clear_target();
                    }
                    delay(MIN_DIFF as u32).await;
                    continue;
                }

                get_engine().idle_json().unwrap_js().set_diff(MIN_DIFF);
                let mut attack_req = string!("fight&a=attack&id=");
                attack_req.push_str(
                    target_lock
                        .as_ref()
                        .unwrap_js()
                        .char_id
                        .to_string()
                        .as_str(),
                );
                let _ = message(&format!(
                    "[MDMA::RS] Atakuję \"{}\"",
                    &target.nick.get_cloned()
                ));
                drop(target_lock);
                if send_request(attack_req).await.is_err() {
                    console_error!()
                }
            } else {
                drop(target_lock);
                console_error!();
                delay(10_000).await;
            }
        }
    };
    wasm_bindgen_futures::spawn_local(future);

    html::init(settings, active_settings)?;

    Ok(())
}

#[rustfmt::skip]
#[allow(non_snake_case)]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(thread_local_v2, static_string)]
    static SOURCE_URL: JsString = "data:image/gif;base64,";

    #[wasm_bindgen(thread_local_v2, static_string)]
    static SRC: JsString = "src";
}

async fn fetch_base64_icon_src(url: &str) -> JsResult<JsString> {
    use web_sys::{Request, RequestInit, Response};

    let opts = RequestInit::new();
    opts.set_method(intern("GET"));
    let request = Request::new_with_str_and_init(url, &opts).map_err(map_err!())?;
    let fetch_promise = window().fetch_with_request(&request);
    let response = Response::from(JsFuture::from(fetch_promise).await.map_err(map_err!())?);

    let base64_src = JsFuture::from(response.text().map_err(map_err!())?)
        .await
        .map(JsString::from)
        .map_err(map_err!())?
        .replace(intern("\n"), intern(""))
        .replace(intern(" "), intern(""));

    SOURCE_URL.with(|src_prefix| Ok(JsString::concat(src_prefix, &base64_src)))
}

async fn get_emotion_source(emotion: Emotion) -> JsResult<JsString> {
    if let Some(source_url) = EMOTION_SOURCES
        .with_borrow(|emotion_sources_map| emotion_sources_map.get(&emotion.name).cloned())
    {
        return Ok(source_url);
    }

    let gif64_url = format!(
        "{}{}{}",
        intern("https://micc.garmory-cdn.cloud/gifcache/obrazki/interface/emo/"),
        emotion.name,
        intern(".gif64"),
    );
    let emo_img_src = fetch_base64_icon_src(&gif64_url).await?;

    EMOTION_SOURCES.with_borrow_mut(|emotion_sources_map| {
        emotion_sources_map.insert(emotion.name, emo_img_src.clone())
    });

    Ok(emo_img_src)
}
