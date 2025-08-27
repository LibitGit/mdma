#[cfg(feature = "ni")]
pub mod api;
pub mod engine;

use engine::Engine;
use js_sys::{Function, Object};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[cfg(feature = "ni")]
use crate::utils::UnwrapJsExt;
use crate::utils::{DefaultResult, JsResult};

pub mod prelude {
    #[cfg(feature = "ni")]
    pub use crate::bindings::get_game_api;
    pub use crate::bindings::{
        AskAlertData,
        ask_alert,
        engine::communication::{self, *},
        engine::other::*,
        // engine::party::*,
        engine::peer::*,
        engine::types::*,
        get_engine,
        message,
    };
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::web_sys::EventTarget, extends = ::js_sys::Object, js_name = Window, typescript_type = "Window")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `Window` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Window)"]
    pub type Window;

    #[wasm_bindgen(catch, method)]
    fn message(this: &Window, msg: &str) -> JsResult<JsValue>;

    #[wasm_bindgen(catch, method, js_name = "isPl")]
    fn __is_pl(this: &Window) -> JsResult<bool>;

    #[wasm_bindgen(catch, method, js_name = "askAlert")]
    fn ask_alert(this: &Window, data: &Object) -> JsResult<JsValue>;

    #[wasm_bindgen(catch, method)]
    pub(crate) fn _g(this: &Window, task: &str) -> JsResult<JsValue>;

    #[cfg(feature = "ni")]
    #[wasm_bindgen(method, getter = "API")]
    pub(crate) fn game_api(this: &Window) -> Option<api::GameApi>;

    #[wasm_bindgen(method, getter = "Engine")]
    pub(crate) fn engine(this: &Window) -> Option<Engine>;
}

#[cfg(not(feature = "ni"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, method, js_name = "mAlert")]
    #[doc = "@param {Array} [callbacks= [ () => {}, () => {}] ]"]
    #[doc = ""]
    #[doc = "@param {any} [hmode=null]"]
    fn m_alert(
        this: &Window,
        txt: &str,
        mode: u8,
        callbacks: &js_sys::Array,
        hmode: &JsValue,
    ) -> JsResult<JsValue>;
}
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, getter = "loadImg")]
    pub fn get_load_img(this: &Window) -> Option<Function>;

    #[wasm_bindgen(method, setter = "loadImg")]
    pub fn set_load_img(this: &Window, value: &Function);
}

pub fn window() -> Window {
    js_sys::global().unchecked_into::<Window>()
}

//TODO: Create own message implementation.
pub fn message(msg: &str) -> JsResult<JsValue> {
    window().message(msg)
}

pub fn is_pl() -> JsResult<bool> {
    window().__is_pl()
}

#[derive(Serialize)]
pub struct AskAlertData<'a> {
    #[serde(rename = "q")]
    question: &'a str,
    #[serde(rename = "clb", with = "serde_wasm_bindgen::preserve")]
    /// Callback executed when yes is clicked
    callback: Function,
    #[serde(rename = "type")]
    ask_type: &'a str,
}

impl<'a> AskAlertData<'a> {
    pub fn new(question: &'a str, callback: Function) -> Self {
        Self {
            question,
            callback,
            ask_type: "yesno4",
        }
    }
}

#[cfg(feature = "ni")]
pub fn ask_alert(data: AskAlertData) -> DefaultResult {
    let data: Object = serde_wasm_bindgen::to_value(&data)
        .map_err(common::map_err!(from))?
        .unchecked_into();
    window().ask_alert(&data)
}

#[cfg(not(feature = "ni"))]
pub fn ask_alert(data: AskAlertData) -> DefaultResult {
    let callbacks = js_sys::Array::of1(&data.callback);

    window().m_alert(data.question, 2, &callbacks, &JsValue::NULL)
}

#[cfg(feature = "ni")]
pub fn get_engine() -> Engine {
    window().engine().unwrap_js()
}

#[cfg(not(feature = "ni"))]
#[inline]
pub fn get_engine() -> Engine {
    window().unchecked_into()
}

#[cfg(feature = "ni")]
pub fn get_game_api() -> api::GameApi {
    window().game_api().unwrap_js()
}
