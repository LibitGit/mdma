use crate::web_extension_sys::{EventTarget, tabs::Tab};
use js_sys::Error;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, Clone)]
    pub type MessageSender;

    #[wasm_bindgen(method, getter)]
    pub fn tab(this: &MessageSender) -> Option<Tab>;

    #[wasm_bindgen(method, getter)]
    pub fn id(this: &MessageSender) -> Option<String>;

    #[wasm_bindgen(method, getter)]
    pub fn url(this: &MessageSender) -> Option<String>;

    #[wasm_bindgen(method, getter, js_name = tlsChannelId)]
    pub fn tls_channel_id(this: &MessageSender) -> Option<String>;
}

#[wasm_bindgen]
extern "C" {
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/runtime/Port)"]
    #[derive(Debug, Clone, PartialEq)]
    pub type Port;

    #[wasm_bindgen(method, getter)]
    pub fn name(this: &Port) -> String;

    #[wasm_bindgen(method, getter)]
    pub fn error(this: &Port) -> Option<Error>;

    #[wasm_bindgen(method)]
    pub fn disconnect(this: &Port);

    #[wasm_bindgen(method, getter, js_name = onDisconnect)]
    pub fn on_disconnect(this: &Port) -> EventTarget;

    #[wasm_bindgen(method, getter, js_name = onMessage)]
    pub fn on_message(this: &Port) -> EventTarget;

    #[wasm_bindgen(method, js_name = postMessage)]
    pub fn post_message(this: &Port, value: &JsValue);

    #[wasm_bindgen(method, getter)]
    pub fn sender(this: &Port) -> Option<MessageSender>;
}
