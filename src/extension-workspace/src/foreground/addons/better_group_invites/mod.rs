//TODO: Add invite cycle that gets all current others and excludes the ones that were already
//invited (idk what to do then), didn't accept the invite and are in group <- for invite by professions.
//Add IDs to inviting states because now when someone changes the delay from bigger to smaller and
//interrupts the process the smaller one might get interrupted
//TODO: If button clicked and there are matches missign fetch data again (?)
//TODO: Update no player message when trying to invite with stasis == true
mod html;

use std::cell::RefCell;
use std::cmp::PartialOrd;
use std::collections::hash_set::Iter;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::iter::Filter;
use std::ops::Deref;

use dominator::events::{Click, Input as InputEvent, KeyDown};
use futures_signals::signal::{Mutable, Signal, SignalExt};
use proc_macros::{ActiveSettings, Setting, Settings};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlInputElement};

use crate::addon_window::ui_components::{ChangeValidity, NickInput};
use crate::bindings::engine::communication;
use crate::prelude::*;

const ADDON_NAME: AddonName = AddonName::BetterGroupInvites;
const ACTIVE_MESSAGE: &str = "Skróty klawiszowe nie mogą być jednocześnie aktywne, jeśli mają przypisane takie same wartości!";

// TODO: Maybe make the fields clones of Mutable<T> ?
struct Candidate {
    profession: Profession,
    relation: Relation,
    nick: String,
    lvl: u16,
    emo: Vec<Emotion>,
    x: Option<u8>,
    y: Option<u8>,
}

impl Candidate {
    fn next_to_hero(&self) -> bool {
        let hero = Hero::get();
        if let Some(hero_x) = hero.x.get()
            && let Some(hero_y) = hero.y.get()
            && let Some(candidate_x) = self.x
            && let Some(candidate_y) = self.y
        {
            return (hero_x as i32 - candidate_x as i32).abs() <= 1
                && (hero_y as i32 - candidate_y as i32).abs() <= 1;
        }

        false
    }
}

impl From<&Other> for Candidate {
    fn from(value: &Other) -> Self {
        Self {
            profession: value.prof.get(),
            relation: value.relation.get(),
            nick: value.nick.get_cloned(),
            lvl: value.lvl.get(),
            emo: value.emo.lock_ref().to_vec(),
            x: Some(value.x.get()),
            y: Some(value.y.get()),
        }
    }
}

impl From<&Peer> for Candidate {
    fn from(value: &Peer) -> Self {
        Self {
            profession: value.prof.get(),
            relation: value.relation.get(),
            nick: value.nick.get_cloned(),
            lvl: value.lvl.get(),
            emo: Vec::new(),
            x: value.x.get(),
            y: value.y.get(),
        }
    }
}

#[derive(Clone, Copy)]
enum Range {
    Maximum,
    Minimum,
}

struct LinkedInput<T: Copy + Clone> {
    value: Mutable<T>,
    root: Mutable<Option<HtmlInputElement>>,
    range: Range,
    custom_validity: Mutable<String>,
}

impl<T> SettingOption for LinkedInput<T>
where
    T: Copy + Clone + serde::Serialize + FromF64 + Display + PartialOrd + 'static,
{
    fn as_option_signal(&self, f: fn(Value) -> Value) -> impl Signal<Item = Value> {
        self.value.signal_ref(move |data| f(json!(data)))
    }
}

impl<T> SettingFromValue for LinkedInput<T>
where
    T: Copy + Clone + serde::Serialize + FromF64 + Display + PartialOrd + 'static,
{
    fn update(&self, value: Value) {
        if let Some(value) = value.as_f64() {
            self.value.replace(T::from_f64(value));
        }
    }
}

impl<T: Copy + Clone> LinkedInput<T> {
    fn new(value: T, range: Range) -> Self {
        Self {
            value: Mutable::new(value),
            root: Mutable::default(),
            range,
            custom_validity: Mutable::default(),
        }
    }
}

trait FromF64 {
    fn from_f64(value: f64) -> Self
    where
        Self: Sized;
}

impl FromF64 for u16 {
    fn from_f64(value: f64) -> Self {
        value as u16
    }
}

impl FromF64 for u32 {
    fn from_f64(value: f64) -> Self {
        value as u32
    }
}

