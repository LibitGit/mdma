pub mod port;

use js_sys::{Array, Object, Promise};
use port::Port;
use wasm_bindgen::prelude::*;

use super::EventTarget;

//#[derive(Serialize)]
//#[serde(rename_all = "camelCase")]
//pub struct MessageOptions {
//    pub include_tls_channel_id: bool,
//}

#[wasm_bindgen]
extern "C" {
    pub type Runtime;

    #[wasm_bindgen(method, getter)]
    pub fn id(this: &Runtime) -> JsValue;

    #[wasm_bindgen(method, js_name = "connect")]
    pub fn connect(this: &Runtime, extension_id: &str, connect_info: &Object) -> Port;

    #[wasm_bindgen(method, js_name = "getURL")]
    pub fn get_url(this: &Runtime, path: &str) -> String;

    #[wasm_bindgen(method, getter, js_name = onConnect)]
    pub fn on_connect(this: &Runtime) -> EventTarget;

    #[wasm_bindgen(method, getter, js_name = onConnectExternal)]
    pub fn on_connect_external(this: &Runtime) -> EventTarget;

    #[wasm_bindgen(method, catch, js_name = "sendMessage")]
    pub async fn send_message(this: &Runtime, message: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, js_name = sendMessage)]
    pub fn send_message_with_id(this: &Runtime, extension_id: &str, message: &JsValue) -> Promise;

    #[wasm_bindgen(method, getter = "lastError")]
    pub fn last_error(this: &Runtime) -> Option<js_sys::Error>;

    #[wasm_bindgen(method, getter = "getContexts")]
    pub async fn get_contexts(this: &Runtime, filter: &Object) -> Array;
}
