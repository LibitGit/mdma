use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type ChatController;

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type ChatInputWrapper;

    #[wasm_bindgen(method, js_name = "getChatInputWrapper")]
    pub fn get_chat_input_wrapper(this: &ChatController) -> Option<ChatInputWrapper>;

    #[wasm_bindgen(catch, method, js_name = "setPrivateMessageProcedure")]
    pub fn set_private_message_procedure(this: &ChatInputWrapper, nick: &str) -> JsResult<JsValue>;
}
