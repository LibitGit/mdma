use wasm_bindgen::prelude::*;

#[cfg(not(feature = "ni"))]
const DEFAULT_DIFF: f64 = 200.0;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type IdleJSON;

    #[cfg(feature = "ni")]
    #[wasm_bindgen(method, js_name = "setDiff")]
    pub fn set_diff(this: &IdleJSON, value: f64);

    #[cfg(feature = "ni")]
    #[wasm_bindgen(method, js_name = "setDefaultDiff")]
    pub fn set_default_diff(this: &IdleJSON);
}
#[cfg(not(feature = "ni"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, setter = "limit")]
    pub fn set_limit(this: &IdleJSON, value: f64);
}

#[cfg(not(feature = "ni"))]
impl IdleJSON {
    pub fn set_diff(&self, value: f64) {
        self.set_limit(value)
    }

    pub fn set_default_diff(&self) {
        self.set_limit(DEFAULT_DIFF)
    }
}