impl<T> LinkedInput<T>
where
    T: Clone + Copy + FromF64 + Display + PartialOrd + 'static,
{
    #[inline]
    fn on_input_factory(
        first: &'static LinkedInput<T>,
        second: &'static LinkedInput<T>,
    ) -> impl FnMut(InputEvent, &HtmlInputElement) + 'static {
        #[inline]
        move |_, input_elem| {
            if let Some(value) = Self::validate_value(
                &first.custom_validity,
                first.range,
                input_elem,
                second.value.get(),
            ) {
                first.custom_validity.set_valid(input_elem);
                first.value.set(value);

                let Some(root) = second.root.get_cloned() else {
                    return console_error!();
                };

                if let Some(other_value) = Self::validate_value(
                    &second.custom_validity,
                    second.range,
                    &root,
                    first.value.get(),
                ) {
                    second.custom_validity.set_valid(&root);
                    second.value.set(other_value);
                }
            }
        }
    }

    #[inline]
    fn validate_value(
        first_validity: &Mutable<String>,
        first_range: Range,
        first_root: &HtmlInputElement,
        second_value: T,
    ) -> Option<T> {
        first_validity.set_valid(first_root);

        let value = first_root.value();

        if value.starts_with("0") && value.len() > 1 {
            first_validity.set_invalid(first_root, String::from("Nieprawidłowa wartość!"));
            return None;
        }

        let validation_error = match first_root.validity() {
            validity if validity.range_overflow() => Some(match first_root.max() {
                max if max.is_empty() => "Przekroczono maksymalną wartość!".to_string(),
                max => format!("Maksymalna wartość wynosi {}!", max),
            }),
            validity if validity.range_underflow() => Some(match first_root.min() {
                min if min.is_empty() => "Przekroczono minimalną wartość!".to_string(),
                min => format!("Minimalna wartość wynosi {}!", min),
            }),
            validity if validity.valid() => None,
            _ => Some(String::from("Nieprawidłowa wartość!")),
        };

        if let Some(validation_error) = validation_error {
            debug_log!(&validation_error);

            first_validity.set_invalid(first_root, validation_error);
            return None;
        }
        if value.is_empty() {
            return None;
        }

        let value = T::from_f64(first_root.value_as_number());
        let error_msg = match first_range {
            Range::Maximum if value < second_value => "mniejsza od wartości minimalnej",
            Range::Minimum if value > second_value => "większa od wartości maksymalnej",
            _ => return Some(value), // value passed validation checks
        };

        let validation_error = format!("Wartość nie może być {} ({})!", error_msg, second_value);
        first_validity.set_invalid(first_root, validation_error);

        None
    }
}

#[derive(Setting)]
struct Delay {
    min: LinkedInput<u32>,
    max: LinkedInput<u32>,
}

impl Delay {
    fn new(min: u32, max: u32) -> Self {
        Self {
            min: LinkedInput::new(min, Range::Minimum),
            max: LinkedInput::new(max, Range::Maximum),
        }
    }
}

#[derive(Setting)]
struct Hotkey {
    value: Mutable<String>,
    active: Mutable<bool>,
}

impl Hotkey {
    fn new(value: &str, active: bool) -> Self {
        Self {
            value: Mutable::new(value.to_owned()),
            active: Mutable::new(active),
        }
    }

    #[inline]
    fn on_key_down_factory(
        first: &'static Self,
        second: &'static Self,
    ) -> impl FnMut(KeyDown, &HtmlInputElement) + 'static {
        #[inline]
        move |event: KeyDown, input_elem: &HtmlInputElement| {
            let value = event.key().to_ascii_uppercase();

            if value.chars().count() > 1 {
                if value == "ESCAPE" {
                    if let Err(_err) = input_elem.blur() {
                        debug_log!(_err);
                        console_error!();
                    }
                }

                event.prevent_default();
                event.stop_propagation();
                return;
            }
            if value.is_empty() || value.chars().any(|c| !c.is_ascii_alphabetic()) {
                return;
            }
            if value == second.value.get_cloned() && second.active.get() && first.active.get() {
                second.active.set(false);
                message(ACTIVE_MESSAGE).unwrap_js();
            }
            if let Err(_err) = input_elem.blur() {
                debug_log!(_err);
                console_error!();
            }

            event.prevent_default();
            event.stop_propagation();
            input_elem.set_value(&value);
            first.value.set(value);
        }
    }

    fn on_click_factory(
        first: &'static Self,
        second: &'static Self,
    ) -> impl FnMut(Click) + 'static {
        move |_| {
            if second.value.get_cloned() == first.value.get_cloned() {
                message(ACTIVE_MESSAGE).unwrap_js();
                second.active.set(false);
            }

            first.active.set(!first.active.get());
        }
    }
}

