use std::{cell::RefCell, sync::OnceLock};

use common::err_code;
use futures::stream::StreamExt;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use serde_json::{Value, json};
use wasm_bindgen::intern;
use web_sys::HtmlElement;

use crate::{
    s,
    utils::{JsResult, UnwrapJsExt},
};

use super::{
    hero::Hero,
    port::{
        Port,
    },
};

static ADDONS: OnceLock<Addons> = OnceLock::new();

macro_rules! init_window_stream {
    ($addon_window:expr, $stream_key:expr, $addon_name:expr) => {
        Self::init_stream(
            $addon_window.active.signal(),
            |active| {
                json!({
                    intern($crate::s!($stream_key)): {
                        intern($crate::s!("active")): active,
                    }
                })
            },
            $addon_name,
        );
        Self::init_stream(
            $addon_window.opacity_lvl.signal(),
            |opacity_lvl| {
                json!({
                    intern($crate::s!($stream_key)): {
                        intern($crate::s!("opacity_lvl")): opacity_lvl,
                    }
                })
            },
            $addon_name,
        );
        if let Some(header_description) = $addon_window.header_description.as_ref() {
            Self::init_stream(
                header_description.signal_cloned(),
                |header_description| {
                    json!({
                        intern($crate::s!($stream_key)): {
                            intern($crate::s!("header_description")): header_description,
                        }
                    })
                },
                $addon_name,
            );
        }
        Self::init_stream(
            $addon_window.size.signal(),
            |size| {
                json!({
                    intern($crate::s!($stream_key)): {
                        intern($crate::s!("size")): size,
                    }
                })
            },
            $addon_name,
        );
        Self::init_stream(
            $addon_window.expanded.signal(),
            |expanded| {
                json!({
                    intern($crate::s!($stream_key)): {
                        intern($crate::s!("expanded")): expanded,
                    }
                })
            },
            $addon_name,
        );
        Self::init_stream(
            $addon_window.left.signal(),
            |left| {
                json!({
                    intern($crate::s!($stream_key)): {
                        intern($crate::s!("left")): left,
                    }
                })
            },
            $addon_name,
        );
        Self::init_stream(
            $addon_window.top.signal(),
            |top| {
                json!({
                    intern($crate::s!($stream_key)): {
                        intern($crate::s!("top")): top,
                    }
                })
            },
            $addon_name,
        );

    };
}

macro_rules! create_addons {
    (
        free { $($free_addon_fieldname:ident),+ $(,)? },
        premium { $($premium_addon_fieldname:ident),+ $(,)? },
    ) => {
        ::paste::paste! {
            __inner_create_addons! {
                $(free $free_addon_fieldname as [<$free_addon_fieldname:camel>],)+
                $(premium $premium_addon_fieldname as [<$premium_addon_fieldname:camel>],)+
            }
        }
    };
}

macro_rules! __inner_create_addons {
    ($($status:ident $field:ident as $variant:ident),+ $(,)?) => {
        macro_rules! init_addons {
            () => {
                $(if $crate::globals::addons::Addons::get()[$crate::globals::addons::AddonName::$variant].is_some() {
                    common::debug_log!(stringify!($field));
                    $crate::addons::$field::init()?;
                })+
            };
        }
        pub(crate) use init_addons;

        macro_rules! export_addon_modules {
            () => {
                $(pub(super) mod $field;)+
            };
        }
        pub(crate) use export_addon_modules;

        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
        pub enum AddonName {
            $($variant),+
        }

        impl AddonName {
            pub(crate) const fn key_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($field),)+
                }
            }
        }

        #[derive(Debug)]
        pub struct Addons {
            $($field: ::core::option::Option<$crate::globals::addons::AddonData>),+
        }

        // SAFETY: There's no threads on wasm32.
        unsafe impl Sync for Addons {}
        unsafe impl Send for Addons {}

        impl Addons {
            pub(super) fn init(config: &mut ::serde_json::Value) -> $crate::utils::JsResult<()> {
                let premium = $crate::globals::premium::Premium::active();
                let this = Self {
                    $($field: match has_access!($status premium) {
                        true => Some(AddonData::new(config[stringify!($field)].take(), AddonName::$variant)),
                        false => None
                    },)+
                    #[cfg(feature = "antyduch")]
                    anty_duch: $crate::globals::premium::Premium::anty_duch().then(|| {
                        AddonData::new(config["antyduch"].take(), AddonName::AntyDuch)
                    }),
                };

                ADDONS.set(this).map_err(|_| ::common::err_code!())
            }

            pub fn iter(&self) -> impl Iterator<Item = Option<(AddonName, &AddonData)>> {
                [$(self.$field.as_ref().map(|data| (AddonName::$variant, data)),)+].into_iter()
            }
        }

        impl std::ops::Index<AddonName> for Addons {
            type Output = Option<AddonData>;

            fn index(&self, index: AddonName) -> &Self::Output {
                match index {
                    $(AddonName::$variant => &self.$field,)+
                }
            }
        }
    };
}

