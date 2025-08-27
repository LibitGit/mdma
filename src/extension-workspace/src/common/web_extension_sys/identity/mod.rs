use js_sys::Object;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub type Identity;

    #[wasm_bindgen(method, catch, js_name = "launchWebAuthFlow")]
    pub async fn launch_web_auth_flow(this: &Identity, details: &Object) -> Result<JsValue, JsValue>;
}