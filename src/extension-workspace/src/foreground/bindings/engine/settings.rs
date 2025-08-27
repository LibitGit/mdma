use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::{globals::hero_settings::HeroSettings, utils::JsResult};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type Settings;

    #[wasm_bindgen(method, getter = "toggleserveroption")]
    pub(crate) fn get_toggle_server_option(this: &Settings) -> Option<Function>;

    #[wasm_bindgen(method, setter, js_name = toggleserveroption)]
    pub(crate) fn set_toggle_server_option(this: &Settings, value: &Function);

    #[wasm_bindgen(catch, method, js_name = toggleserveroption)]
    pub(crate) fn toggle_server_option(this: &Settings, option: &JsValue) -> JsResult<JsValue>;
}

impl Settings {
    pub(crate) async fn init_server_option_config() {
        HeroSettings::init_server_option_config().await
    }

    //pub(crate) fn observe_toggle_server_option(&self) -> JsResult<()> {
    //    let original_toggle_server_option = self.get_toggle_server_option().ok_or_else(|| {
    //        common::debug_log!("ERROR IN HERE");
    //        err_code!()
    //    })?;
    //    let new_toggle_server_option = closure!(
    //        { let settings = self.clone() },
    //        move |server_option: JsValue| -> DefaultResult {
    //            match CharacterSettingId::try_from(&server_option) {
    //                Ok(CharacterSettings::FRIEND_NOTIF_ID) | Ok(CharacterSettings::CLAN_NOTIF_ID) => message(intern(s!("[MDMA::RS] Ta opcja jest potrzebna do poprawnego działania zestawu!"))),
    //                _ => original_toggle_server_option.call1(&settings, &server_option),
    //            }
    //        },
    //    );
    //
    //    self.set_toggle_server_option(&new_toggle_server_option);
    //
    //    Ok(())
    //}
}
