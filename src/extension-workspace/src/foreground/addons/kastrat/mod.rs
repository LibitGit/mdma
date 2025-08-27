use std::ops::Deref;

use dominator::events::KeyDown;
use futures::StreamExt;
use futures_signals::map_ref;
use futures_signals::signal::{self, Mutable, MutableLockRef, Signal, SignalExt};
use futures_signals::signal_map::SignalMapExt;
use futures_signals::signal_vec::SignalVecExt;
use proc_macros::{ActiveSettings, Setting, Settings};
use serde::{Deserialize, Serialize};

use crate::bindings::engine::hero::AutoGoToData;
use crate::bindings::engine::types::MapMode;
use crate::color_mark::{Color, ColorMark};
use crate::prelude::*;

mod html;

const ADDON_NAME: AddonName = AddonName::Kastrat;
const TARGET_COLOR: Color = Color::Red;
pub const MIN_DIFF: f64 = 120.0;

#[derive(Clone, Setting)]
struct Level {
    min: Mutable<u16>,
    max: Mutable<u16>,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            min: Mutable::new(1),
            max: Mutable::new(500),
        }
    }
}

impl Level {
    fn contains_signal<B: Signal<Item = u16>>(
        &self,
        other_level: B,
    ) -> impl Signal<Item = bool> + use<B> {
        map_ref! {
            let min = self.min.signal(),
            let max = self.max.signal(),
            let cmp = other_level => {
                (*min..=*max).contains(cmp)
            }
        }
    }
}

//#[derive(Clone, Setting, Default)]
//struct ExclusionList {
//    nicks: NickInput,
//    //clans:
//}

#[derive(Debug, Clone)]
pub struct TargetData {
    ///// Account id of the player.
    //pub account: u32,
    /// Character id of the player.
    pub char_id: OtherId,
    /// List of emotions currently displayed by the player.
    pub emo: Emotions,
    /// Player's current clan if any.
    pub clan: Mutable<Option<Clan>>,
    /// Current level of the player.
    pub lvl: Mutable<u16>,
    /// Current operational level of the player.
    /// If the player's level is <= 300 the operational level will have the exact same value.
    pub operational_lvl: Mutable<u16>,
    /// Player's in-game nick.
    pub nick: Mutable<String>,
    /// Player's in-game profession.
    pub prof: Mutable<Profession>,
    /// Hero's relation relative to the player.
    pub relation: Mutable<Relation>,
    ///// Specifies whether the player is currently AFK.
    //pub stasis: Mutable<bool>,
    /// Player's current x-axis coordinate relative to the left border of the map.
    pub x: Mutable<Option<u8>>,
    /// Player's current y-axis coordinate relative to the top border of the map.
    pub y: Mutable<Option<u8>>,
    /// Specifies if the player is currently on the wanted list.
    pub wanted: Mutable<bool>,
}

impl TargetData {
    const ATTACK_RANGE: f64 = 2.9;

    fn observe_other(&self, other: Other) {
        let this_x = self.x.clone();
        let future = other.x.signal().for_each(move |new_x| {
            this_x.set_neq(Some(new_x));
            debug_log!("setting x:", new_x);
            async {}
        });
        wasm_bindgen_futures::spawn_local(future);

        let this_y = self.y.clone();
        let future = other.y.signal().for_each(move |new_y| {
            this_y.set_neq(Some(new_y));
            debug_log!("setting y:", new_y);
            async {}
        });
        wasm_bindgen_futures::spawn_local(future);
    }

    fn is_wanted(&self) -> bool {
        self.wanted.get()
    }

    /// Specifies whether the target is currently in a fight.
    /// This is done by checking it's emotions vector.
    pub fn in_battle(&self) -> bool {
        self.emo
            .lock_ref()
            .iter()
            .any(|emotion| emotion.name == EmotionName::Battle)
    }

    ///// Specifies whether the target is a friend | clan | clan ally | fraction ally.
    //pub fn is_friendly(&self) -> bool {
    //    matches!(
    //        self.relation.get(),
    //        Relation::Friend | Relation::Clan | Relation::ClanAlly | Relation::FractionAlly
    //    )
    //}

