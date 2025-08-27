use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type ItemsMarkManager;

    #[wasm_bindgen(method, getter = "newItem")]
    pub(crate) fn get_new_item(this: &ItemsMarkManager) -> Option<Function>;

    #[wasm_bindgen(method, setter = "newItem")]
    pub(crate) fn set_new_item(this: &ItemsMarkManager, value: &Function) -> Option<Function>;
}
