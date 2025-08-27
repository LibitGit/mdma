pub mod battle;
pub mod chat;
pub mod communication;
pub mod disable_items;
pub mod hero;
pub mod hero_eq;
pub mod idle_json;
pub mod iframe_window;
pub mod interface;
pub mod items;
pub mod items_mark;
pub mod lock_manager;
pub mod log_off;
pub mod map;
pub mod other;
pub mod renderer;
pub mod settings;
pub mod skills;
pub mod steps_to_send;
pub mod types;
// TODO: Move the Other impl to "other" module and move the current contents of
// other module somewhere else.
pub mod others;
pub mod party;
pub mod peer;
pub mod show_eq;
pub mod targets;
pub mod who_is_here;

#[cfg(not(feature = "ni"))]
use common::err_code;
use wasm_bindgen::prelude::*;

use crate::utils::JsResult;
#[cfg(not(feature = "ni"))]
use crate::utils::UnwrapJsExt;

use super::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `Engine` object."]
    pub type Engine;

    #[cfg(feature = "ni")]
    #[wasm_bindgen(method, getter = "apiData")]
    pub fn api_data(this: &Engine) -> Option<super::api::ApiData>;

    #[wasm_bindgen(method, getter = "iframeWindowManager")]
    pub fn inline_frame_window_manager(this: &Engine)
    -> Option<iframe_window::IFrameWindowManager>;

    #[wasm_bindgen(method, getter = "targets")]
    pub fn targets(this: &Engine) -> Option<targets::Targets>;

    #[wasm_bindgen(method, getter = "whoIsHere")]
    pub fn who_is_here(this: &Engine) -> Option<who_is_here::WhoIsHere>;

    #[wasm_bindgen(method, getter = "disableItemsManager")]
    pub fn disable_items_manager(this: &Engine) -> Option<disable_items::DisableItemsManager>;

    #[wasm_bindgen(method, getter = "itemsMarkManager")]
    pub fn items_mark_manager(this: &Engine) -> Option<items_mark::ItemsMarkManager>;

    #[wasm_bindgen(method, getter = "showEqManager")]
    pub fn show_eq_manager(this: &Engine) -> Option<show_eq::ShowEqManager>;

    // TODO: Check if this is correct after party gets destroyed.
    #[wasm_bindgen(method, getter = "party")]
    fn __party(this: &Engine) -> Option<party::PartyManager>;

    #[wasm_bindgen(method, getter = "renderer")]
    pub fn renderer(this: &Engine) -> Option<renderer::Renderer>;

    #[wasm_bindgen(method, getter)]
    pub fn interface(this: &Engine) -> Option<interface::Interface>;

    #[wasm_bindgen(method, getter = "others")]
    pub fn others(this: &Engine) -> Option<others::OthersManager>;

    #[wasm_bindgen(method, getter = "hero")]
    pub fn hero(this: &Engine) -> Option<hero::Hero>;

    #[wasm_bindgen(method, getter)]
    pub fn settings(this: &Engine) -> Option<settings::Settings>;

    #[wasm_bindgen(method, getter = "map")]
    pub fn map(this: &Engine) -> Option<map::Map>;

    #[wasm_bindgen (extends = ::js_sys::Function)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type SendRequest;

    #[wasm_bindgen(method, catch, js_name = "call")]
    pub fn send_task(this: &SendRequest, context: &Communication, task: &str) -> JsResult<JsValue>;

    #[wasm_bindgen(method, catch, js_name = "call")]
    pub fn send_task_with_callback(
        this: &SendRequest,
        context: &Communication,
        task: &str,
        callback: &JsValue,
    ) -> JsResult<JsValue>;

    #[wasm_bindgen(method, catch, js_name = "call")]
    pub fn send_task_with_callback_and_payload(
        this: &SendRequest,
        context: &Communication,
        task: &str,
        callback: &JsValue,
        payload: &JsValue,
    ) -> JsResult<JsValue>;
}