    pub(crate) fn in_attack_range_signal(&self) -> impl Signal<Item = bool> + use<> {
        let hero = Hero::get();
        map_ref! {
            let hero_x = hero.x.signal(),
            let hero_y = hero.y.signal(),
            let target_x = self.x.signal(),
            let target_y = self.y.signal() => {
                if let Some(target_x) = *target_x
                    && let Some(target_y) = *target_y
                    && let Some(hero_x) = *hero_x
                    && let Some(hero_y) = *hero_y
                {
                    let distance = (target_x as f64 - hero_x as f64).hypot(target_y as f64 - hero_y as f64);
                    debug_log!(target_x, target_y, distance);
                    distance < Self::ATTACK_RANGE
                } else {
                    false
                }
            }
        }.dedupe()
    }

    pub(crate) fn coords_signal(&self) -> impl Signal<Item = Option<(u8, u8)>> + use<> {
        map_ref! {
            let target_x = self.x.signal(),
            let target_y = self.y.signal() => {
                target_x.zip(*target_y)
            }
        }
        .dedupe()
    }

    pub(crate) fn in_attack_range(&self) -> bool {
        let hero = Hero::get();

        if let Some(target_x) = self.x.get()
            && let Some(target_y) = self.y.get()
            && let Some(hero_x) = hero.x.get()
            && let Some(hero_y) = hero.y.get()
        {
            let distance = (target_x as f64 - hero_x as f64).hypot(target_y as f64 - hero_y as f64);
            return distance < Self::ATTACK_RANGE;
        }

        false
    }
}

impl PartialEq for TargetData {
    fn eq(&self, other: &Self) -> bool {
        self.char_id == other.char_id
    }
}

impl PartialEq<Other> for TargetData {
    fn eq(&self, other: &Other) -> bool {
        self.char_id == other.char_id
    }
}

impl From<Other> for TargetData {
    fn from(value: Other) -> Self {
        let Other {
            //account,
            char_id,
            emo,
            clan,
            lvl,
            operational_lvl,
            nick,
            prof,
            relation,
            //stasis,
            x,
            y,
            wanted,
            ..
        } = value;

        Self {
            //account,
            char_id,
            emo,
            clan,
            lvl,
            operational_lvl,
            nick,
            prof,
            relation,
            //stasis,
            x: Mutable::new(Some(x.get())),
            y: Mutable::new(Some(y.get())),
            wanted,
        }
    }
}

// TODO: Move `Target` struct into a separate module
#[derive(Default, Clone)]
pub(crate) struct Target(Mutable<Option<TargetData>>);

impl Target {
    pub fn init(&'static self) {
        let future = self
            .signal_ref(|t| t.as_ref().map(|t| t.char_id))
            .switch(move |id_opt| {
                signal::option(
                    id_opt.map(|char_id| Others::get().signal_map_cloned().key_cloned(char_id)),
                )
                .map(Option::flatten)
            })
            .dedupe_cloned()
            .for_each(|other_opt| {
                // debug_log!("NEW OBSERVE", other_opt.is_some());
                if let Some(target_data) = self.lock_ref().deref() {
                    match other_opt {
                        Some(other) => target_data.observe_other(other),
                        None => {
                            target_data.emo.clear();
                            target_data.x.set_neq(None);
                            target_data.y.set_neq(None);
                        }
                    }
                }

                async {}
            });
        wasm_bindgen_futures::spawn_local(future);
    }

    pub(crate) fn set(&self, other_data: Other, addon_name: AddonName) {
        other_data
            .init_color_mark(TARGET_COLOR, addon_name)
            .unwrap_js();
        #[cfg(feature = "ni")]
        let char_id = other_data.char_id;
        let old_target_opt = self.0.replace(Some(other_data.into()));

        if let Some(old_target) = old_target_opt.as_ref() {
            #[cfg(feature = "ni")]
            get_engine()
                .targets()
                .unwrap_js()
                .delete_arrow(&format!("Other-{}", old_target.char_id))
                .unwrap_js();
            ColorMark::remove(TARGET_COLOR, addon_name, old_target.char_id);
        }

        #[cfg(feature = "ni")]
        {
            if old_target_opt.is_none_or(|target| target.char_id != char_id) {
                get_engine()
                    .targets()
                    .unwrap_js()
                    .delete_arrow(&format!("Other-{}", char_id))
                    .unwrap_js();
            }

            let other = get_engine()
                .others()
                .unwrap_js()
                .get_by_id(char_id)
                .unwrap_js();
            get_engine()
                .targets()
                .unwrap_js()
                .add_arrow(false, &other.d().nick(), &other, "Other", "attack")
                .unwrap_js();
        }
    }

