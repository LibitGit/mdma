use common::{map_err, messaging::prelude::*};
use wasm_bindgen::prelude::*;

use crate::dispatcher::Dispatcher;

#[wasm_bindgen(js_name = "handleMessage")]
pub async fn handle_message(message: JsValue) -> Result<(), JsValue> {
    let message: Message = serde_wasm_bindgen::from_value(message).map_err(map_err!())?;

    Dispatcher::dispatch(message).await.map_err(map_err!(from))
}