use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use futures::channel::oneshot;
use futures_signals::signal::Mutable;
use futures_signals::signal_map::MutableBTreeMapLockMut;
use js_sys::{Function, Promise};
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{
    BoolFromInt, DeserializeFromStr, DisplayFromStr, serde_as, skip_serializing_none,
};
use wasm_bindgen::{intern, prelude::*};
use web_sys::MessageEvent;

use crate::bindings::window;
use crate::interface::{CONSOLE_LOGS, ConsoleLog, ConsoleLogTypes};
use crate::prelude::*;

use super::types::{EquipmentSlot, ItemClass};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type Communication;

    #[wasm_bindgen(catch, method, js_name = "initWebSocket")]
    pub fn init_web_socket(this: &Communication) -> JsResult<()>;

    #[wasm_bindgen(method, setter = "initWebSocket")]
    pub fn set_init_web_socket(this: &Communication, value: &Function);

    #[wasm_bindgen(catch, method, js_name = "onmessage")]
    pub fn onmessage(this: &Communication, evt: &JsValue) -> JsResult<JsValue>;

    #[wasm_bindgen(method, setter = "onmessage")]
    pub fn set_onmessage(this: &Communication, value: &Function);

    #[wasm_bindgen(method, getter = "onmessage")]
    pub fn get_onmessage(this: &Communication) -> Option<Function>;
}

thread_local! {
    #[doc = "Keeps track of an internal task queue and replaces empty tasks with ones from the queue."]
    #[doc = "Does not keep track of order."]
    static TASKS: RefCell<Vec<(String, oneshot::Sender<()>)>> = const { RefCell::new(Vec::new()) };
}

fn clear_queue(task: &mut String) {
    TASKS.with_borrow_mut(|tasks| {
        let mut index = 0;

        if task == intern(s!("_")) {
            let Some((new_task, sender)) = tasks.pop() else {
                return;
            };
            *task = new_task;
            if sender.send(()).is_err() {
                console_error!();
            }
        }

        while index < tasks.len() {
            let (stored_task, _) = unsafe { tasks.get_unchecked(index) };

            if stored_task != task {
                index += 1;
                continue;
            }

            let (_, sender) = tasks.swap_remove(index);
            if sender.send(()).is_err() {
                console_error!();
            }
        }
    });
}