    pub(crate) fn clear(&self, addon_name: AddonName) -> Option<TargetData> {
        let old_target = self.0.replace(None);

        if let Some(old_target) = old_target.as_ref() {
            get_engine().idle_json().unwrap_js().set_default_diff();
            #[cfg(feature = "ni")]
            get_engine()
                .targets()
                .unwrap_js()
                .delete_arrow(&format!("Other-{}", old_target.char_id))
                .unwrap_js();
            ColorMark::remove(TARGET_COLOR, addon_name, old_target.char_id);
        }

        old_target
    }

    pub(crate) fn signal_cloned(&self) -> impl Signal<Item = Option<TargetData>> {
        self.0.signal_cloned()
    }

    pub(crate) fn signal_ref<B, F>(&self, f: F) -> impl Signal<Item = B>
    where
        F: FnMut(&Option<TargetData>) -> B,
    {
        self.0.signal_ref(f)
    }

    pub fn active_signal(&self) -> impl Signal<Item = bool> {
        self.signal_ref(Option::is_some).dedupe()
    }

    //pub(crate) fn lock_mut(&self) -> MutableLockMut<'_, Option<TargetData>> {
    //    self.0.lock_mut()
    //}

    pub(crate) fn lock_ref(&self) -> MutableLockRef<'_, Option<TargetData>> {
        self.0.lock_ref()
    }

    pub(crate) fn is_wanted(&self) -> bool {
        self.0
            .lock_ref()
            .as_ref()
            .is_some_and(|other| other.is_wanted())
    }

    pub(crate) fn set_candidate(
        &self,
        best_candidate: Option<(Other, f64, (f64, f64))>,
        addon_name: AddonName,
    ) {
        let Some((candidate, candidate_distance, (hero_x, hero_y))) = best_candidate else {
            if self.lock_ref().is_some() {
                self.clear(ADDON_NAME);
            }
            // debug_log!("CLEARING TARGET");
            return;
        };

        if let Some(target_data) = self.lock_ref().as_ref()
            && let Some(target_x) = target_data.x.get()
            && let Some(target_y) = target_data.y.get()
        {
            // If this isn't here replacing the Target with the same candidate
            // breaks coords updating since Target::init dedupes on the same charId.
            if candidate.char_id == target_data.char_id {
                return;
            }

            let target_distance = (target_x as f64 - hero_x).hypot(target_y as f64 - hero_y);

            if candidate_distance >= target_distance {
                return;
            }
        }

        debug_log!("SETTING TARGET", candidate.nick.get_cloned().as_str());
        self.set(candidate, addon_name);
    }
}

#[derive(Default, Deserialize, Serialize)]
struct HotkeyValue {
    value: String,
    alt_key: bool,
    ctrl_key: bool,
    shift_key: bool,
}

impl HotkeyValue {
    fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
            alt_key: false,
            ctrl_key: false,
            shift_key: true,
        }
    }
}

impl window_events::Hotkey for HotkeyValue {
    fn value(&self) -> &str {
        &self.value
    }

    fn alt_key(&self) -> bool {
        self.alt_key
    }

    fn ctrl_key(&self) -> bool {
        self.ctrl_key
    }

    fn shift_key(&self) -> bool {
        self.shift_key
    }
}

#[derive(Clone, Setting)]
struct Hotkey {
    value: Mutable<HotkeyValue>,
    active: Mutable<bool>,
}

impl Hotkey {
    fn new(value: &str) -> Self {
        Self {
            value: Mutable::new(HotkeyValue::new(value)),
            active: Mutable::new(false),
        }
    }
}

#[derive(Settings)]
struct Settings {
    /// Determines whether wanted players are valid targets on MapMode::AgreePvp.
    wanted_targetting: Mutable<bool>,
    attack_toggle_hotkey: Hotkey,
    track_hotkey: Hotkey,
    msg: Mutable<bool>,
    /// Whether track button is visible.
    track_button: Mutable<bool>,
    //exclusion_list: ExclusionList,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wanted_targetting: Mutable::new(false),
            attack_toggle_hotkey: Hotkey::new("X"),
            track_hotkey: Hotkey::new("L"),
            msg: Mutable::new(false),
            track_button: Mutable::new(true),
        }
    }
}

