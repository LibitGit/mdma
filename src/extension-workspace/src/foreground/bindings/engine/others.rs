use js_sys::Function;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use crate::{globals::OtherId, utils::JsResult};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type OthersManager;

    #[wasm_bindgen(method, js_name = "getById")]
    pub fn get_by_id(this: &OthersManager, other_id: OtherId) -> Option<Other>;

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type OtherData;

    #[wasm_bindgen(method, getter = "id")]
    fn __id(this: &OtherData) -> JsValue;

    #[wasm_bindgen(method, getter = "nick")]
    pub fn nick(this: &OtherData) -> String;

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type Other;

    #[wasm_bindgen(catch, method, js_name = "showEq")]
    pub fn show_eq(this: &Other) -> JsResult<JsValue>;

    #[wasm_bindgen(method, getter = "d")]
    pub fn d(this: &Other) -> OtherData;

    #[wasm_bindgen(catch, method, js_name = "draw")]
    pub(crate) fn draw(this: &Other, ctx: &CanvasRenderingContext2d) -> JsResult<JsValue>;

    #[wasm_bindgen(method, setter = "draw")]
    pub(crate) fn set_draw(this: &Other, value: &Function);

    #[wasm_bindgen(method, getter = "draw")]
    pub(crate) fn get_draw(this: &Other) -> Option<Function>;

    #[wasm_bindgen(catch, method, js_name = "getOrder")]
    pub(crate) fn get_order(this: &Other) -> JsResult<JsValue>;

    #[wasm_bindgen(method, setter = "getOrder")]
    pub(crate) fn set_get_order(this: &Other, value: &Function);

    #[wasm_bindgen(method, getter = "getOrder")]
    pub(crate) fn get_get_order(this: &Other) -> Option<Function>;

    #[wasm_bindgen(method, getter = "rx")]
    pub(crate) fn rx(this: &Other) -> Option<f64>;

    #[wasm_bindgen(method, getter = "ry")]
    pub(crate) fn ry(this: &Other) -> Option<f64>;

    #[wasm_bindgen(method, getter = "fh")]
    pub(crate) fn fh(this: &Other) -> Option<f64>;

    #[wasm_bindgen(method, getter = "fw")]
    pub(crate) fn fw(this: &Other) -> Option<f64>;

    #[wasm_bindgen(catch, method, js_name = "update")]
    pub(crate) fn update(this: &Other) -> JsResult<JsValue>;

    #[wasm_bindgen(method, setter = "update")]
    pub(crate) fn set_update(this: &Other, value: &Function);

    #[wasm_bindgen(method, getter = "update")]
    pub(crate) fn get_update(this: &Other) -> Function;
}