#[cfg(feature = "ni")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, method, js_name = "getEv")]
    pub fn get_ev(this: &Engine) -> JsResult<f64>;

    #[wasm_bindgen(method, js_name = "setEv")]
    pub fn set_ev(this: &Engine, ev: f64);

    #[wasm_bindgen(method, getter = "allInit")]
    pub fn get_all_init(this: &Engine) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    pub fn communication(this: &Engine) -> Option<Communication>;

    #[wasm_bindgen(method, setter = "send2")]
    pub fn set_send(this: &Communication, value: &SendRequest);

    #[wasm_bindgen(method, getter = "send2")]
    pub fn get_send(this: &Communication) -> Option<SendRequest>;

    #[wasm_bindgen(catch, method, js_name = "send2")]
    pub fn send(this: &Communication, task: &str) -> JsResult<JsValue>;

    #[wasm_bindgen(method, getter = "items")]
    pub fn items_manager(this: &Engine) -> Option<items::ItemsManager>;

    #[wasm_bindgen(method, getter = "windowMaxZIndex")]
    pub fn window_max_z_index(this: &Engine) -> Option<f64>;

    #[wasm_bindgen(method, setter = "windowMaxZIndex")]
    pub fn set_window_max_z_index(this: &Engine, z_index: f64);

    #[wasm_bindgen(method, getter = "battle")]
    pub fn battle(this: &Engine) -> Option<battle::Battle>;

    #[wasm_bindgen(method, getter = "idleJSON")]
    pub fn idle_json(this: &Engine) -> Option<idle_json::IdleJSON>;

    #[wasm_bindgen(method, getter = "lock")]
    pub fn lock_manager(this: &Engine) -> Option<lock_manager::LockManager>;

    #[wasm_bindgen(method, getter = "chatController")]
    pub fn chat_controller(this: &Engine) -> Option<chat::ChatController>;

    #[wasm_bindgen(method, getter = "stepsToSend")]
    pub fn steps_to_send(this: &Engine) -> Option<steps_to_send::StepsToSend>;

    #[wasm_bindgen(method, getter = "heroEquipment")]
    pub fn hero_equipment(this: &Engine) -> Option<hero_eq::HeroEquipment>;

    #[wasm_bindgen(method, getter = "skills")]
    pub fn skills(this: &Engine) -> Option<skills::SkillsManager>;

    #[wasm_bindgen(method, getter = "logOff")]
    pub fn log_off(this: &Engine) -> Option<log_off::LogOff>;
}

#[cfg(not(feature = "ni"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `g` object on the game's old interface."]
    pub type OldEngine;

    #[wasm_bindgen(method, getter = "g")]
    pub fn g(this: &Engine) -> Option<OldEngine>;

    #[wasm_bindgen(method, getter = "ev")]
    pub fn get_ev(this: &OldEngine) -> JsValue;

    #[wasm_bindgen(method, setter = "ev")]
    pub fn set_ev(this: &OldEngine, ev: f64);

    #[wasm_bindgen(method, getter = "skills")]
    pub fn skills(this: &OldEngine) -> Option<skills::SkillsManager>;

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type MouseMove;

    #[wasm_bindgen(method, getter = "mouseMove")]
    pub fn get_mouse_move(this: &OldEngine) -> Option<MouseMove>;

    #[wasm_bindgen(method, getter = "active")]
    pub fn get_active(this: &MouseMove) -> Option<bool>;

    #[wasm_bindgen(method, setter = "active")]
    pub fn set_active(this: &MouseMove, active: bool);

    #[wasm_bindgen(method, getter = "init")]
    pub fn get_init(this: &OldEngine) -> Option<f64>;

    #[wasm_bindgen(method, getter = "sendRequest")]
    pub fn get_send(this: &Communication) -> Option<SendRequest>;

    #[wasm_bindgen(method, setter = "sendRequest")]
    pub fn set_send(this: &Communication, value: &SendRequest);

    #[wasm_bindgen(catch, method, js_name = "sendRequest")]
    pub fn send(this: &Communication, task: &str) -> JsResult<JsValue>;

    #[wasm_bindgen(method, getter = "battle")]
    pub fn battle(this: &OldEngine) -> JsValue;

    #[wasm_bindgen(method, getter = "delays")]
    pub fn idle_json(this: &OldEngine) -> Option<idle_json::IdleJSON>;

    #[wasm_bindgen(method, getter = "lock")]
    pub fn lock_manager(this: &OldEngine) -> Option<lock_manager::LockManager>;

    #[wasm_bindgen(method, getter = "chatController")]
    pub fn chat_controller(this: &OldEngine) -> Option<chat::ChatController>;

    #[wasm_bindgen(method, getter = "logoff")]
    fn __log_off(this: &OldEngine) -> Option<log_off::LogOff>;
}

