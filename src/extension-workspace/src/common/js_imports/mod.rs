use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, variadic, js_name = log)]
    pub fn js_log(items: Box<[JsValue]>);
    #[wasm_bindgen(js_namespace = console, variadic, js_name = debug)]
    pub fn js_debug(items: Box<[JsValue]>);
    #[wasm_bindgen(js_namespace = console, js_name = debug)]
    pub fn js_debug_2(log1: &str, log2: &JsValue);
    #[wasm_bindgen(js_namespace = console, variadic, js_name = error)]
    pub fn js_error(items: Box<[JsValue]>);
}

#[cfg(debug_assertions)]
#[wasm_bindgen(inline_js = "
    export function breakpoint() {
        debugger;
    }
")]
extern "C" {
    pub fn breakpoint();
}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn breakpoint() {}
