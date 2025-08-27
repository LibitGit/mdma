use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type LockManager;

    #[wasm_bindgen(method, getter = "list")]
    pub fn lock_list(this: &LockManager) -> JsValue;

    #[wasm_bindgen(catch, method, js_name = "add")]
    pub fn add_lock(this: &LockManager, lock_name: &str) -> JsResult<()>;

    #[wasm_bindgen(catch, method, js_name = "remove")]
    pub fn remove_lock(this: &LockManager, lock_name: &str) -> JsResult<()>;
}
