use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type StepsToSend;
}

#[cfg(feature = "ni")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, getter = "steps")]
    pub(crate) fn get_steps(this: &StepsToSend) -> Option<js_sys::Array>;

    #[wasm_bindgen(method, setter = "steps")]
    pub(crate) fn set_steps(this: &StepsToSend, val: &JsValue);
}

#[cfg(not(feature = "ni"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, getter = "ml")]
    pub(crate) fn get_steps(this: &StepsToSend) -> Option<js_sys::Array>;

    #[wasm_bindgen(method, setter = "ml")]
    pub(crate) fn set_steps(this: &StepsToSend, val: &JsValue);
}
