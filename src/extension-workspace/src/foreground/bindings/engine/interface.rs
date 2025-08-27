use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type Interface;

    #[wasm_bindgen(catch, method, js_name = "clickWhoIsHere")]
    pub(crate) fn click_who_is_here(this: &Interface) -> JsResult<JsValue>;

    #[wasm_bindgen(method, setter = "clickWhoIsHere")]
    pub(crate) fn set_click_who_is_here(this: &Interface, value: &Function);

    #[wasm_bindgen(method, getter = "clickWhoIsHere")]
    pub(crate) fn get_click_who_is_here(this: &Interface) -> Option<Function>;

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type JQueryObject;

    #[wasm_bindgen(method)]
    pub(crate) fn get(this: &JQueryObject, index: i32) -> JsValue;

    #[wasm_bindgen(catch, method, js_name = "get$GAME_CANVAS")]
    fn __get_game_canvas(this: &Interface) -> JsResult<JQueryObject>;

    #[wasm_bindgen(catch, method, js_name = "initBanners")]
    pub(crate) fn init_banners(this: &Interface) -> JsResult<JsValue>;

    #[wasm_bindgen(method, setter = "initBanners")]
    pub(crate) fn set_init_banners(this: &Interface, value: &Function);

    #[wasm_bindgen(method, getter = "initBanners")]
    pub(crate) fn get_init_banners(this: &Interface) -> Option<Function>;
}
