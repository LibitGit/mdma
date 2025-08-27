use common::map_err;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{globals::OtherId, utils::JsResult};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type IFrameWindowManager;

    #[wasm_bindgen(catch, method, js_name = "newPlayerProfile")]
    fn __new_player_profile(
        this: &IFrameWindowManager,
        options: &js_sys::Object,
    ) -> JsResult<JsValue>;
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfileOptions {
    account_id: u32,
    character_id: OtherId,
}

impl PlayerProfileOptions {
    pub fn new(account_id: u32, character_id: OtherId) -> Self {
        Self {
            account_id,
            character_id,
        }
    }
}

impl IFrameWindowManager {
    pub fn new_player_profile(&self, options: &PlayerProfileOptions) -> JsResult<JsValue> {
        let options = serde_wasm_bindgen::to_value(options).map_err(map_err!())?;

        self.__new_player_profile(options.unchecked_ref())
    }
}