impl Communication {
    pub(crate) fn try_init_game(&self, original_init: &Function) -> JsResult<()> {
        js_sys::Reflect::delete_property(self, &JsValue::from_str("initWebSocket"))
            .map_err(map_err!())?;
        self.set_init_web_socket(original_init);

        web_sys::console::log_5(
            &JsValue::from_str(s!("%c MDMA %c %c Rust ")),
            &JsValue::from_str(s!(
                "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &JsValue::from_str(s!("")),
            &JsValue::from_str(s!(
                "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &JsValue::from_str(s!("Calling game init...")),
        );

        self.init_web_socket().map_err(|_error| {
            debug_log!("Gameinit call error:", _error);
            web_sys::console::error_5(
                &JsValue::from_str(s!("%c MDMA %c %c Rust ")),
                &JsValue::from_str(s!(
                    "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
                )),
                &JsValue::from_str(s!("")),
                &JsValue::from_str(s!(
                    "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
                )),
                &JsValue::from_str(s!("Failed to call game init.")),
            );

            err_code!()
        })
    }

    // TODO: Fix problems with borrow checker, before trying to emit based on send task.
    ///Hooks to the function that handles requests sent to the margonem's server.
    pub(crate) fn observe_send(&self) -> JsResult<()> {
        let original_send = self.get_send().ok_or_else(|| err_code!())?;
        let context = self.clone();
        let new_send =
            Closure::<dyn FnMut(String, JsValue, JsValue) -> DefaultResult>::wrap(Box::new(
                move |mut task: String, callback: JsValue, payload: JsValue| -> DefaultResult {
                    #[cfg(debug_assertions)]
                    Self::_g_debug_log(&task, &callback, &payload);

                    clear_queue(&mut task);

                    if callback.is_truthy() && payload.is_truthy() {
                        original_send.send_task_with_callback_and_payload(
                            &context, &task, &callback, &payload,
                        )
                    } else if payload.is_truthy() {
                        original_send.send_task_with_callback_and_payload(
                            &context,
                            &task,
                            &JsValue::FALSE,
                            &payload,
                        )
                    } else if callback.is_truthy() {
                        original_send.send_task_with_callback(&context, &task, &callback)
                    } else {
                        original_send.send_task(&context, &task)
                    }
                },
            ))
            .into_js_value()
            .unchecked_into();

        self.set_send(&new_send);

        Ok(())
    }

    #[cfg(debug_assertions)]
    fn _g_debug_log(task: &str, callback: &JsValue, payload: &JsValue) {
        if !crate::utils::window().has_own_property(&JsValue::from_str("mdma_dbg")) {
            return;
        }

        if task == intern(s!("_")) {
            return;
        }

        if callback.is_truthy() && payload.is_truthy() {
            return debug_log!(task, callback, payload);
        }
        if payload.is_truthy() {
            return debug_log!(task, false, payload);
        }
        if callback.is_truthy() {
            return debug_log!(task, callback);
        }

        debug_log!(task);
    }
}

/// Hooks to the function that handles margonem server’s response
#[wasm_bindgen(js_name = "observeOnmessage")]
pub fn observe_on_message(
    web_socket: web_sys::WebSocket,
    communication: Communication,
) -> JsResult<()> {
    use crate::dispatcher::dispatch_events;

    // TODO:
    // if crate::globals::get_access_level()
    //     <= crate::globals::port::task::Task::UNAUTHORIZED_ACCESS_LEVEL
    // {
    //     return Ok(());
    // }

    let original_onmessage = communication.get_onmessage().ok_or_else(|| err_code!())?;
    let new_on_message_web_socket = Closure::<dyn FnMut(JsValue) -> Promise>::wrap(Box::new({
        let communication = communication.clone();
        move |message_event: JsValue| -> Promise {
            let original_onmessage = original_onmessage.clone();
            let communication = communication.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                let mut message_event: MessageEvent = message_event.unchecked_into();
                let message_event_data = message_event.data();
                let data_str = message_event_data.as_string().ok_or_else(|| err_code!())?;

                //CRASH_HANDLE.with_borrow_mut(|crash_handle| {
                //    if crash_handle.crashed {
                //        if let Err(_err) = crash_handle.try_handle_crash(message_event) {
                //            debug_log!(_err.to_string());
                //            //TODO: Handle if crash failed?
                //        };
                //    }
                //});

                #[cfg(debug_assertions)]
                log_json_data(&message_event);

                let res_as_value: Value =
                    serde_json::from_str(&data_str).map_err(map_err!(from))?;
                let mut res: Response =
                    serde_json::from_value(res_as_value.clone()).map_err(map_err!(from))?;

                let console_log = ConsoleLog::new(
                    ConsoleLogTypes::CommunicationData,
                    message_event_data,
                    res.ev,
                );
                if let Some(ev) = res.ev {
                    get_engine().set_ev(ev);
                }

                CONSOLE_LOGS.with_borrow_mut(|logs| {
                    if logs.len() == logs.capacity() {
                        logs.pop_back();
                    }

                    logs.push_front(console_log);
                });

                dispatch_events(res.clone());
                if Emitter::emit_events(&mut res).await {
                    replace_message_event_data(&mut message_event, &res, res_as_value)?;
                }

                original_onmessage
                    .call1(&communication, &message_event)
                    .map_err(map_err!())?;

                // FIXME: Is it ok to await in here ?
                Emitter::emit_after_events(&res).await;

                Ok(JsValue::UNDEFINED)
            })
        }
    }))
    .into_js_value()
    .unchecked_into();

    js_sys::Reflect::delete_property(web_socket.unchecked_ref(), &JsValue::from_str("onmessage"))
        .map_err(map_err!())?;
    communication.set_onmessage(&new_on_message_web_socket);

    Ok(())
}

// TODO: Iterate the values recursively. No idea how...
//       Either this or keep Response in sync with the game...
//       Or never partially remove a field.
fn replace_message_event_data(
    original_message_event: &mut MessageEvent,
    socket_response: &Response,
    response_as_value: Value,
) -> JsResult<()> {
    let dictionary = web_sys::MessageEventInit::new();
    let Value::Object(mut new_data) = response_as_value else {
        return Err(err_code!());
    };

    // Retain Some or values undefined in Response.
    new_data.retain(|key, value| {
        socket_response
            .has_field(key, value)
            .is_none_or(|is_some| is_some)
    });

    let data = JsValue::from_str(&serde_json::to_string(&new_data).map_err(map_err!(from))?);
    dictionary.set_data(&data);
    dictionary.set_bubbles(original_message_event.bubbles());
    dictionary.set_cancelable(original_message_event.cancelable());
    dictionary.set_composed(original_message_event.composed());
    dictionary.set_last_event_id(&original_message_event.last_event_id());
    dictionary.set_origin(&original_message_event.origin());
    dictionary.set_ports(&original_message_event.ports());
    dictionary.set_source(original_message_event.source().as_ref());

    *original_message_event =
        MessageEvent::new_with_event_init_dict(&original_message_event.type_(), &dictionary)
            .map_err(map_err!())?;

    Ok(())
}

#[cfg(debug_assertions)]
fn log_json_data(message_event: &MessageEvent) {
    use serde::Serialize;
    use serde_json::Value;
    use serde_wasm_bindgen::Serializer;

    use crate::utils::window;

    let data = message_event.data().as_string().unwrap_js();
    let response = serde_json::from_str::<serde_json::Value>(&data).unwrap_js();
    let Value::Object(response_map) = response else {
        return debug_log!(&format!("{response:?}"));
    };

    if response_map.len() == 2 && response_map.contains_key("ev") && response_map.contains_key("e")
    {
        return;
    }

    if !window().has_own_property(&JsValue::from_str("mdma_dbg")) {
        return;
    }

    common::js_imports::js_log(Box::from([
        JsValue::from_str(intern(s!("%cRESPONSE: %o"))),
        JsValue::from_str(intern(s!("color: gold"))),
        response_map
            .serialize(&Serializer::json_compatible())
            .unwrap_js(),
    ]));

    if !window().has_own_property(&JsValue::from_str("mdma_dbg2")) {
        return;
    }

    response_map.iter().for_each(|(key, value)| {
        if matches!(key.as_str(), "e" | "ev" | "js") {
            return;
        }

        common::js_imports::js_log(Box::from([
            JsValue::from_str(&format!("%c{}: %o", key)),
            JsValue::from_str(intern(s!("color: gold"))),
            value.serialize(&Serializer::json_compatible()).unwrap_js(),
        ]));
    });
}

const EVENT_STATS: [&str; 16] = [
    "Urodziny Margonem",
    "Wielkanoc",
    "Sabat Czarownic",
    "Noc Kupały",
    "Wakacje",
    "Halloween",
    "Gwiazdka",
    "Boże Narodzienie",
    "Event świąteczny",
    "Pamiątka z okazji",
    "One Night Casino",
    "Licytacja",
    "Swięto Plonów",
    "Pierwszy dzień wiosny",
    "Majówkowy Festyn",
    "Dzień Dziecka",
];

pub type Id = i32;

pub(crate) fn send_task(task: &str) -> JsResult<()> {
    if get_engine().log_off().is_none() {
        window()._g(task)?;
        return Ok(());
    }

    debug_log!(&format!("Not sending: {task} since hero is logging off!"));
    Ok(())
}

pub(crate) async fn __send_task(task: &str) -> DefaultResult {
    wait_for_without_timeout(can_send_idle_request, 1000).await;
    window()._g(task)
}

// Resolves before send2/sendRequest is called with the given task.
pub async fn send_request(task: String) -> JsResult<()> {
    let (tx, rx) = oneshot::channel();
    TASKS.with_borrow_mut(|tasks| tasks.push((task.clone(), tx)));
    window()._g(&task)?;

    rx.await.map_err(map_err!(from))
}

//TODO: Check pvp_protected and captcha.
//Captcha either by storing a thread_local or add to globals or check Engine.lock.list['captcha']
pub fn can_send_idle_request() -> bool {
    let engine = get_engine();
    //Logging off
    if engine.log_off().is_some() {
        return false;
    }
    //Initializing game
    if engine.get_all_init().is_none_or(|all_init| !all_init) {
        return false;
    }
    //let pvp_protected = false;
    //let has_captcha = false;
    if crate::globals::hero::Hero::in_battle() {
        return false;
    }

    true
}

//pub(crate) fn update_ws_message_data(js_message_event: &mut JsValue, data_str: &str) {
//    let Ok(message_event) = js_message_event.clone().dyn_into::<MessageEvent>() else {
//        return console_error!(error::std::obf_cast!("message_event"));
//    };
//
//    let init = web_sys::MessageEventInit::new();
//    init.set_data(&JsValue::from_str(data_str));
//    init.set_bubbles(message_event.bubbles());
//    init.set_cancelable(message_event.cancelable());
//    init.set_composed(message_event.composed());
//    init.set_source(message_event.source().as_ref());
//    init.set_origin(&message_event.origin());
//    init.set_last_event_id(&message_event.last_event_id());
//
//    match MessageEvent::new_with_event_init_dict(&message_event.type_(), &init) {
//        Ok(message_event) => *js_message_event = JsValue::from(message_event),
//        Err(_err) => {
//            debug_log!("MESSAGE EVENT DATA UPDATE FAILED:", _err);
//            console_error!(error::std::obf_set!("js_message_event"));
//        }
//    }
//}

// fn append_ws_message_data(js_message_event: &mut JsValue,) {
//     let Ok(message_event) = js_message_event
//         .clone()
//         .dyn_into::<MessageEvent>()
//     else {
//         return console_error!(error::std::obf_cast!("message_event"));
//     };
//     let Some(message_data) = message_event
//         .data()
//         .as_string() else  {
//         return console_error!(error::std::obf_cast!("message_data"));
//     };
//     let Ok(mut data ) = serde_json::from_str::<Value>(&message_data) else {
//         return console_error!( error::std::obf_cast!("data"))
//     };
//
//     match data.as_object_mut() {
//         Some(json_obj) => json_obj.,
//         None => return console_error!(error::std::obf_get!("json_obj")),
//     };
// }

pub(crate) mod js_imports {
    use js_sys::Function;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = _g)]
        pub(crate) fn send_task_with_callback(task: &str, clb: &Function);

        ///If you don't want to provide a callback use false instead of a Function;
        #[wasm_bindgen(js_name = _g)]
        pub(crate) fn send_task_with_callback_and_payload(
            task: &str,
            clb: &JsValue,
            payload: &JsValue,
        );
    }
}

// pub(crate) fn bool_de<'de, D>(deserializer: D) -> Result<bool, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     use serde_json::Value::{self, *};
//     Ok(match Value::deserialize(deserializer)? {
//         Null => false,
//         Bool(v) => v,
//         Number(v) => v.as_i64().unwrap_or_default() != 0,
//         String(v) => !v.is_empty(),
//         Array(v) => !v.is_empty(),
//         Object(_) => true,
//     })
// }

pub(crate) mod clan {
    #[allow(non_upper_case_globals)]
    pub(crate) const get_members: &str = "clan&a=members";
}