#[derive(Setting)]
struct Relations {
    none: Mutable<bool>,
    friend: Mutable<bool>,
    clan: Mutable<bool>,
    clan_ally: Mutable<bool>,
    fraction_ally: Mutable<bool>,
}

impl Relations {
    fn new(none: bool, friend: bool, clan: bool, clan_ally: bool, fraction_ally: bool) -> Self {
        Self {
            none: Mutable::new(none),
            friend: Mutable::new(friend),
            clan: Mutable::new(clan),
            clan_ally: Mutable::new(clan_ally),
            fraction_ally: Mutable::new(fraction_ally),
        }
    }
}

#[derive(Setting)]
struct FromPeers {
    from_location: Mutable<bool>,
    clan: Mutable<bool>,
    friend: Mutable<bool>,
}

impl FromPeers {
    fn new(from_location: bool, clan: bool, friend: bool) -> Self {
        Self {
            from_location: Mutable::new(from_location),
            clan: Mutable::new(clan),
            friend: Mutable::new(friend),
        }
    }
}

#[derive(Setting)]
struct MassInvite {
    hotkey: Hotkey,
    peers: FromPeers,
}

impl MassInvite {
    fn new(hotkey: Hotkey, peers: FromPeers) -> Self {
        Self { hotkey, peers }
    }
}

#[derive(Settings)]
struct Settings {
    delay: Delay,
    excluded_nicks: NickInput,
    hotkey: Hotkey,
    //TODO: Use Cell instead ?
    #[setting(skip)]
    inviting: Mutable<bool>,
    #[setting(skip)]
    interrupt: RefCell<Vec<()>>,
    mass_invite: MassInvite,
    relations: Relations,
}

impl Default for Settings {
    fn default() -> Self {
        let mass_invite_hotkey = Hotkey::new(s!("B"), true);
        let mass_invite_peers = FromPeers::new(false, true, true);
        Self {
            delay: Delay::new(150, 200),
            excluded_nicks: NickInput::default(),
            hotkey: Hotkey::new(s!("V"), true),
            inviting: Mutable::new(false),
            interrupt: RefCell::new(Vec::new()),
            mass_invite: MassInvite::new(mass_invite_hotkey, mass_invite_peers),
            relations: Relations::new(false, true, true, true, true),
        }
    }
}

impl Settings {
    #[inline]
    fn shake_on_unsaved(window_type: WindowType) -> impl FnMut(Click) + 'static {
        #[inline]
        move |_| {
            let Some(addon_data) = Addons::get_addon(ADDON_NAME) else {
                return console_error!();
            };
            let root_lock = addon_data.get(window_type).root.borrow();
            let Some(root) = root_lock.as_ref() else {
                return console_error!();
            };
            let Ok(invalid_inputs) = root.query_selector_all("input:invalid") else {
                return console_error!();
            };

            if invalid_inputs.length() == 0 {
                let settings_window_status = &addon_data.settings_window.active;
                return settings_window_status.set(!settings_window_status.get());
            }

            for input_root in invalid_inputs.values() {
                let Ok(input_root) = input_root.and_then(|root| root.dyn_into()) else {
                    return console_error!();
                };

                Self::trigger_animation(input_root, "warning-glow", 500);
            }

            Self::trigger_animation(root.clone().into(), "shake", 300);
        }
    }

    #[inline]
    fn trigger_animation(elem: Element, animation_class: &'static str, remove_after: u32) {
        if let Err(_err) = elem.class_list().add_1(animation_class) {
            debug_log!(_err);
            console_error!();
        }

        wasm_bindgen_futures::spawn_local(async move {
            delay(remove_after).await;

            if let Err(_err) = elem.class_list().remove_1(animation_class) {
                debug_log!(_err);
                console_error!();
            }
        })
    }
}

