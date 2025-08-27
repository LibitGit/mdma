use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type Renderer;

    #[wasm_bindgen(method, getter = "add")]
    pub(crate) fn get_add(this: &Renderer) -> Function;

    #[wasm_bindgen(method, setter = "add")]
    pub(crate) fn set_add(this: &Renderer, val: &Function);

    #[wasm_bindgen(method, js_name = "add")]
    pub(crate) fn add_1(this: &Renderer, drawable_obj: &JsValue);

    #[wasm_bindgen(method, js_name = "getHighestOrderWithoutSort")]
    pub(crate) fn get_highest_order_without_sort(this: &Renderer) -> f64;
}
