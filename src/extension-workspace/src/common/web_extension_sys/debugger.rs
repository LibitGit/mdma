use js_sys::Object;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub type Debugger;

    #[wasm_bindgen(method, catch)]
    pub async fn attach(
        this: &Debugger,
        debuggee: &JsValue,
        required_version: &str,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch)]
    pub async fn detach(this: &Debugger, debuggee: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, js_name = "sendCommand")]
    pub async fn send_command(
        this: &Debugger,
        target: &JsValue,
        method: &str,
        command_params: Option<&Object>,
    ) -> Result<JsValue, JsValue>;

}