#[derive(Default, Setting)]
struct ProfessionSetting {
    value: Mutable<u8>,
    active: Mutable<bool>,
    #[setting(skip)]
    custom_validity: Mutable<String>,
    #[setting(skip)]
    root: Mutable<Option<HtmlInputElement>>,
}

impl ProfessionSetting {
    fn on_input_factory(
        professions: &'static Professions,
        target_prof: &'static Self,
    ) -> impl FnMut(InputEvent, &HtmlInputElement) + 'static {
        #[inline]
        move |_, input_elem| {
            let ProfessionSetting {
                custom_validity,
                value,
                active,
                ..
            } = &target_prof;
            custom_validity.set_valid(input_elem);

            let input_value = input_elem.value();

            if input_value.starts_with("0") && input_value.len() > 1 {
                professions.turn_off_checkboxes();
                return custom_validity
                    .set_invalid(input_elem, String::from("Nieprawidłowa wartość!"));
            }

            let validation_error = match input_elem.validity() {
                validity if validity.range_overflow() => Some(match input_elem.max() {
                    max if max.is_empty() => "Przekroczono maksymalną wartość!".to_string(),
                    max => format!("Maksymalna wartość wynosi {}!", max),
                }),
                validity if validity.range_underflow() => Some(match input_elem.min() {
                    min if min.is_empty() => "Przekroczono minimalną wartość!".to_string(),
                    min => format!("Minimalna wartość wynosi {}!", min),
                }),
                validity if validity.valid() => None,
                _ => Some(String::from("Nieprawidłowa wartość!")),
            };

            if let Some(validation_error) = validation_error {
                debug_log!(&validation_error);
                professions.turn_off_checkboxes();
                return custom_validity.set_invalid(input_elem, validation_error);
            }
            if input_value.is_empty() {
                return;
            }

            let input_value = input_elem.value_as_number() as u8;

            value.set_neq(input_value);

            if active.get() && !professions.below_party_limit(&target_prof) {
                professions.turn_off_checkboxes();
                message("Maksymalna liczba zaproszeń to 9!").unwrap_js();
            }
        }
    }
}

#[derive(Default, Setting)]
struct Professions {
    active: Mutable<bool>,
    warrior: ProfessionSetting,
    mage: ProfessionSetting,
    paladin: ProfessionSetting,
    hunter: ProfessionSetting,
    tracker: ProfessionSetting,
    blade_dancer: ProfessionSetting,
}

struct ProfessionsIter<'a> {
    data: &'a Professions,
    index: usize,
}

impl<'a> Iterator for ProfessionsIter<'a> {
    type Item = (Profession, &'a ProfessionSetting);

    fn next(&mut self) -> Option<Self::Item> {
        use Profession::*;

        let ret = match self.index {
            0 => Some((Warrior, &self.data.warrior)),
            1 => Some((Mage, &self.data.mage)),
            2 => Some((Paladin, &self.data.paladin)),
            3 => Some((Hunter, &self.data.hunter)),
            4 => Some((Tracker, &self.data.tracker)),
            5 => Some((BladeDancer, &self.data.blade_dancer)),
            _ => None,
        };
        self.index += 1;

        ret
    }
}

impl Professions {
    fn iter(&self) -> ProfessionsIter<'_> {
        ProfessionsIter {
            data: self,
            index: 0,
        }
    }

    fn below_party_limit(&self, target_prof: &ProfessionSetting) -> bool {
        let ProfessionSetting {
            value,
            custom_validity,
            ..
        } = target_prof;

        if !custom_validity.get_cloned().is_empty() {
            return false;
        }

        let mut sum = 0;

        for (_, profession) in self.iter() {
            debug_log!(sum);
            if profession.active.get() {
                sum += profession.value.get();
            }
        }

        sum + value.get() < 10
    }