macro_rules! has_access {
    (free $access_level:expr) => {
        true
    };
    (premium $premium:expr) => {
        $premium
    };
}

create_addons! {
    free {
        accept_group,
        accept_summon,
        better_messages,
        better_who_is_here,
        online_peers,
        smart_forge,
        znacznik,
        grounded_mob_timers,
    },
    premium {
        adaptive_builds,
        better_group_invites,
        kastrat,
        hero_neon,
    },
}

impl Addons {
    pub fn get() -> &'static Addons {
        ADDONS.wait()
    }

    pub fn get_addon(addon_name: AddonName) -> Option<&'static AddonData> {
        Self::get()[addon_name].as_ref()
    }

    pub fn get_window_size(
        addon_name: AddonName,
        window_type: WindowType,
    ) -> Option<&'static Mutable<u8>> {
        Self::get()[addon_name]
            .as_ref()
            .map(|addon_data| &addon_data.get(window_type).size)
    }

    pub(crate) fn __internal_get_settings(addon_name: AddonName) -> Value {
        Self::get()[addon_name].as_ref().unwrap_js().settings[s!("settings")].clone()
    }

    pub(crate) fn __internal_get_active_settings(addon_name: AddonName) -> Value {
        Self::get()[addon_name].as_ref().unwrap_js().settings[s!("active_settings")].clone()
    }

    pub(crate) fn is_active(addon_name: AddonName) -> bool {
        Self::get()[addon_name]
            .as_ref()
            .map(|data| data.active.get())
            .unwrap_or(false)
    }

    pub(crate) fn active_signal(addon_name: AddonName) -> Option<impl Signal<Item = bool>> {
        Self::get()[addon_name]
            .as_ref()
            .map(|data| data.active.signal())
    }

    pub fn toggle_addon_active_state(
        addon_name: AddonName,
        window_type: WindowType,
    ) -> JsResult<()> {
        let addon_data = Self::get_addon(addon_name).ok_or_else(|| err_code!())?;

        // If the window isn't active, don't update the "on-peak" class.
        if addon_data
            .get(window_type)
            .active
            .replace_with(|status| !*status)
        {
            return Ok(());
        }

        let root_lock = addon_data.get(window_type).root.borrow();
        let root = root_lock.as_ref().ok_or_else(|| err_code!())?;

        crate::addon_window::try_update_on_peak(root, true)
    }
}

impl AddonName {
    pub(crate) const fn as_str(&self) -> &'static str {
        use AddonName::*;