impl Settings {
    fn init(&self, active_settings: &ActiveSettings, event: KeyDown) {
        if !Addons::is_active(ADDON_NAME) {
            return;
        }

        if self.track_hotkey.active.get() {
            match window_events::validate_keydown_event(
                &event,
                self.track_hotkey.value.lock_ref().deref(),
            ) {
                Ok(true) => {
                    let target_lock = active_settings.target.lock_ref();
                    let Some(target) = target_lock.as_ref() else {
                        return;
                    };

                    if let Some(target_x) = target.x.get()
                        && let Some(target_y) = target.y.get()
                    {
                        let dest = AutoGoToData::new(target_x, target_y);
                        let _ = message(&format!(
                            "[MDMA::RS] Podchodzę do \"{}\"...",
                            target.nick.lock_ref().deref()
                        ));

                        get_engine()
                            .hero()
                            .unwrap_js()
                            .auto_go_to(&dest)
                            .unwrap_js();
                    }

                    event.prevent_default();
                    event.stop_immediate_propagation();
                    return;
                }
                Ok(false) => {}
                Err(err_code) => console_error!(err_code),
            }
        }
        if self.attack_toggle_hotkey.active.get() {
            match window_events::validate_keydown_event(
                &event,
                self.attack_toggle_hotkey.value.lock_ref().deref(),
            ) {
                Ok(true) => {
                    let old = active_settings.attack_toggle.replace_with(|old| !*old);
                    let msg = match old {
                        true => "[MDMA::RS] Wyłączono automatyczne atakowanie...",
                        false => "[MDMA::RS] Włączono automatyczne atakowanie...",
                    };
                    let _ = message(msg);
                    event.prevent_default();
                    event.stop_immediate_propagation();
                }
                Ok(false) => {}
                Err(err_code) => console_error!(err_code),
            }
        }
    }

    fn can_attack_on_map(&self, target: &Target, map_mode: MapMode) -> bool {
        #[cfg(debug_assertions)]
        if map_mode == MapMode::Arena {
            return true;
        }
        if target.is_wanted() && map_mode == MapMode::AgreePvp {
            return self.wanted_targetting.get();
        }

        matches!(
            map_mode,
            MapMode::Pvp | MapMode::InstanceSolo | MapMode::InstanceGrp
        )
    }
}

trait Targetable {
    fn targetable_signal(
        &self,
        active_settings: &ActiveSettings,
    ) -> impl Signal<Item = bool> + use<Self>;
}

impl Targetable for Other {
    fn targetable_signal(
        &self,
        active_settings: &ActiveSettings,
    ) -> impl Signal<Item = bool> + use<> {
        let not_in_battle = self
            .emo
            .signal_vec()
            .filter(|emotion| emotion.name == EmotionName::Battle)
            .is_empty();
        let char_id = self.char_id;
        let not_from_party = Party::get()
            .signal_vec_keys()
            .filter(move |&member_id| member_id == char_id)
            .is_empty();

        map_ref! {
            let from_lvl = active_settings.lvl.contains_signal(self.lvl.signal()),
            let not_in_battle = not_in_battle,
            let not_friendly = signal::not(self.friendly_signal()),
            let not_from_party = not_from_party => {
                *from_lvl && *not_in_battle && *not_friendly && *not_from_party
            }
        }
    }
}

#[derive(ActiveSettings)]
struct ActiveSettings {
    lvl: Level,
    attack_toggle: Mutable<bool>,
    #[setting(skip)]
    target: Target,
}

impl Default for ActiveSettings {
    fn default() -> Self {
        Self {
            attack_toggle: Mutable::new(true),
            lvl: Level::default(),
            //exclusion_list: ExclusionList::default(),
            target: Target::default(),
            //msg: Mutable::new(true),
        }
    }
}

