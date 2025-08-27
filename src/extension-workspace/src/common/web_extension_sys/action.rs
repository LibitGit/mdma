use js_sys::{Object, Promise};
use serde::Serialize;
use serde_with::{serde_as, skip_serializing_none};
use wasm_bindgen::prelude::*;

use super::{
    EventTarget,
    tabs::{TabId, WindowId},
};

#[skip_serializing_none]
#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PopupDetails<'a> {
    #[serde(rename = "popup")]
    pub relative_path: &'a str,
    pub tab_id: Option<TabId>,
}

impl<'a> PopupDetails<'a> {
    pub fn new(relative_path: &'a str, tab_id: Option<TabId>) -> Self {
        Self {
            relative_path,
            tab_id,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenPopupOptions {
    pub window_id: Option<WindowId>,
}

#[wasm_bindgen]
extern "C" {
    // https://developer.chrome.com/docs/extensions/reference/action/
    pub type Action;

    // https://developer.chrome.com/docs/extensions/reference/action/#event-onClicked
    #[wasm_bindgen(method, getter, js_name = onClicked)]
    pub fn on_clicked(this: &Action) -> EventTarget;

    //#[wasm_bindgen(method, js_name = "getPopup")]
    //pub fn get_popup(this: &Action, details: &Object) -> Promise;

    #[wasm_bindgen(method, js_name = "setPopup")]
    pub fn set_popup(this: &Action, details: &Object) -> Promise;

    #[wasm_bindgen(catch, method, js_name = "openPopup")]
    pub async fn open_popup(this: &Action) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, method, js_name = "openPopup")]
    pub async fn open_popup_with_options(
        this: &Action,
        options: &Object,
    ) -> Result<JsValue, JsValue>;
}