    fn on_click_factory(
        professions: &'static Self,
        target_prof: &'static ProfessionSetting,
    ) -> impl FnMut(Click) + 'static {
        #[inline]
        move |event| {
            let ProfessionSetting {
                active,
                custom_validity,
                ..
            } = &target_prof;

            if active.get() {
                return active.set(false);
            }

            let validity_error = custom_validity.get_cloned();

            if !validity_error.is_empty() {
                event.prevent_default();
                event.stop_immediate_propagation();
                message(&validity_error).unwrap_js();
                return;
            }

            match professions.below_party_limit(&target_prof) {
                true => active.set_neq(true),
                false => {
                    event.prevent_default();
                    event.stop_immediate_propagation();
                    message("Maksymalna liczba zaproszeń to 9!").unwrap_js();
                }
            }
        }
    }

    fn turn_off_checkboxes(&self) {
        let Professions {
            warrior,
            mage,
            paladin,
            hunter,
            tracker,
            blade_dancer,
            ..
        } = self;
        let professions = [warrior, mage, paladin, hunter, tracker, blade_dancer];

        for profession in professions.into_iter() {
            profession.active.set_neq(false);
        }
    }
}

#[derive(Default, Setting)]
struct Nicks {
    active: Mutable<bool>,
    values: NickInput,
}

#[derive(Setting)]
struct LevelRange {
    min: LinkedInput<u16>,
    max: LinkedInput<u16>,
    active: Mutable<bool>,
}

impl Default for LevelRange {
    fn default() -> Self {
        Self {
            min: LinkedInput {
                value: Mutable::new(1),
                root: Mutable::new(None),
                range: Range::Minimum,
                custom_validity: Mutable::new(String::new()),
            },
            max: LinkedInput {
                value: Mutable::new(500),
                root: Mutable::new(None),
                range: Range::Maximum,
                custom_validity: Mutable::new(String::new()),
            },
            active: Mutable::new(false),
        }
    }
}

#[derive(Default, ActiveSettings)]
struct ActiveSettings {
    with_nicks: Nicks,
    with_prof: Professions,
    with_lvl: LevelRange,
}

impl ActiveSettings {
    fn invite_button_onclick_factory(
        active_settings: &'static Self,
        settings: &'static Settings,
    ) -> impl FnMut(Click) + 'static {
        #[inline]
        move |_event: Click| {
            let candidate_ids = get_regular_invite_candidates(&settings);
            let others_lock = Others::get().lock_ref();
            let mut candidates = others_lock
                .iter()
                .filter_map(
                    |(other_id, other_data)| match candidate_ids.contains(other_id) {
                        true => Some((*other_id, other_data.into())),
                        false => None,
                    },
                )
                .collect();

            drop(others_lock);
            let candidates = Self::filter_candidates(&active_settings, &mut candidates);

            wasm_bindgen_futures::spawn_local(async move {
                send_regular_invite_invites(candidates.iter(), &settings).await;
            })
        }
    }

    fn mass_invite_button_onclick_factory(
        active_settings: &'static Self,
        settings: &'static Settings,
    ) -> impl FnMut(Click) + 'static {
        #[inline]
        move |_event: Click| {
            let candidate_ids = get_mass_invite_candidates(&settings);
            let others_lock = Others::get().lock_ref();
            let peers_lock = Peers::get().lock_ref();
            let mut candidates = others_lock
                .iter()
                .filter_map(
                    |(other_id, other_data)| match candidate_ids.contains(other_id) {
                        true => Some((*other_id, other_data.into())),
                        false => None,
                    },
                )
                .chain(peers_lock.iter().filter_map_online(|(peer_id, peer_data)| {
                    match candidate_ids.contains(peer_id) {
                        true => Some((*peer_id, peer_data.into())),
                        false => None,
                    }
                }))
                .collect();

            drop(others_lock);
            drop(peers_lock);
            let candidates = Self::filter_candidates(&active_settings, &mut candidates);

            wasm_bindgen_futures::spawn_local(async move {
                send_mass_invite_invites(candidates.iter(), &settings).await
            });
        }
    }

    fn filter_candidates(
        active_settings: &Self,
        candidates: &mut HashMap<OtherId, Candidate>,
    ) -> HashSet<OtherId> {
        if active_settings.with_prof.active.get() {
            Self::filter_by_relation(&active_settings.with_prof, candidates);
        }
        if active_settings.with_nicks.active.get() {
            Self::filter_by_nick(&active_settings.with_nicks.values.nicks, candidates);
        }
        if active_settings.with_lvl.active.get() {
            Self::filter_by_lvl(
                active_settings.with_lvl.min.value.get(),
                active_settings.with_lvl.max.value.get(),
                candidates,
            )
        }

        candidates.iter().map(|(id, _)| *id).collect()
    }

    fn filter_by_relation(professions: &Professions, candidates: &mut HashMap<OtherId, Candidate>) {
        let mut professions_left: HashMap<Profession, i32> = HashMap::new();

        for (profession, data) in professions.iter() {
            let value = match data.active.get() {
                true => data.value.get() as i32,
                false => 0,
            };
            professions_left.insert(profession, value);
        }

        for (_, party_member_data) in Party::get().lock_ref().iter() {
            let Some(party_member_prof) = party_member_data.profession.get() else {
                continue;
            };
            let Some(profession) = professions_left.get_mut(&party_member_prof) else {
                return console_error!();
            };

            *profession -= 1;
        }

        candidates.retain(|_, candidate| {
            let Some(profession_left) = professions_left.get(&candidate.profession) else {
                console_error!();
                return false;
            };

            *profession_left > 0
        })
    }

    // FIXME: Add handling for polish non-ASCII characters.
    fn filter_by_nick(
        specified_nicks: &Mutable<Vec<String>>,
        candidates: &mut HashMap<OtherId, Candidate>,
    ) {
        let specified_nicks_lock = specified_nicks.lock_ref();

        candidates.retain(|_, candidate| {
            specified_nicks_lock
                .iter()
                .any(|specified_nick| specified_nick.eq_ignore_ascii_case(&candidate.nick))
        });
    }

    fn filter_by_lvl(min: u16, max: u16, others: &mut HashMap<OtherId, Candidate>) {
        others.retain(|_, candidate| candidate.lvl >= min && candidate.lvl <= max)
    }
}

