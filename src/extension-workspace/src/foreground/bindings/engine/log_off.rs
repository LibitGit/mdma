use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type LogOff;
}

#[cfg(not(feature = "ni"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, getter = "start")]
    pub fn start(this: &LogOff) -> Option<i32>;
}