#[cfg(not(feature = "ni"))]
use std::cell::RefCell;

#[cfg(not(feature = "ni"))]
thread_local!(static WINDOW_MAX_Z_INDEX: RefCell<Option<f64>> = const { RefCell::new(Some(400.0)) });

#[cfg(not(feature = "ni"))]
impl Engine {
    #[inline]
    pub fn get_ev(&self) -> JsResult<f64> {
        Ok(self
            .g()
            .ok_or_else(|| err_code!())?
            .get_ev()
            .unchecked_into_f64())
    }

    pub fn battle(&self) -> Option<battle::Battle> {
        self.g()?.battle().dyn_into().ok()
    }

    #[inline]
    pub fn set_ev(&self, ev: f64) {
        self.g().unwrap_js().set_ev(ev);
    }

    #[inline]
    pub fn get_all_init(&self) -> Option<bool> {
        self.g()?.get_init().map(|init_lvl| init_lvl == 5.0)
    }

    pub fn skills(&self) -> Option<skills::SkillsManager> {
        let skills = self.g()?.skills()?;

        (skills.get_skills().length() > 0).then_some(skills)
    }

    #[inline]
    pub fn communication(&self) -> Option<Communication> {
        Some(self.clone().unchecked_into())
    }

    // #[inline]
    // pub fn items_manager(&self) -> Option<items::ItemsManager> {
    //     Some(self.clone().unchecked_into())
    // }

    #[inline]
    pub fn window_max_z_index(&self) -> Option<f64> {
        WINDOW_MAX_Z_INDEX.with_borrow_mut(|z_index| *z_index)
    }

    #[inline]
    pub fn set_window_max_z_index(&self, z_index: f64) {
        WINDOW_MAX_Z_INDEX.with_borrow_mut(|old| *old = Some(z_index))
    }

    #[inline]
    pub fn idle_json(&self) -> Option<idle_json::IdleJSON> {
        self.g()?.idle_json()
    }

    pub fn lock_manager(&self) -> Option<lock_manager::LockManager> {
        self.g()?.lock_manager()
    }

    pub fn chat_controller(&self) -> Option<chat::ChatController> {
        self.g()?.chat_controller()
    }

    #[cfg(feature = "antyduch")]
    pub fn steps_to_send(&self) -> Option<steps_to_send::StepsToSend> {
        Some(self.hero()?.unchecked_into())
    }

    #[inline]
    pub fn hero_equipment(&self) -> Option<hero_eq::HeroEquipment> {
        Some(self.g()?.unchecked_into())
    }

    #[inline]
    pub fn log_off(&self) -> Option<log_off::LogOff> {
        let log_off = self.g()?.__log_off()?;

        log_off.start().is_some().then_some(log_off)
    }
}

impl Engine {
    // pub fn party(&self) -> Option<party::PartyManager> {
    //     let party_or_bool = self.__party()?;
    //     if party_or_bool.js_typeof() == intern(s!("object")) {
    //         return Some(party_or_bool);
    //     }

    //     None
    // }
}

//impl Interface {
//    pub fn get_game_canvas(&self) -> JsResult<CanvasRenderingContext2d> {
//        let canvas = self.__get_game_canvas().map_err(map_err!())?.get(0);
//
//        if canvas.is_undefined() {
//            return Err(err_code!());
//        }
//
//        canvas
//            .unchecked_into::<HtmlCanvasElement>()
//            .get_context(intern("2d"))
//            .map_err(map_err!())?
//            .map(JsCast::unchecked_into)
//            .ok_or_else(|| err_code!())
//    }
//}