        match self {
            AcceptGroup => "Akceptowanie Zaproszeń Do Grupy",
            AcceptSummon => "Akceptowanie Przywołań",
            AdaptiveBuilds => "Adaptacyjne Zestawy Do Walki",
            #[cfg(feature = "antyduch")]
            AntyDuch => "Anty Duch",
            // AutoHeal => "AutoHeal",
            BetterGroupInvites => "Zapraszanie Do Grupy",
            BetterWhoIsHere => "Gracze Na Mapie",
            BetterMessages => "Poprawione Powiadomienia",
            Kastrat => "Kastrat",
            OnlinePeers => "Rówieśnicy Online",
            SmartForge => "Super Rzemieślnik",
            Znacznik => "Znacznik",
            HeroNeon => "Neon Bohatera",
            GroundedMobTimers => "Timery Mobów Na Ziemi"
        }
    }

    //TODO: Generate hashed classes different for each session ?
    pub(crate) const fn to_class(self) -> &'static str {
        use AddonName::*;

        match self {
            AcceptGroup => "accept-group",
            AcceptSummon => "accept-summon",
            AdaptiveBuilds => "adaptive-builds",
            #[cfg(feature = "antyduch")]
            AntyDuch => "anty-duch",
            // AutoHeal => "auto-heal",
            BetterGroupInvites => "better-group-invites",
            BetterWhoIsHere => "better-who-is-here",
            BetterMessages => "better-messages",
            Kastrat => "kastrat",
            OnlinePeers => "online-peers",
            SmartForge => "smart-forge",
            Znacznik => "znacznik",
            HeroNeon => "hero-neon",
            GroundedMobTimers => "grounded-mob-timers"
        }
    }

    pub(crate) fn default_header_description(self, window_type: WindowType) -> Option<String> {
        use AddonName::*;

        let ret = match window_type {
            WindowType::AddonWindow => match self {
                AcceptGroup => None,
                AcceptSummon => None,
                AdaptiveBuilds => None,
                #[cfg(feature = "antyduch")]
                AntyDuch => Some("Anty Duch"),
                // AutoHeal => "auto-heal",
                BetterGroupInvites => Some("Zapraszanie Do Grupy"),
                BetterWhoIsHere => Some("Gracze Na Mapie"),
                BetterMessages => Some("Poprawione Powiadomienia"),
                Kastrat => Some("Kastrat"),
                OnlinePeers => Some("Rówieśnicy Online"),
                SmartForge => Some("Super Rzemieślnik"),
                Znacznik => Some("Znacznik"),
                HeroNeon => Some("Neon Bohatera"),
                GroundedMobTimers => None,
            },
            WindowType::SettingsWindow => match self {
                AcceptGroup => Some("Konfiguracja Akceptowania Zaproszeń Do Grup"),
                AcceptSummon => Some("Konfiguracja Akceptowania Przywołań"),
                AdaptiveBuilds => Some("Konfiguracja Adaptacyjnych Zestawów Do Walki"),
                #[cfg(feature = "antyduch")]
                AntyDuch => Some("Konfiguracja Anty Ducha"),
                // AutoHeal => "auto-heal",
                BetterGroupInvites => Some("Konfiguracja Wysyłania Zaproszeń Do Grup"),
                BetterWhoIsHere => Some("Konfiguracja Graczy Na Mapie"),
                BetterMessages => None,
                Kastrat => Some("Konfiguracja Kastrata"),
                OnlinePeers => Some("Konfiguracja Rówieśników Online"),
                SmartForge => Some("Konfiguracja Super Rzemieślnika"),
                Znacznik => None,
                HeroNeon => None,
                GroundedMobTimers => Some("Konfiguracja Timerów Mobów Na Ziemi")
            },
        };

        ret.map(ToOwned::to_owned)
    }
}

#[derive(Debug)]
pub struct AddonWindowDetails {
    pub active: Mutable<bool>,
    // TODO: Use Mutable<Option<_> here
    pub opacity_lvl: Mutable<u8>,
    pub header_description: Option<Mutable<String>>,
    // TODO: Maybe don't use u8 here ?
    pub size: Mutable<u8>,
    pub expanded: Mutable<bool>,
    pub left: Mutable<f64>,
    pub top: Mutable<f64>,
    pub root: RefCell<Option<HtmlElement>>,
}