pub(crate) mod friends {
    #[allow(non_upper_case_globals)]
    pub(crate) const get_friends: &str = "friends&a=show";
}

pub(crate) mod party {
    use std::fmt::Display;

    pub(crate) const ACCEPT: &str = "party&a=accept&answer=";
    pub(crate) const ACCEPT_SUMMON: &str = "party&a=acceptsummon&answer=";

    pub(crate) fn accept(answer: bool) -> String {
        format!("{}{}", ACCEPT, answer as u8)
    }

    pub(crate) fn accept_summon(answer: bool) -> String {
        format!("{}{}", ACCEPT_SUMMON, answer as u8)
    }

    pub(crate) fn invite<A: Display>(id: A) -> String {
        format!("party&a=inv&id={}", id)
    }
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Response {
    pub artisanship: Option<Artisanship>,
    // pub(crate) alert: Option<String>,
    pub ask: Option<Ask>,
    pub business_cards: Option<BusinessCards>,
    #[serde(rename = "cl")]
    pub collisions: Option<String>,
    #[serde(rename = "gw2")]
    pub gateways: Option<Vec<i32>>,
    // pub browser_token: Option<String>,
    pub chat: Option<Chat>,
    // pub dead: Option<i32>,
    pub emo: Option<Vec<Emotion>>,
    pub enhancement: Option<Enhancement>,
    pub ev: Option<f64>,
    pub f: Option<FightData>,
    #[serde(deserialize_with = "friends_de")]
    pub friends: Option<Vec<Friend>>,
    #[serde(rename = "friends_max")]
    pub friends_max: Option<u8>,
    pub enemies: Option<serde_json::Value>,
    #[serde(rename = "enemies_max")]
    pub enemies_max: Option<u8>,
    pub h: Option<HeroData>,
    #[serde_as(as = "Option<HashMap<DisplayFromStr, _>>")]
    pub item: Option<HashMap<Id, Item>>,
    pub loot: Option<Loot>,
    #[serde(deserialize_with = "members_de")]
    pub members: Option<Vec<ClanMember>>,
    pub npcs: Option<Vec<NpcData>>,
    #[serde(rename = "npc_tpls")]
    pub npc_tpls: Option<Vec<NpcTemplate>>,
    #[serde(rename = "npcs_del")]
    pub npcs_del: Option<Vec<NpcDelData>>,
    #[serde_as(as = "Option<HashMap<DisplayFromStr, _>>")]
    pub other: Option<HashMap<Id, OtherData>>,
    pub party: Option<PartyData>,
    pub t: Option<String>,
    pub town: Option<TownData>,
    #[serde(rename = "settings")]
    pub character_settings: Option<CharacterSettings>,
    pub w: Option<String>,
    pub world_config: Option<WorldConfigData>,
    // pub world_time: Option<i32>,
}

impl Response {
    /// Indexing a [`Response`] with &str.
    ///
    /// Returns [`None`] if the value is not a field of Response, otherwise true if the field is [`Some`].
    pub fn has_field(&self, field_name: &str, value: &mut Value) -> Option<bool> {
        match field_name {
            "artisanship" => Some(self.artisanship.is_some()),
            "ask" => Some(self.ask.is_some()),
            "businessCards" => Some(self.business_cards.is_some()),
            "cl" => Some(self.collisions.is_some()),
            "gw2" => Some(self.gateways.is_some()),
            "chat" => Some(self.chat.is_some()),
            "emo" => Some(self.emo.is_some()),
            "enhancement" => Some(self.enhancement.is_some()),
            "ev" => Some(self.ev.is_some()),
            "friends" => Some(self.friends.is_some()),
            "friends_max" => Some(self.friends_max.is_some()),
            "enemies" => Some(self.enemies.is_some()),
            "enemies_max" => Some(self.enemies_max.is_some()),
            "h" => Some(self.h.is_some()),
            "item" => {
                let Some(items) = self.item.as_ref() else {
                    return Some(false);
                };
                let value = value.as_object_mut()?;
                value.retain(|key, _| {
                    if let Ok(item_id) = key.parse::<Id>() {
                        // debug_log!("item_id:", item_id, items.contains_key(&item_id));
                        items.contains_key(&item_id)
                    } else {
                        true
                    }
                });

                Some(!value.is_empty())
            }
            "loot" => Some(self.loot.is_some()),
            "members" => Some(self.members.is_some()),
            "npcs" => Some(self.npcs.is_some()),
            "npc_tpls" => Some(self.npc_tpls.is_some()),
            "npcs_del" => Some(self.npcs_del.is_some()),
            "other" => Some(self.other.is_some()),
            "party" => Some(self.party.is_some()),
            "t" => Some(self.t.is_some()),
            "town" => Some(self.town.is_some()),
            "settings" => Some(self.character_settings.is_some()),
            "w" => Some(self.w.is_some()),
            "worldConfig" => Some(self.world_config.is_some()),
            _ => None,
        }
    }
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TownData {
    pub name: Option<String>,
    pub id: Option<Id>,
    pub x: Option<u8>,
    pub y: Option<u8>,
    pub visibility: Option<i32>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "camelCase")]
pub struct FightData {
    #[serde_as(as = "Option<BoolFromInt>")]
    pub end_battle: Option<bool>,
}

#[derive(Debug, Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LootWantState {
    NotWant = 0,
    Want = 1,
    MustHave = 2,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Loot {
    pub init: Option<u8>,
    pub source: Option<String>,
    #[serde_as(as = "Option<HashMap<DisplayFromStr, _>>")]
    pub states: Option<HashMap<Id, LootWantState>>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Artisanship {
    open: Option<String>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct UsagesPreview {
    pub count: Option<u16>,
    pub limit: Option<u16>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct EnhanceProgress {
    pub current: Option<u32>,
    pub max: Option<u32>,
    #[serde(rename = "upgradeLevel")]
    pub upgrade_level: Option<u8>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct EnhanceUpgradable {}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct Enhancement {
    pub usages_preview: Option<UsagesPreview>,
    #[serde(rename = "itemId")]
    pub item_id: Option<Id>,
    pub progressing: Option<EnhanceProgress>,
    pub upgradable: Option<EnhanceUpgradable>,
}

#[derive(Debug, Serialize, DeserializeFromStr, Clone, Copy, PartialEq, Eq)]
pub enum SettingAction {
    Init,
    UpdateData,
}

impl FromStr for SettingAction {
    type Err = f64;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == intern(s!("INIT")) {
            Ok(Self::Init)
        } else if s == intern(s!("UPDATE_DATA")) {
            Ok(Self::UpdateData)
        } else {
            Err(err_code!(as_num))
        }
    }
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct CharacterSettings {
    pub action: SettingAction,
    pub list: Option<HeroSettingsData>,
}

impl CharacterSettings {
    pub(crate) const FRIEND_NOTIF_ID: CharacterSettingId = CharacterSettingId(15);
    pub(crate) const CLAN_NOTIF_ID: CharacterSettingId = CharacterSettingId(9);

    pub(crate) async fn init_setting(setting_id: CharacterSettingId) {
        if let Err(err_code) =
            __send_task(&format!("settings&action=update&id={}&v=1", setting_id.0))
                .await
                .map_err(map_err!())
        {
            console_error!(err_code)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterSettingId(u8);

impl PartialEq<u8> for CharacterSettingId {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl<'a> TryFrom<&'a JsValue> for CharacterSettingId {
    type Error = JsValue;

    fn try_from(value: &'a JsValue) -> Result<Self, Self::Error> {
        let value = value.unchecked_into_f64() as u8;
        if CharacterSettings::FRIEND_NOTIF_ID == value {
            Ok(CharacterSettings::FRIEND_NOTIF_ID)
        } else if CharacterSettings::CLAN_NOTIF_ID == value {
            Ok(CharacterSettings::CLAN_NOTIF_ID)
        } else {
            Err(err_code!())
        }
    }
}

// TODO: impl Serialize ?
#[derive(Debug, Serialize, Clone, Copy, Default)]
pub struct HeroSettingsData {
    pub friend_login_notif: Option<SettingData>,
    pub clan_login_notif: Option<SettingData>,
}

impl<'de> Deserialize<'de> for HeroSettingsData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SettingVisitor;

        impl<'de> Visitor<'de> for SettingVisitor {
            type Value = HeroSettingsData;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an array of maps of maps")
            }

            fn visit_seq<M>(self, mut seq: M) -> Result<Self::Value, M::Error>
            where
                M: SeqAccess<'de>,
            {
                let mut friend_login_notif = None;
                let mut clan_login_notif = None;

                while let Some(entry) = seq.next_element::<HashMap<String, SettingData>>()? {
                    for (key, value) in entry {
                        match key.as_str() {
                            "15" => {
                                friend_login_notif = Some(value);
                            }
                            "9" => {
                                clan_login_notif = Some(value);
                            }
                            _ => {}
                        }
                    }
                }

                Ok(HeroSettingsData {
                    friend_login_notif,
                    clan_login_notif,
                })
            }
        }

        deserializer.deserialize_seq(SettingVisitor)
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct SettingData {
    #[serde(rename = "v")]
    pub value: Option<bool>,
}

// TODO: Is the enumaration correct ?
// For instance in online peers counter tip the friend should
// be more important than clan member.
#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum Relation {
    None = 1,
    Friend = 2,
    Enemy = 3,
    Clan = 4,
    ClanAlly = 5,
    ClanEnemy = 6,
    FractionAlly = 7,
    FractionEnemy = 8,
}

impl Relation {
    pub fn to_str(self) -> &'static str {
        use Relation::*;

        match self {
            None => "none",
            Friend => "friend",
            Enemy => "enemy",
            Clan => "clan",
            ClanAlly => "clan-ally",
            ClanEnemy => "clan-enemy",
            FractionAlly => "fraction-ally",
            FractionEnemy => "fraction-enemy",
        }
    }
}

impl<'de> Deserialize<'de> for Relation {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        match value as i32 {
            1 => Ok(Relation::None),
            2 => Ok(Relation::Friend),
            3 => Ok(Relation::Enemy),
            4 => Ok(Relation::Clan),
            5 => Ok(Relation::ClanAlly),
            6 => Ok(Relation::ClanEnemy),
            7 => Ok(Relation::FractionAlly),
            8 => Ok(Relation::FractionEnemy),
            _ => Err(serde::de::Error::missing_field("a valid relation")),
        }
    }
}

#[derive(Debug, PartialEq, Default, Eq, Clone, Copy, Hash)]
pub enum EmotionName {
    AbyssOut,
    Angry,
    Bat,
    Battle,
    Frnd, //what is this?
    Login,
    Logoff,
    LvlUp,
    PvpProtected,
    Respawned,
    Away,
    /// Not a built-in emotion
    AwayEnd,
    Spider,
    Stasis,
    /// Not a built-in emotion
    StasisEnd,
    Teleported,
    #[default]
    Undefined,
    Noemo,
}

// FIXME: AwayEnd and StasisEnd cannot be added to the emotion vector.
impl fmt::Display for EmotionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use EmotionName::*;
        let emotion = match self {
            AbyssOut => Ok("abbysout"),
            Angry => Ok("angry"),
            Away | AwayEnd => Ok("away"),
            Bat => Ok("bat"),
            Battle => Ok("battle"),
            Frnd => Ok("frnd"),
            Login => Ok("login"),
            Logoff => Ok("logoff"),
            LvlUp => Ok("lvlup"),
            Noemo => Ok("noemo"),
            PvpProtected => Ok("pvpprotected"),
            Respawned => Ok("respawned"),
            Spider => Ok("spider"),
            Stasis | StasisEnd => Ok("stasis"),
            Teleported => Ok("teleported"),
            _ => Err(fmt::Error),
        }?;

        write!(f, "{emotion}")
    }
}

impl FromStr for EmotionName {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use EmotionName::*;

        match s {
            "abbysout" => Ok(AbyssOut),
            "angry" => Ok(Angry),
            "away" => Ok(Away),
            "bat" => Ok(Bat),
            "battle" => Ok(Battle),
            "frnd" => Ok(Frnd),
            "login" => Ok(Login),
            "logoff" => Ok(Logoff),
            "lvlup" => Ok(LvlUp),
            "noemo" => Ok(Noemo),
            "pvpprotected" => Ok(PvpProtected),
            "respawned" => Ok(Respawned),
            "spider" => Ok(Spider),
            "stasis" => Ok(Stasis),
            "teleported" => Ok(Teleported),
            _ => Ok(Undefined),
        }
    }
}

impl EmotionName {
    pub(crate) fn get_duration(self) -> Option<u32> {
        use EmotionName::*;

        match self {
            AbyssOut => Some(2000),
            Angry => Some(8000),
            Bat => Some(8000),
            Battle => None,
            Frnd => Some(3750),
            Login => Some(2000),
            Logoff => Some(4000),
            LvlUp => Some(1000),
            PvpProtected => Some(4000),
            Respawned => Some(2000),
            Away => Some(8000),
            AwayEnd => None,
            Spider => Some(3000),
            Stasis => None,
            StasisEnd => None,
            Teleported => Some(2000),
            Undefined => Some(8000),
            Noemo => None,
        }
    }

    pub(crate) fn is_removable(self) -> bool {
        use EmotionName::*;

        if matches!(self, Battle | Away | Stasis) {
            return false;
        }

        true
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct BusinessCard {
    #[serde(rename = "acc")]
    pub account: Option<u32>,
    //"clan": 11793,
    //"icon": "/noob/mm.gif",
    #[serde(rename = "id")]
    pub char_id: Option<Id>,
    //"lvl": 64,
    //"nick": "George Montez",
    //"oplvl": 64,
    //"prof": "m",
    //"sex": true
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[repr(transparent)]
pub struct BusinessCards(Vec<BusinessCard>);

impl IntoIterator for BusinessCards {
    type Item = BusinessCard;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub(crate) struct ChatMessage {
    code: Option<String>,
    related: Option<Vec<Id>>,
}

impl ChatMessage {
    pub(crate) fn get_parsed_code(&self) -> Option<Value> {
        serde_json::from_str(self.code.as_ref()?).ok()
    }

    pub(crate) fn get_first_related(&self) -> Option<Id> {
        self.related.as_ref()?.first().copied()
    }

    // TODO: Return result ?
    ///None -> something went wrong
    ///Some(false) -> nothing to update
    ///Some(true) -> updated a peers online status successfully
    pub(crate) fn try_update_one_peer(
        &self,
        peers_lock: &mut MutableBTreeMapLockMut<PeerId, super::peer::Peer>,
    ) -> Option<bool> {
        let message_id = self
            .get_parsed_code()?
            .as_object()?
            .get(intern(s!("message")))?
            .as_array()?
            .first()?
            .as_object()?
            .get("id")?
            .as_u64()
            .map(MessageId::from)?;

        if message_id == MessageId::Undefined {
            return Some(false);
        }

        let peer_id = self.get_first_related()?;

        peers_lock
            .update_cloned(&peer_id, |peer_data| {
                peer_data.online.set_neq(match peer_data.relation.get() {
                    Relation::Clan => message_id == MessageId::ClanLogin,
                    Relation::Friend => message_id == MessageId::FriendLogin,
                    _ => unreachable!("Peer has incorrect relation."),
                });
            })
            .map(|_old_peer_data| true)
    }
}

#[derive(Debug, PartialEq)]
enum MessageId {
    ClanLogoff,
    ClanLogin,
    FriendLogoff,
    FriendLogin,
    Undefined,
}

impl From<u64> for MessageId {
    fn from(value: u64) -> Self {
        match value {
            2305000 => Self::ClanLogoff,
            2305001 => Self::ClanLogin,
            2701000 => Self::FriendLogoff,
            2701001 => Self::FriendLogin,
            _ => Self::Undefined,
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub(crate) struct SystemChannel {
    msg: Option<Vec<ChatMessage>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Channels {
    system: Option<SystemChannel>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Chat {
    channels: Option<Channels>,
}

impl Chat {
    pub(crate) fn get_system_messages(&self) -> Option<&Vec<ChatMessage>> {
        self.channels.as_ref()?.system.as_ref()?.msg.as_ref()
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct WorldConfigData {
    #[serde(rename = "worldname")]
    pub world_name: Option<String>,
    #[serde(rename = "npcresp")]
    pub npc_resp: Option<f32>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Ask {
    pub q: Option<String>,
    pub m: Option<String>,
    pub re: Option<String>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(default)]
pub struct Emotion {
    #[serde_as(as = "DisplayFromStr")]
    pub name: EmotionName,
    pub source_id: Id,
    // TODO: EmotionsData.OBJECT_TYPE.OTHER <- map to this
    pub source_type: u8,
    // pub end_ts: Option<u32>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct HeroData {
    #[serde_as(as = "Option<BoolFromInt>")]
    pub back: Option<bool>,
    pub account: Option<Id>,
    // pub attr: Option<u8>,
    // pub bag: Option<u32>,
    // pub bagi: Option<u32>,
    // pub bint: Option<u32>,
    // pub blockade: Option<u32>,
    // pub bstr: Option<u32>,
    pub clan: Option<HeroClan>,
    // pub credits: Option<u32>,
    // pub cur_battle_set: Option<u32>,
    // pub cur_skill_set: Option<u32>,
    // pub dir: Option<u8>,
    // pub exp: Option<u64>,
    // pub gender: Option<String>,
    // pub gold: Option<u64>,
    // pub goldlim: Option<u64>,
    // pub healpower: Option<u32>,
    // pub honor: Option<u32>,
    pub id: Option<Id>,
    // pub img: Option<String>,
    // pub is_blessed: Option<u8>,
    pub lvl: Option<u16>,
    // pub mails: Option<u32>,
    // pub mails_all: Option<u32>,
    // pub mails_last: Option<String>,
    // pub mpath: Option<String>,
    pub nick: Option<String>,
    // pub opt: Option<u32>,
    // pub party: Option<u32>,
    // pub passive_stats: Option<String>,
    // pub prof: Option<String>,
    // pub pvp: Option<u32>,
    // pub runes: Option<u32>,
    // pub stamina: Option<u32>,
    // pub stamina_renew_sec: Option<u32>,
    // pub stamina_ts: Option<u32>,
    #[serde_as(as = "Option<BoolFromInt>")]
    pub stasis: Option<bool>,
    pub stasis_incoming_seconds: Option<u8>,
    // pub trade: Option<u32>,
    // pub ttl: Option<i32>,
    // pub ttl_del: Option<u32>,
    // pub ttl_end: Option<u32>,
    // pub ttl_value: Option<u32>,
    // pub uprawnienia: Option<u32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub vip: Option<u8>,
    // pub wanted: Option<u32>,
    // pub warrior_stats: Option<WarriorStats>,
    pub x: Option<u8>,
    pub y: Option<u8>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(default)]
pub struct HeroClan {
    //pub id: Option<Id>,
    //pub name: Option<String>,
    //pub rank: Option<u8>,
}
//
// #[skip_serializing_none]
// #[derive(Debug, Serialize, Deserialize, Clone, Default)]
// #[serde(default)]
// pub struct WarriorStats {
//     pub ac: Option<u32>,
//     pub acdmg: Option<u32>,
//     pub acmdmg: Option<u8>,
//     pub act: Option<u8>,
//     pub ag: Option<u32>,
//     pub attack: Option<Attack>,
//     pub crit: Option<f32>,
//     pub critval: Option<f32>,
//     pub energy: Option<u32>,
//     pub energygain: Option<u8>,
//     pub evade: Option<Vec<f64>>,
//     pub heal: Option<u32>,
//     pub hp: Option<u32>,
//     pub it: Option<u32>,
//     pub legbon_cleanse: Option<u8>,
//     pub legbon_curse: Option<u8>,
//     pub legbon_dmgred: Option<u8>,
//     pub legbon_holytouch: Option<Vec<u32>>,
//     pub legbon_lastheal: Option<Vec<u32>>,
//     pub lowcrit: Option<f32>,
//     pub lowevade: Option<u8>,
//     pub maxhp: Option<u32>,
//     pub of_crit: Option<f32>,
//     pub of_critval: Option<f32>,
//     pub of_wound0: Option<u32>,
//     pub of_wound1: Option<u32>,
//     pub resfire: Option<u8>,
//     pub resfrost: Option<u8>,
//     pub reslight: Option<u8>,
//     pub sa: Option<f32>,
//     pub slow: Option<u32>,
//     pub st: Option<u32>,
// }
//
// #[skip_serializing_none]
// #[derive(Debug, Serialize, Deserialize, Clone, Default)]
// #[serde(rename_all = "camelCase", default)]
// pub struct Attack {
//     pub physical_main_hand: Option<Damage>,
//     pub physical_off_hand: Option<Damage>,
// }
//
// #[skip_serializing_none]
// #[derive(Debug, Serialize, Deserialize, Clone, Default)]
// #[serde(default)]
// pub struct Damage {
//     pub average: Option<u32>,
//     pub max: Option<u32>,
//     pub min: Option<u32>,
// }

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Item {
    #[serde(
        serialize_with = "item_class_serialize",
        deserialize_with = "item_class_deserialize"
    )]
    pub cl: Option<ItemClass>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub del: Option<u8>,
    // pub hid: Option<String>,
    // pub icon: Option<String>,
    pub loc: Option<String>,
    pub name: Option<String>,
    // pub own: Option<u32>,
    // pub pr: Option<u32>,
    // pub prc: Option<String>,
    pub st: Option<u8>,
    pub stat: Option<String>,
    // pub tpl: Option<u32>,
    pub x: Option<u16>,
    pub y: Option<u16>,
    #[serde(skip)]
    pub(crate) disabled: Mutable<bool>,
}

pub fn item_class_serialize<S>(item: &Option<ItemClass>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match item {
        Some(item_class) => serializer.serialize_u8(*item_class as u8),
        None => serializer.serialize_none(),
    }
}

pub fn item_class_deserialize<'de, D>(deserializer: D) -> Result<Option<ItemClass>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionItemClassVisitor;

    impl<'de> Visitor<'de> for OptionItemClassVisitor {
        type Value = Option<ItemClass>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer between 1 and 32 or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct ItemClassVisitor;

            impl Visitor<'_> for ItemClassVisitor {
                type Value = ItemClass;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("an integer between 1 and 32")
                }

                fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    match value {
                        1 => Ok(ItemClass::OneHandWeapon),
                        2 => Ok(ItemClass::TwoHandWeapon),
                        3 => Ok(ItemClass::OneAndHalfHandWeapon),
                        4 => Ok(ItemClass::DistanceWeapon),
                        5 => Ok(ItemClass::HelpWeapon),
                        6 => Ok(ItemClass::WandWeapon),
                        7 => Ok(ItemClass::OrbWeapon),
                        8 => Ok(ItemClass::Armor),
                        9 => Ok(ItemClass::Helmet),
                        10 => Ok(ItemClass::Boots),
                        11 => Ok(ItemClass::Gloves),
                        12 => Ok(ItemClass::Ring),
                        13 => Ok(ItemClass::Necklace),
                        14 => Ok(ItemClass::Shield),
                        15 => Ok(ItemClass::Neutral),
                        16 => Ok(ItemClass::Consume),
                        17 => Ok(ItemClass::Gold),
                        18 => Ok(ItemClass::Keys),
                        19 => Ok(ItemClass::Quest),
                        20 => Ok(ItemClass::Renewable),
                        21 => Ok(ItemClass::Arrows),
                        22 => Ok(ItemClass::Talisman),
                        23 => Ok(ItemClass::Book),
                        24 => Ok(ItemClass::Bag),
                        25 => Ok(ItemClass::Bless),
                        26 => Ok(ItemClass::Upgrade),
                        27 => Ok(ItemClass::Recipe),
                        28 => Ok(ItemClass::Coinage),
                        29 => Ok(ItemClass::Quiver),
                        30 => Ok(ItemClass::Outfits),
                        31 => Ok(ItemClass::Pets),
                        32 => Ok(ItemClass::Teleports),
                        _ => Err(E::custom(format!("invalid item class code: {}", value))),
                    }
                }

                fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    self.visit_u64(value as u64)
                }

                fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    self.visit_u64(value as u64)
                }

                fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    self.visit_u64(value as u64)
                }
            }

            deserializer.deserialize_u8(ItemClassVisitor).map(Some)
        }

        // Handle null values from JSON
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_option(OptionItemClassVisitor)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DamageType {
    Poison,
    Wound,
    Fire,
    Frost,
    Light,
    #[default]
    Undefined,
}

impl DamageType {
    pub(crate) fn into_class(self) -> Option<&'static str> {
        match self {
            Self::Undefined => None,
            Self::Fire => Some("fire"),
            Self::Frost => Some("frost"),
            Self::Wound => Some("wound"),
            Self::Poison => Some("poison"),
            Self::Light => Some("light"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Rarity {
    Common,
    Unique,
    Heroic,
    Upgraded,
    Legendary,
    #[default]
    Artifact,
}

impl<'a> From<&'a str> for Rarity {
    fn from(value: &'a str) -> Self {
        match value {
            "common" => Self::Common,
            "unique" => Self::Unique,
            "heroic" => Self::Heroic,
            "upgraded" => Self::Upgraded,
            "legendary" => Self::Legendary,
            _ => Self::Artifact,
        }
    }
}

impl From<Rarity> for &'static str {
    fn from(value: Rarity) -> Self {
        match value {
            Rarity::Common => "common",
            Rarity::Unique => "unique",
            Rarity::Heroic => "heroic",
            Rarity::Upgraded => "upgraded",
            Rarity::Legendary => "legendary",
            Rarity::Artifact => "artifact",
        }
    }
}

impl<'a> From<&'a Rarity> for &'static str {
    fn from(value: &'a Rarity) -> Self {
        match value {
            Rarity::Common => "common",
            Rarity::Unique => "unique",
            Rarity::Heroic => "heroic",
            Rarity::Upgraded => "upgraded",
            Rarity::Legendary => "legendary",
            Rarity::Artifact => "artifact",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum BindType {
    Binds,
    SoulBound,
    PermBound,
}
// TODO: Check target_rarity for e3 drops.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum TargetRarity {
    Common,
    Unique,
    Heroic,
    Upgraded,
    Legendary,
}

impl FromStr for TargetRarity {
    type Err = f64;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use TargetRarity::*;

        match s {
            "common" => Ok(Common),
            "unique" => Ok(Unique),
            "heroic" => Ok(Heroic),
            "upgraded" => Ok(Upgraded),
            "legendary" => Ok(Legendary),
            _ => Err(err_code!(as_num)),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ItemStats {
    pub(crate) amount: Option<u32>,
    /// Describes whether an item comes from any event from the game or no.
    /// Not present in game responses.
    pub(crate) from_event: bool,
    /// Whether this is a limited use item.
    pub cursed: bool,
    /// Describes an item which can be used for reselecting bonuses of fully upgraded items.
    /// Not present by default.
    pub bonus_reselect: bool,
    /// Without this property items can be used for artisanship so `Option` is not needed here.
    pub(crate) artisan_worthless: bool,
    /// Indicates whether an item has a user defined description or image.
    /// Not present by default.
    pub(crate) personal: bool,
    /// Determines item rarities an item with `ItemClass::Upgrade` can be used on.
    /// If not present on an item with `ItemClass::Upgrade`, the upgrade can be used on any item
    /// rarity.
    pub target_rarity: Option<TargetRarity>,
    ///(map_id, teleport_x, teleport_y, map_name)
    pub(crate) custom_teleport: Option<(Id, u8, u8, String)>,
    pub bind: Option<BindType>,
    pub(crate) rarity: Rarity,
    pub lvl: Option<i32>,
    pub enhancement_upgrade_lvl: Option<u8>,
    pub bonus_not_selected: bool,
    #[serde(skip)]
    pub(crate) dmg_type: DamageType,
}

impl ItemStats {
    fn new(item_stat: &str) -> Self {
        let mut default = Self::default();

        if EVENT_STATS.iter().any(|event| item_stat.contains(event)) {
            default.from_event = true;
        }

        default
    }

    fn parse_one_stat(dest: &mut Self, stat_attribute: &str, attribute_value: &str) {
        //debug_log!(stat_attribute, "=> \"", attribute_value, "\"");
        match stat_attribute {
            "bonus_reselect" => dest.bonus_reselect = true,
            "target_rarity" => {
                dest.target_rarity = Some(TargetRarity::from_str(attribute_value).unwrap_js())
            }
            "amount" => dest.amount = Some(attribute_value.parse::<u32>().unwrap_or_default()),
            "binds" => dest.bind = Some(BindType::Binds),
            "soulbound" => dest.bind = Some(BindType::SoulBound),
            "permbound" => dest.bind = Some(BindType::PermBound),
            "personal" => dest.personal = true,
            "artisan_worthless" => dest.artisan_worthless = true,
            // "bonus_reselect" => dest.bonus_reselect = Some(true),
            // "cansplit" => dest.cansplit = Some(attribute_value.eq("1")),
            // "capacity" => {
            //     dest.capacity =
            //         Some(attribute_value.parse::<u32>().unwrap_or_default())
            // }
            "custom_teleport" => {
                dest.custom_teleport = Some(Self::parse_custom_teleport(attribute_value))
            }
            "poison" => dest.dmg_type = DamageType::Poison,
            "wound" => dest.dmg_type = DamageType::Wound,
            "fire" => dest.dmg_type = DamageType::Fire,
            "frost" => dest.dmg_type = DamageType::Frost,
            "light" => dest.dmg_type = DamageType::Light,
            "rarity" => dest.rarity = attribute_value.into(),
            "cursed" => dest.cursed = true,
            "lvl" => dest.lvl = attribute_value.parse::<i32>().ok(),
            "enhancement_upgrade_lvl" => {
                dest.enhancement_upgrade_lvl = attribute_value.parse::<u8>().ok()
            }
            "bonus_not_selected" => dest.bonus_not_selected = true,
            _ => (),
        }
    }

    fn parse_custom_teleport(teleport_data: &str) -> (Id, u8, u8, String) {
        let mut map_id = String::new();
        let mut x = String::new();
        let mut y = String::new();
        let mut map_name = String::new();
        let mut write_type: u8 = 0;
        for char in teleport_data.chars() {
            match write_type {
                0 if char == ',' => write_type += 1,
                0 => map_id.push(char),
                1 if char == ',' => write_type += 1,
                1 => x.push(char),
                2 if char == ',' => write_type += 1,
                2 => y.push(char),
                3 if char != ',' => map_name.push(char),
                _ => break,
            }
        }

        (
            map_id.parse().unwrap_or_default(),
            x.parse().unwrap_or_default(),
            y.parse().unwrap_or_default(),
            map_name,
        )
    }
}

impl Item {
    pub(crate) fn merge(&mut self, item: &mut Item) {
        let Self {
            del,
            loc,
            stat,
            x,
            y,
            st,
            ..
        } = self;

        if item.loc.is_some() {
            *loc = item.loc.take();
        }
        if item.del.is_some() {
            *del = item.del.take();
        }
        if item.stat.is_some() {
            *stat = item.stat.take();
        }
        if item.x.is_some() {
            *x = item.x.take();
        }
        if item.y.is_some() {
            *y = item.y.take();
        }
        if item.st.is_some() {
            *st = item.st.take();
        }
    }

    pub fn parse_stats(&self) -> Option<ItemStats> {
        let item_stat = self.stat.as_ref()?;
        let mut current_stats = ItemStats::new(item_stat);
        let mut stat_attribute = String::new();
        let mut attribute_value = String::new();
        let mut write_name = true;

        //debug_log!("item_stat", item_stat.as_str());
        for char in item_stat.chars() {
            if char == ';' {
                ItemStats::parse_one_stat(&mut current_stats, &stat_attribute, &attribute_value);
                write_name = true;
                stat_attribute.clear();
                attribute_value.clear();
                continue;
            }
            if write_name {
                if char == '=' {
                    write_name = false;
                    continue;
                }
                stat_attribute.push(char);
                continue;
            }

            attribute_value.push(char);
        }

        //debug_log!("stat_attribute:", &stat_attribute);
        ItemStats::parse_one_stat(&mut current_stats, &stat_attribute, &attribute_value);

        Some(current_stats)
    }

    pub(crate) fn get_bag_slot(&self) -> Option<EquipmentSlot> {
        if EquipmentSlot::from(self.st?) != EquipmentSlot::InBag {
            return None;
        }

        let y = self.y?;

        if y < 6 {
            Some(EquipmentSlot::FirstBagSlot)
        } else if y < 12 {
            Some(EquipmentSlot::SecondBagSlot)
        } else if y <= 17 {
            Some(EquipmentSlot::ThirdBagSlot)
        } else if (36..=41).contains(&y) {
            Some(EquipmentSlot::SpecialBagSlot)
        } else {
            None
        }
    }
}

pub(crate) trait PeerData {
    fn id(&self) -> Id;
    fn lvl(&mut self) -> u16;
    fn oplvl(&mut self) -> u16;
    fn nick(&mut self) -> String;
    fn prof(&mut self) -> Profession;
    fn x(&mut self) -> u8;
    fn y(&mut self) -> u8;
    fn map_name(&mut self) -> String;
    fn relation(&self) -> Relation;
    fn is_online(&self) -> bool;
}

macro_rules! make_peer {
    ($peer_name:ident, $relation:expr, { fn is_online(&$this:ident) -> bool $code:block }) => {
        impl PeerData for $peer_name {
            #[inline]
            fn id(&self) -> Id {
                self.id
            }

            #[inline]
            fn lvl(&mut self) -> u16 {
                std::mem::take(&mut self.lvl)
            }

            #[inline]
            fn oplvl(&mut self) -> u16 {
                std::mem::take(&mut self.oplvl)
            }

            #[inline]
            fn nick(&mut self) -> String {
                std::mem::take(&mut self.nick)
            }

            #[inline]
            fn prof(&mut self) -> Profession {
                std::mem::take(&mut self.prof)
            }

            #[inline]
            fn x(&mut self) -> u8 {
                std::mem::take(&mut self.x)
            }

            #[inline]
            fn y(&mut self) -> u8 {
                std::mem::take(&mut self.y)
            }

            #[inline]
            fn map_name(&mut self) -> String {
                std::mem::take(&mut self.map_name)
            }

            #[inline]
            fn relation(&self) -> Relation {
                $relation
            }

            #[inline]
            fn is_online(&$this) -> bool $code
        }
    };
}

// TODO: Use enum for online status
// TODO: Use game names from SocietyItem module for fields ?
#[derive(Debug, Clone)]
pub struct Friend {
    pub id: Id,
    pub nick: String,
    pub outfit_path: String,
    pub lvl: u16,
    pub oplvl: u16,
    pub prof: Profession,
    pub map_name: String,
    pub x: u8,
    pub y: u8,
    pub online_status: String,
    pub last_online: u64,
}

make_peer!(Friend, Relation::Friend, {
    fn is_online(&self) -> bool {
        self.online_status == wasm_bindgen::intern(obfstr::obfstr!("online"))
    }
});

impl Serialize for Friend {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(11))?;
        seq.serialize_element(&(self.id as f64))?;
        seq.serialize_element(&self.nick)?;
        seq.serialize_element(&self.outfit_path)?;
        seq.serialize_element(&(self.lvl as f64))?;
        seq.serialize_element(&(self.oplvl as f64))?;
        seq.serialize_element(&self.prof)?;
        seq.serialize_element(&self.map_name)?;
        seq.serialize_element(&(self.x as f64))?;
        seq.serialize_element(&(self.y as f64))?;
        seq.serialize_element(&self.online_status)?;
        seq.serialize_element(&(self.last_online as f64))?;
        seq.end()
    }
}

pub fn friends_de<'de, D>(deserializer: D) -> Result<Option<Vec<Friend>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct FriendVecVisitor;

    impl<'de> Visitor<'de> for FriendVecVisitor {
        type Value = Option<Vec<Friend>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a flat array representing multiple friends")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut friends = Vec::new();
            while let Some(id) = seq
                .next_element::<String>()?
                .and_then(|d| d.parse::<Id>().ok())
            {
                let nick = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let outfit_path = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let lvl = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?
                    .parse()
                    .map_err(de::Error::custom)?;
                let oplvl = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(5, &self))?
                    .parse()
                    .map_err(de::Error::custom)?;
                let prof = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(6, &self))?
                    .parse()
                    .map_err(de::Error::custom)?;
                let map_name = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(7, &self))?;
                let x = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(8, &self))?
                    .parse()
                    .map_err(de::Error::custom)?;
                let y = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(9, &self))?
                    .parse()
                    .map_err(de::Error::custom)?;
                let online_status = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(10, &self))?;
                let last_online = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(11, &self))?
                    .parse()
                    .map_err(de::Error::custom)?;

                friends.push(Friend {
                    id,
                    nick,
                    outfit_path,
                    lvl,
                    oplvl,
                    prof,
                    map_name,
                    x,
                    y,
                    online_status,
                    last_online,
                });
            }

            Ok(Some(friends))
        }
    }

