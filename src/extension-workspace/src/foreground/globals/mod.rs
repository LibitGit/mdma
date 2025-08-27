pub mod addons;
pub mod collisions;
// TODO: Document this module.
/// Emitter emits all interceptors and handlers for the specified event, waiters
/// get notified before handlers are run.
pub mod emitter;
pub mod hero;
pub(crate) mod hero_settings;
pub mod items;
pub mod npcs;
pub mod others;
pub mod party;
pub mod peers;
pub mod players_online;
pub mod port;
pub mod premium;
pub mod town;
pub mod world_config;

use std::future::Future;
use std::pin::Pin;

use common::debug_log;
use futures::StreamExt;
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_map::{
    Len, MutableBTreeMap, MutableBTreeMapEntries, MutableBTreeMapKeys, MutableBTreeMapLockMut,
    MutableBTreeMapLockRef, MutableSignalMap,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wasm_bindgen::{JsValue, intern};

use crate::{
    bindings::engine::communication::{self, __send_task, Id, Response},
    s,
    utils::{JsResult, UnwrapJsExt, delay, delay_range, window_events},
};

pub mod prelude {
    pub use super::collisions::{Collisions, Gateways, MapCollisions, NpcCollisions};
    pub use super::{
        GlobalBTreeMap, ItemId, OtherId,
        addons::{AddonData, AddonDataMarker, AddonName, AddonWindowDetails, Addons, WindowType},
        emitter::{Emitter, EmitterEvent},
        hero::Hero,
        hero_settings::HeroSettings,
        items::ItemBTreeMap as Items,
        npcs::{NpcTemplates, Npcs},
        others::OtherBTreeMap as Others,
        party::PartyBTreeMap as Party,
        peers::{FilterOnline, PeerBTreeMap as Peers, PeerId},
        players_online::PlayersOnline,
        port::Port,
        town::Town,
        world_config::WorldConfig,
        premium::Premium,
    };
}

//TODO: Should these be newtype wrappers ?
pub type OtherId = Id;
//TODO: Verify this doesn't overflow.
pub type ItemId = Id;

pub(crate) struct ManagerGlobals {
    pub(crate) widget_active: Mutable<bool>,
    pub(crate) hotkey: Mutable<ManagerHotkey>,
}

impl ManagerGlobals {
    fn new(widget_active: Mutable<bool>, hotkey: Mutable<ManagerHotkey>) -> Self {
        Self {
            widget_active,
            hotkey,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct ManagerHotkey {
    pub(crate) value: String,
    pub(crate) ctrl_key: bool,
    pub(crate) shift_key: bool,
    pub(crate) alt_key: bool,
}

impl Default for ManagerHotkey {
    fn default() -> Self {
        Self {
            value: String::from("TAB"),
            ctrl_key: false,
            shift_key: false,
            alt_key: false,
        }
    }
}

impl window_events::Hotkey for ManagerHotkey {
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

// TODO: Better name for variants.
#[derive(Debug)]
pub(super) enum GlobalsError {
    Unauthorized,
    Unrecoverable(JsValue),
}

impl GlobalsError {
    #[track_caller]
    fn unrecoverable() -> Self {
        Self::Unrecoverable(common::err_code!(track_caller))
    }
}

impl From<JsValue> for GlobalsError {
    fn from(value: JsValue) -> Self {
        Self::Unrecoverable(value)
    }
}

pub(super) struct Globals;

impl Globals {
    pub(super) async fn init() -> Result<&'static ManagerGlobals, GlobalsError> {
        port::Port::init_authorized().await?;
        premium::Premium::init().await?;
        hero::Hero::init().await?;

        let mut addons_config =
        port::Port::init_session().await?;

        // emitter::Emitter::init()?;
        // addons::Addons::init(config)?;
        // hero_settings::HeroSettings::init()?;
        // items::ItemBTreeMap::init()?;
        // others::OtherBTreeMap::init()?;
        // party::PartyBTreeMap::init()?;
        // peers::PeerBTreeMap::init()?;
        // town::Town::init()?;
        // players_online::PlayersOnline::init()?;
        // npcs::NpcTemplates::init()?;
        // npcs::Npcs::init()?;

        // let widget_active = Self::init_widget_state(&config);
        // let manager_hotkey = Self::init_manager_hotkey(config);
        // let manager_globals: &'static ManagerGlobals =
        //     Box::leak(Box::new(ManagerGlobals::new(widget_active,
        // manager_hotkey))); Ok(Some(manager_globals))
        todo!()
    }

    fn init_widget_state(config: &Value) -> Mutable<bool> {
        let widget_active = Mutable::new(config[s!("widget")].as_bool().unwrap_or(true));
        let future = widget_active
            .signal()
            .to_stream()
            .skip(1)
            .for_each(|active| async move {
                let hero = hero::Hero::get();
                let settings = json!({
                    intern(&hero.account_string): {
                        intern(&hero.char_id_string): {
                            intern(s!("widget")): active
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

                //debug_log!(&msg);
                todo!()
                // port::Port::send(msg).await;
            });
        wasm_bindgen_futures::spawn_local(future);

        widget_active
    }

    fn init_manager_hotkey(config: &mut Value) -> Mutable<ManagerHotkey> {
        let manager_hotkey = Mutable::new(
            serde_json::from_value(config[s!("manager_hotkey")].take()).unwrap_or_default(),
        );
        let future = manager_hotkey
            .signal_ref(|change| json!(change))
            .to_stream()
            .skip(1)
            .for_each(|active| async move {
                let hero = hero::Hero::get();
                let settings = json!({
                intern(&hero.account_string): {
                    intern(&hero.char_id_string): {
                            intern(s!("manager_hotkey")): active
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
                // port::Port::send(msg).await;
            });
        wasm_bindgen_futures::spawn_local(future);

        manager_hotkey
    }

    pub(super) async fn start_players_online_update_interval() {
        loop {
            match players_online::PlayersOnline::update().await {
                Ok(false) => delay(1_000).await,
                Ok(true) => delay_range(60_000, 120_000).await,
                Err(err_code) => {
                    console_error!(err_code);
                    delay_range(60_000, 120_000).await
                }
            }
        }
    }

    // TODO: Remove interceptor on _g error.
    // TODO: Fetch clan members after joining clan, remove after leaving.
    pub(super) async fn start_peers_map_update_interval() {
        use self::emitter::{Emitter, EmitterEvent};
        use self::peers::PeerBTreeMap as Peers;

        fn callback<'a>(
            socket_response: &'a mut Response,
        ) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>> {
            Box::pin(async move {
                Peers::extract_from_socket_response(socket_response);
                Ok(())
            })
        }

        loop {
            let intercepted =
                Emitter::intercept_limited(EmitterEvent::Friends, 1, callback).is_ok();

            if intercepted && let Err(_err) = __send_task(communication::friends::get_friends).await
            {
                return debug_log!(_err);
            }

            if !hero::Hero::is_in_clan() {
                // TODO: This is not 10mins lmao also remove this.
                // 10min + up to 15s
                delay_range(600_000, 615_000).await;
                continue;
            }

            delay_range(1500, 3000).await;

            let intercepted =
                Emitter::intercept_limited(EmitterEvent::Members, 1, callback).is_ok();

            if intercepted && let Err(_err) = __send_task(communication::clan::get_members).await {
                return debug_log!(_err);
            }

            // 10min + up to 15s
            delay_range(600_000, 615_000).await;
        }
    }
}

// TODO: Deprecate use macro for methods instead ?
pub trait GlobalBTreeMap<K: Ord, V> {
    // TODO: Move this to another trait and make GlobalBTreeMap a supertrait.
    fn get(&self) -> &MutableBTreeMap<K, V>;

    fn lock_ref(&self) -> MutableBTreeMapLockRef<'_, K, V> {
        self.get().lock_ref()
    }

    fn lock_mut(&self) -> MutableBTreeMapLockMut<'_, K, V> {
        self.get().lock_mut()
    }

    fn signal_map_cloned(&self) -> MutableSignalMap<K, V>
    where
        K: Ord + Clone,
        V: Clone,
    {
        self.get().signal_map_cloned()
    }

    fn signal_vec_keys(&self) -> MutableBTreeMapKeys<K, V>
    where
        K: Ord + Clone,
        V: Clone,
    {
        self.get().signal_vec_keys()
    }

    fn len(&self) -> Len<MutableSignalMap<K, V>>
    where
        K: Ord + Clone,
        V: Clone,
    {
        self.get().len()
    }

    fn entries_cloned(&self) -> MutableBTreeMapEntries<K, V>
    where
        K: Ord + Clone,
        V: Clone,
    {
        self.get().entries_cloned()
    }
}