pub(crate) fn init() -> JsResult<()> {
    let active_settings = ActiveSettings::new(ADDON_NAME);
    active_settings.target.init();
    let settings = Settings::new(ADDON_NAME);

    // PRECEDENCE:
    // 1. settings/active_settings change
    // 2. new_other/hero coords change
    // 3. other coords change
    // 4. filter out untargetable
    // 5. min_by distance
    let future = Addons::active_signal(ADDON_NAME)
        .ok_or_else(|| err_code!())?
        .switch(|addon_active| {
            active_settings
                .attack_toggle
                .signal()
                .map(move |attack_active| addon_active && attack_active)
        })
        .dedupe()
        .switch(|active| {
            Hero::get()
                .coords_signal()
                .map(move |coords| active.then_some(coords).flatten())
        })
        .dedupe()
        .switch(move |hero_coords| {
            let Some((hero_x, hero_y)) = hero_coords else {
                return signal::always(None).boxed_local();
            };
            let (hero_x, hero_y) = (hero_x as f64, hero_y as f64);

            Others::get()
                .entries_cloned()
                .filter_signal_cloned(move |(_, other)| other.targetable_signal(active_settings))
                .map_signal(|(_, other)| other.coords_signal().map(move |_| other.clone()))
                .to_signal_cloned()
                .map(move |others| {
                    debug_log!("NEW OTHERS");
                    // This is to prevent skipping set_candidate if current target leaves the map
                    // and there are no players closer than the target was.
                    {
                        let target_lock = active_settings.target.lock_ref();
                        if let Some(target_id) = target_lock.as_ref().map(|t| t.char_id) {
                            if !others.iter().any(|other| other.char_id == target_id) {
                                drop(target_lock);
                                active_settings.target.clear(ADDON_NAME);
                            }
                        }
                    }

                    others
                        .into_iter()
                        .map(|candidate| {
                            let distance = (candidate.x.get() as f64 - hero_x)
                                .hypot(candidate.y.get() as f64 - hero_y);
                            (candidate, distance)
                        })
                        .min_by(|(_, distance_a), (_, distance_b)| distance_a.total_cmp(distance_b))
                        .map(|(other, dist)| (other, dist, (hero_x, hero_y)))
                })
                .boxed_local()
        })
        .for_each(move |best_candidate| {
            active_settings
                .target
                .set_candidate(best_candidate, ADDON_NAME);
            async {}
        });
    wasm_bindgen_futures::spawn_local(future);

    let future = async move {
        loop {
            let future = active_settings
                .target
                .signal_ref(move |value| {
                    signal::option(value.as_ref().map(TargetData::in_attack_range_signal)).map(
                        |in_range| {
                            if in_range.is_none() {
                                // debug_log!("RANGE IS NONE");
                            }
                            if let None | Some(false) = in_range {
                                // debug_log!("NOT IN RANGE");
                                return false;
                            }
                            let map_mode = get_engine()
                                .map()
                                .unwrap_js()
                                .map_data()
                                .unwrap_js()
                                .get_pvp()
                                .unwrap_or(MapMode::NonPvp);
                            if !settings.can_attack_on_map(&active_settings.target, map_mode) {
                                // debug_log!("CANT ATTACK ON MAP");
                                return false;
                            }
                            if Hero::in_battle() {
                                // debug_log!("HERO IN BATTLE");
                                return false;
                            }

                            true
                        },
                    )
                })
                .flatten()
                .dedupe()
                .wait_for(true);
            future.await;

            if settings.msg.get() {
                let _ = message(&format!(
                    "[MDMA::RS] Atakuję \"{}\"...",
                    active_settings
                        .target
                        .lock_ref()
                        .as_ref()
                        .unwrap_js()
                        .nick
                        .lock_ref()
                        .deref()
                ));
            }

            let mut attack_req = string!("fight&a=attack&id=");
            attack_req.push_str(
                active_settings
                    .target
                    .lock_ref()
                    .as_ref()
                    .unwrap_js()
                    .char_id
                    .to_string()
                    .as_str(),
            );
            if send_request(attack_req).await.is_err() {
                console_error!()
            }
        }
    };
    wasm_bindgen_futures::spawn_local(future);

    let future = active_settings
        .target
        .signal_ref(move |value| {
            signal::option(value.as_ref().map(TargetData::in_attack_range_signal))
                .map(|jd| jd.is_some_and(|cond| cond))
        })
        .flatten()
        .dedupe()
        .for_each(move |attacking| {
            match attacking && Addons::is_active(ADDON_NAME) && active_settings.attack_toggle.get()
            {
                true => get_engine().idle_json().unwrap_js().set_diff(MIN_DIFF),
                false => get_engine().idle_json().unwrap_js().set_default_diff(),
            }

            async {}
        });
    wasm_bindgen_futures::spawn_local(future);

    let future = Addons::active_signal(ADDON_NAME)
        .ok_or_else(|| err_code!())?
        .to_stream()
        .skip(1)
        .for_each(move |active| {
            let is_attacking = active_settings
                .target
                .lock_ref()
                .deref()
                .as_ref()
                .is_some_and(|target| target.in_attack_range())
                && active_settings.attack_toggle.get();

            match active && is_attacking {
                true => get_engine().idle_json().unwrap_js().set_diff(MIN_DIFF),
                false => get_engine().idle_json().unwrap_js().set_default_diff(),
            }

            async {}
        });
    wasm_bindgen_futures::spawn_local(future);

    html::init(settings, active_settings)
}