    deserializer.deserialize_seq(FriendVecVisitor)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NpcDelData {
    pub id: Option<Id>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcData {
    pub id: Option<Id>,
    #[serde(rename = "tpl")]
    pub template_id: Option<Id>,
    pub x: Option<u8>,
    pub y: Option<u8>,
    pub walkover: Option<bool>,
    pub group: Option<u16>,
}

impl NpcData {
    pub fn has_collision(&self) -> Option<bool> {
        let templates_lock = NpcTemplates::get().lock_ref();
        let npc_tpl = templates_lock.get(&self.template_id?)?;
        let npc_type = npc_tpl.npc_type?;

        Some(npc_type != 4 && npc_type != 7 && !self.walkover?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcTemplate {
    pub id: Option<Id>,
    // TODO: Should this be a u8 newtype wrapper ?
    #[serde(rename = "warrior_type")]
    pub warrior_type: Option<i32>,
    #[serde(rename = "type")]
    pub npc_type: Option<i32>,
    pub nick: Option<String>,
    pub level: Option<u16>,
    pub elastic_level_factor: Option<i8>, // Should this be an u8?
                                          //srajId: Option<i32>
                                          //nick: String,
                                          //level: Option<u16>
                                          //prof: Option<Profession>
}

#[derive(Debug, Clone)]
pub struct ClanMember {
    pub id: Id,
    pub nick: String,
    pub lvl: u16,
    pub oplvl: u16,
    pub prof: Profession,
    pub map_name: String,
    pub x: u8,
    pub y: u8,
    pub clan_rank_id: u8,
    pub last_online: u64,
    pub outfit_path: String,
}

make_peer!(ClanMember, Relation::Clan, {
    fn is_online(&self) -> bool {
        self.last_online == 0
    }
});

impl Serialize for ClanMember {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(10))?;
        seq.serialize_element(&(self.id as f64))?;
        seq.serialize_element(&self.nick)?;
        seq.serialize_element(&(self.lvl as f64))?;
        seq.serialize_element(&(self.oplvl as f64))?;
        seq.serialize_element(&self.prof)?;
        seq.serialize_element(&self.map_name)?;
        seq.serialize_element(&(self.x as f64))?;
        seq.serialize_element(&(self.y as f64))?;
        seq.serialize_element(&(self.clan_rank_id as f64))?;
        seq.serialize_element(&(self.last_online as f64))?;
        seq.serialize_element(&self.outfit_path)?;
        seq.end()
    }
}

pub fn members_de<'de, D>(deserializer: D) -> Result<Option<Vec<ClanMember>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ClanMemberVecVisitor;

    impl<'de> Visitor<'de> for ClanMemberVecVisitor {
        type Value = Option<Vec<ClanMember>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a flat array representing multiple clan members")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut members = Vec::new();
            while let Some(id) = seq.next_element::<Id>()? {
                let nick = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let lvl = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let oplvl = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let prof = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(5, &self))?
                    .parse()
                    .map_err(de::Error::custom)?;
                let map_name = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(6, &self))?;
                let x = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(7, &self))?;
                let y = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(8, &self))?;
                let clan_rank_id = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(9, &self))?;
                let last_online = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(9, &self))?;
                let outfit_path = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(10, &self))?;

                members.push(ClanMember {
                    id,
                    nick,
                    lvl,
                    oplvl,
                    prof,
                    map_name,
                    x,
                    y,
                    clan_rank_id,
                    last_online,
                    outfit_path,
                });
            }

            let members = match members.is_empty() {
                true => None,
                false => Some(members),
            };
            Ok(members)
        }
    }

    deserializer.deserialize_seq(ClanMemberVecVisitor)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Profession {
    BladeDancer = 1,
    Hunter,
    Mage,
    Paladin,
    Tracker,
    #[default]
    Warrior,
}