impl AddonWindowDetails {
    fn new(config: &Value, addon_name: AddonName, window_type: WindowType) -> Self {
        let settings_value = match window_type {
            WindowType::AddonWindow => &config[intern(s!("active_settings"))],
            WindowType::SettingsWindow => &config[intern(s!("settings"))],
        };
        let active = Mutable::new(
            settings_value[intern(s!("active"))]
                .as_bool()
                .unwrap_or(false),
        );
        let left = Mutable::new(settings_value[intern(s!("left"))].as_f64().unwrap_or(1f64));
        let top = Mutable::new(settings_value[intern(s!("top"))].as_f64().unwrap_or(1f64));
        let opacity_lvl = Mutable::new(
            settings_value[intern(s!("opacity_lvl"))]
                .as_f64()
                .unwrap_or(4f64) as u8,
        );
        let header_description = settings_value[intern(s!("header_description"))]
            .as_str()
            .map(ToOwned::to_owned)
            .or_else(|| addon_name.default_header_description(window_type))
            .map(Mutable::new);
        let size = Mutable::new(settings_value[intern(s!("size"))].as_f64().unwrap_or(0f64) as u8);
        let expanded = Mutable::new(
            settings_value[intern(s!("expanded"))]
                .as_bool()
                .unwrap_or(true),
        );

        Self {
            active,
            opacity_lvl,
            header_description,
            size,
            expanded,
            left,
            top,
            root: RefCell::new(None),
        }
    }
}

#[derive(Debug)]
pub struct AddonData {
    pub active: Mutable<bool>,
    pub active_settings_window: AddonWindowDetails,
    pub settings_window: AddonWindowDetails,
    settings: Value,
}

impl AddonData {
    fn new(config: Value, addon_name: AddonName) -> Self {
        let active = Mutable::new(config[intern(s!("active"))].as_bool().unwrap_or(false));
        let active_settings_window =
            AddonWindowDetails::new(&config, addon_name, WindowType::AddonWindow);
        let settings_window =
            AddonWindowDetails::new(&config, addon_name, WindowType::SettingsWindow);

        let addon_data = Self {
            active,
            active_settings_window,
            settings_window,
            settings: config,
        };

        addon_data.stream_changes(addon_name);

        addon_data
    }

    //TODO: Change to a proc macro ?
    ///Initializes streaming each property change to extension background.
    fn stream_changes(&self, addon_name: AddonName) {
        Self::init_stream(
            self.active.signal(),
            |active| {
                json!({
                    intern(s!("active")): active,
                })
            },
            addon_name,
        );

        init_window_stream!(self.active_settings_window, "active_settings", addon_name);

        init_window_stream!(self.settings_window, "settings", addon_name);
    }

    fn init_stream<T: serde::Serialize + 'static>(
        signal: impl Signal<Item = T> + 'static,
        get_setting: fn(T) -> Value,
        addon_name: AddonName,
    ) {
        let future = signal
            .to_stream()
            .skip(1)
            .for_each(move |change| async move {
                let hero = Hero::get();
                let settings = json!({
                    intern(&hero.account_string): {
                        intern(&hero.char_id_string): {
                            addon_name.key_str(): get_setting(change)
                        }
                    }
                });
                // let msg = Task::builder()
                //     .target(Targets::Background)
                //     .task(Tasks::AddonData)
                //     .settings(settings)
                //     .build()
                //     .unwrap_js()
                //     .to_value()
                //     .unwrap_js();

                    todo!()
                // Port::send(msg).await;
            });
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    AddonWindow,
    SettingsWindow,
}

impl WindowType {
    pub(crate) fn to_toggle_class(self) -> &'static str {
        match self {
            Self::AddonWindow => "config-window-toggle",
            Self::SettingsWindow => "mdma-settings-button",
        }
    }
}

pub trait AddonDataMarker<'a> {
    fn get(self, window_type: WindowType) -> &'a AddonWindowDetails;
}

impl<'a> AddonDataMarker<'a> for &'a AddonData {
    fn get(self, window_type: WindowType) -> &'a AddonWindowDetails {
        match window_type {
            WindowType::AddonWindow => &self.active_settings_window,
            WindowType::SettingsWindow => &self.settings_window,
        }
    }
}
