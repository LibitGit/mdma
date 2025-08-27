use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type WhoIsHere;

    #[wasm_bindgen(method, getter = "managePanelVisible")]
    pub(crate) fn get_manage_panel_visible(this: &WhoIsHere) -> Option<Function>;

    #[wasm_bindgen(method, setter = "managePanelVisible")]
    pub(crate) fn set_manage_panel_visible(this: &WhoIsHere, val: &Function);

    #[wasm_bindgen(catch, method, js_name = "isShow")]
    pub(crate) fn is_show(this: &WhoIsHere) -> JsResult<bool>;

    #[wasm_bindgen(catch, method, js_name = "closePanel")]
    pub(crate) fn close_panel(this: &WhoIsHere) -> JsResult<()>;
}