impl Serialize for Profession {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl fmt::Display for Profession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Profession::*;
        match self {
            Warrior => write!(f, "w"),
            Mage => write!(f, "m"),
            Paladin => write!(f, "p"),
            Hunter => write!(f, "h"),
            Tracker => write!(f, "t"),
            BladeDancer => write!(f, "b"),
        }
    }
}

impl FromStr for Profession {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Profession::*;

        match s {
            "w" => Ok(Warrior),
            "m" => Ok(Mage),
            "p" => Ok(Paladin),
            "h" => Ok(Hunter),
            "t" => Ok(Tracker),
            "b" => Ok(BladeDancer),
            _ => Err("expected a valid profession character"),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct OtherDataEmotions {
    pub(crate) stasis: Option<Emotion>,
    pub(crate) stasis_incoming: Option<Emotion>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OtherAction {
    Create,
    Undefined,
}

impl FromStr for OtherAction {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use OtherAction::*;

        match s {
            "CREATE" => Ok(Create),
            _ => Ok(Undefined),
        }
    }
}

impl fmt::Display for OtherAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CREATE")
    }
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct OtherData {
    pub account: Option<u32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub action: Option<OtherAction>,
    // pub attr: Option<u8>,
    pub del: Option<u8>,
    // pub dir: Option<u8>,
    // pub icon: Option<String>,
    pub clan: Option<Clan>,
    // pub is_blessed: Option<u8>,
    pub lvl: Option<u16>,
    #[serde(rename = "oplvl")]
    pub operational_lvl: Option<u16>,
    pub nick: Option<String>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub prof: Option<Profession>,
    pub relation: Option<Relation>,
    // pub rights: Option<u8>,
    #[serde_as(as = "Option<BoolFromInt>")]
    pub stasis: Option<bool>,
    #[serde(rename = "stasis_incoming_seconds")]
    pub stasis_incoming_seconds: Option<u8>,
    // pub vip: Option<String>,
    // pub who_is_here: Option<String>,
    pub x: Option<u8>,
    pub y: Option<u8>,
    #[serde_as(as = "Option<BoolFromInt>")]
    pub(crate) wanted: Option<bool>,
}

impl PartialEq for OtherData {
    fn eq(&self, other: &Self) -> bool {
        self.account.unwrap_or_default() == other.account.unwrap_or_default()
    }
}

impl OtherData {
    // TODO: Send `EmotionName::AwayEnd` only if the sleep has ended.
    pub(crate) fn parse_emotions(
        &self,
        char_id: OtherId,
        was_in_stasis: bool,
    ) -> OtherDataEmotions {
        let mut other_emotions = OtherDataEmotions::default();

        match self.stasis {
            Some(true) => {
                other_emotions.stasis = Some(Emotion {
                    name: EmotionName::Stasis,
                    source_id: char_id,
                    source_type: 1,
                })
            }
            Some(false) => {
                if was_in_stasis {
                    other_emotions.stasis = Some(Emotion {
                        name: EmotionName::StasisEnd,
                        source_id: char_id,
                        source_type: 1,
                    });
                }
            }
            None => {}
        };
        match self.stasis_incoming_seconds {
            Some(0) => {
                other_emotions.stasis_incoming = Some(Emotion {
                    name: EmotionName::AwayEnd,
                    source_id: char_id,
                    source_type: 1,
                });
            }
            Some(_) => {
                other_emotions.stasis_incoming = Some(Emotion {
                    name: EmotionName::Away,
                    source_id: char_id,
                    source_type: 1,
                });
            }
            None => {}
        };

        other_emotions
    }

    /////Determines whether a player is in hero's party.
    //pub(crate) fn in_party(&self) -> bool {
    //    let Some(party) = get_engine().party() else {
    //        return false;
    //    };
    //
    //    party
    //        .get_members()
    //        .has_own_property(&JsValue::from_f64(self.char_id as f64))
    //}
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Clan {
    pub(crate) id: i32,
    pub(crate) name: String,
}

impl PartialEq for Clan {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct PartyData {
    #[serde_as(as = "Option<BTreeMap<DisplayFromStr, _>>")]
    pub members: Option<BTreeMap<OtherId, PartyMemberData>>,
    // pub partyexp: Option<u16>,
    // pub partygrpkill: Option<u32>, // what does this do?
}

// TODO: Rename to Details ?
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct PartyMemberData {
    pub account: u32,
    #[serde_as(as = "BoolFromInt")]
    pub commander: bool,
    pub hp_cur: u32,
    pub hp_max: u32,
    pub icon: String,
    #[serde(rename = "id")]
    pub char_id: OtherId,
    pub nick: String,
}