pub(crate) fn init() -> JsResult<()> {
    let settings = Settings::new(ADDON_NAME);
    let active_settings = ActiveSettings::new(ADDON_NAME);

    try_add_listener(settings)?;
    html::init(settings, active_settings)
}

#[derive(PartialEq)]
enum InviteType {
    Regular,
    Mass,
    Null,
}

impl InviteType {
    fn get(event: &web_sys::KeyboardEvent, settings: &Settings) -> JsResult<Self> {
        use crate::utils::window_events::verify_keyboard_event;

        if settings.hotkey.active.get()
            && verify_keyboard_event(event, &settings.hotkey.value.get_cloned())?
        {
            return Ok(Self::Regular);
        }
        if settings.mass_invite.hotkey.active.get()
            && verify_keyboard_event(event, &settings.mass_invite.hotkey.value.get_cloned())?
        {
            return Ok(Self::Mass);
        }

        Ok(Self::Null)
    }
}

// TODO: Remove listener on addon toggle instead of checking if active here ?
// TODO: Make this an associated function with a better name.
fn try_add_listener(settings: &'static Settings) -> JsResult<()> {
    use crate::utils::window;
    use common::closure;

    let better_group_invites_listener = closure!(move |event: web_sys::KeyboardEvent| async move {
        if !Addons::is_active(ADDON_NAME) {
            return;
        }

        match InviteType::get(&event, &settings).unwrap_js() {
            InviteType::Regular => {
                event.prevent_default();
                event.stop_immediate_propagation();

                let regular_invite_candidates = get_regular_invite_candidates(&settings);
                send_regular_invite_invites(regular_invite_candidates.iter(), &settings).await;
            }
            InviteType::Mass => {
                event.prevent_default();
                event.stop_immediate_propagation();

                let mass_invite_candidates = get_mass_invite_candidates(&settings);
                send_mass_invite_invites(mass_invite_candidates.iter(), &settings).await;
            }
            InviteType::Null => return,
        }
    });

    window()
        .add_event_listener_with_callback("keydown", &better_group_invites_listener)
        .map_err(map_err!())
}

trait FilterCanInvite {
    fn filter_regular_invite_candidates<'a>(
        self,
        settings: &Settings,
    ) -> Filter<Self, impl FnMut(&(&'a OtherId, &'a Other)) -> bool>
    where
        Self: Iterator<Item = (&'a OtherId, &'a Other)> + Sized,
    {
        let from_none = settings.relations.none.get();
        let from_friends = settings.relations.friend.get();
        let from_clan = settings.relations.clan.get();
        let from_clan_ally = settings.relations.clan_ally.get();
        let from_fraction_ally = settings.relations.fraction_ally.get();
        let excluded_nicks_lock = settings.excluded_nicks.nicks.lock_ref();

        self.filter(move |&(other_id, other_data)| {
            is_regular_invite_candidate(
                *other_id,
                &other_data.into(),
                &from_none,
                &from_friends,
                &from_clan,
                &from_clan_ally,
                &from_fraction_ally,
                excluded_nicks_lock.deref(),
            )
        })
    }

    fn filter_mass_invite_candidates<'a>(
        self,
        settings: &Settings,
    ) -> Filter<Self, impl FnMut(&(&'a OtherId, Candidate)) -> bool>
    where
        Self: Iterator<Item = (&'a OtherId, Candidate)> + Sized,
    {
        self.filter(move |(other_id, other_data)| {
            is_mass_invite_candidate(**other_id, other_data, settings)
        })
    }
}

impl<I: Iterator> FilterCanInvite for I {}

///Returns `true` if not a valid candidate.
fn filter_out_candidate(other_id: Id, candidate: &Candidate, excluded_nicks: &[String]) -> bool {
    //Whether other is already in the party.
    if Party::get().lock_ref().contains_key(&other_id) {
        return true;
    }

    //Whether other is in the exclusion list.
    if excluded_nicks.contains(&candidate.nick) {
        return true;
    }

    candidate.emo.iter().any(|emotion| {
        matches!(
            emotion.name,
            EmotionName::Undefined
                | EmotionName::Battle
                | EmotionName::Logoff
                | EmotionName::Stasis
        )
    })
}

fn is_regular_invite_candidate(
    other_id: Id,
    candidate: &Candidate,
    &from_none: &bool,
    &from_friends: &bool,
    &from_clan: &bool,
    &from_clan_ally: &bool,
    &from_fraction_ally: &bool,
    excluded_nicks: &[String],
) -> bool {
    if filter_out_candidate(other_id, candidate, excluded_nicks) {
        return false;
    }

    match candidate.relation {
        Relation::None if candidate.next_to_hero() => from_none,
        Relation::Friend => from_friends,
        Relation::Clan => from_clan,
        Relation::ClanAlly => from_clan_ally,
        Relation::FractionAlly => from_fraction_ally,
        _ => false,
    }
}

fn is_mass_invite_candidate(other_id: i32, candidate: &Candidate, settings: &Settings) -> bool {
    let from_clan = settings.mass_invite.peers.clan.get();
    let from_friends = settings.mass_invite.peers.friend.get();
    let from_location = settings.mass_invite.peers.from_location.get();
    let excluded_nicks = settings.excluded_nicks.lock_ref();

    //debug_log!(
    //    "filter out candidate:",
    //    filter_out_candidate(other_id, candidate, excluded_nicks.deref())
    //);
    if filter_out_candidate(other_id, candidate, excluded_nicks.deref()) {
        return false;
    }
    let other_relation = candidate.relation;

    if other_relation == Relation::Clan {
        return from_clan;
    }
    if other_relation == Relation::Friend {
        return from_friends;
    }
    if !from_location {
        return false;
    }
    if other_relation == Relation::None {
        return settings.relations.none.get() && candidate.next_to_hero();
    }
    if other_relation == Relation::ClanAlly {
        return settings.relations.clan_ally.get();
    }
    if other_relation == Relation::FractionAlly {
        return settings.relations.fraction_ally.get();
    }

    false
}

fn get_regular_invite_candidates(settings: &Settings) -> HashSet<OtherId> {
    let others_lock = Others::get().lock_ref();

    others_lock
        .iter()
        .filter_regular_invite_candidates(settings)
        .map(|(candidate_id, _)| *candidate_id)
        .collect()
}

async fn send_regular_invite_invites(mut candidates: Iter<'_, OtherId>, settings: &Settings) {
    if settings.inviting.get() {
        message("Usuwam zakolejkowane zaproszenia...").unwrap_js();

        settings.interrupt.borrow_mut().push(());
    }

    let Some(candidate_id) = candidates.next() else {
        message("Brak graczy do zapraszania!").unwrap_js();
        return;
    };

    settings.inviting.set(true);
    communication::send_task(&communication::party::invite(candidate_id)).unwrap_js();

    let min_delay = settings.delay.min.value.get();
    let max_delay = settings.delay.max.value.get();

    for other_id in candidates {
        let others_lock = Others::get().lock_ref();
        let Some(other_data) = others_lock.get(other_id) else {
            continue;
        };

        let can_invite = is_regular_invite_candidate(
            *other_id,
            &other_data.into(),
            settings.relations.none.lock_ref().deref(),
            settings.relations.friend.lock_ref().deref(),
            settings.relations.clan.lock_ref().deref(),
            settings.relations.clan_ally.lock_ref().deref(),
            settings.relations.fraction_ally.lock_ref().deref(),
            settings.excluded_nicks.nicks.lock_ref().deref(),
        );
        if !can_invite {
            continue;
        }

        delay_range(min_delay as usize, max_delay as usize).await;

        let mut interrupt_lock = settings.interrupt.borrow_mut();
        if !interrupt_lock.is_empty() {
            interrupt_lock.pop();
            // Use return here instead of break since the last send_invites call
            // will set inviting to false.
            return;
        }

        communication::send_task(&communication::party::invite(other_id)).unwrap_js();
    }

    settings.inviting.set(false);
}

// TODO: Make this an associated function.
fn get_mass_invite_candidates(settings: &Settings) -> HashSet<OtherId> {
    let from_clan = settings.mass_invite.peers.clan.get();
    let from_friends = settings.mass_invite.peers.friend.get();
    let from_location = settings.mass_invite.peers.from_location.get();

    let mut candidates = HashSet::with_capacity(250);
    let others_lock = Others::get().lock_ref();

    if from_location {
        let candidates_from_location = others_lock
            .iter()
            .map(|(id, data)| (id, data.into()))
            .filter_mass_invite_candidates(settings)
            .map(|(other_id, _)| *other_id);

        candidates.extend(candidates_from_location);
    }
    if from_friends || from_clan {
        let peers_lock = Peers::get().lock_ref();
        let candidates_from_peers = peers_lock
            .iter()
            .filter_online(|(_, peer_data)| match peer_data.relation.get() {
                Relation::Clan => from_clan,
                Relation::Friend => from_friends,
                _ => false,
            })
            // TODO: Should this get from the others map ??
            .map(|(id, data)| match others_lock.get(id) {
                Some(other_data) => (id, other_data.into()),
                None => (id, data.into()),
            })
            .filter_mass_invite_candidates(settings)
            .map(|(peer_id, _)| *peer_id);

        candidates.extend(candidates_from_peers);
    }

    candidates
}

async fn send_mass_invite_invites(mut candidates: Iter<'_, OtherId>, settings: &Settings) {
    use crate::utils::delay_range;

    if settings.inviting.get() {
        message("Usuwam zakolejkowane zaproszenia...").unwrap_js();

        settings.interrupt.borrow_mut().push(());
    }

    let Some(candidate_id) = candidates.next() else {
        message("Brak graczy do zapraszania!").unwrap_js();
        return;
    };

    settings.inviting.set(true);
    communication::send_task(&communication::party::invite(candidate_id)).unwrap_js();

    let min_delay = settings.delay.min.value.get();
    let max_delay = settings.delay.max.value.get();

    for other_id in candidates {
        let others_lock = Others::get().lock_ref();
        let peers_lock = Peers::get().lock_ref();
        let Some(candidate) = others_lock
            .get(other_id)
            .map(Into::into)
            .or_else(|| peers_lock.get_online(other_id).map(Into::into))
        else {
            continue;
        };

        let can_invite = is_mass_invite_candidate(*other_id, &candidate, settings);
        if !can_invite {
            continue;
        }

        delay_range(min_delay as usize, max_delay as usize).await;

        let mut interrupt_lock = settings.interrupt.borrow_mut();
        if !interrupt_lock.is_empty() {
            interrupt_lock.pop();
            // Use return here instead of break since the last send_invites call
            // will set inviting to false.
            return;
        }

        communication::send_task(&communication::party::invite(other_id)).unwrap_js();
    }

    settings.inviting.set(false);
}
