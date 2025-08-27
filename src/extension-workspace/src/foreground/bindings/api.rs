use common::{closure, err_code, map_err};
use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

use super::get_engine;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The game's `API` object."]
    pub type GameApi;

    #[wasm_bindgen(catch, method, js_name = "addCallbackToEvent")]
    pub fn add_callback_to_event(
        this: &GameApi,
        event_name: &JsValue,
        value: &Function,
    ) -> JsResult<JsValue>;

    #[wasm_bindgen(catch, method, js_name = "removeCallbackFromEvent")]
    pub fn remove_callback_from_event(
        this: &GameApi,
        event_name: &JsValue,
        value: &Function,
    ) -> JsResult<JsValue>;

    #[wasm_bindgen(catch, method, js_name = "callEvent")]
    pub fn call_event(
        this: &GameApi,
        event_name: &str,
        event_data: &JsValue,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, getter = "callEvent")]
    pub fn get_call_event(this: &GameApi) -> Option<Function>;

    #[wasm_bindgen(method, setter = "callEvent")]
    pub fn set_call_event(this: &GameApi, value: &Function);

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type ApiData;

    #[wasm_bindgen(method, getter = "AFTER_INTERFACE_START")]
    pub(crate) fn after_interface_start(this: &ApiData) -> JsValue;

    #[wasm_bindgen(method, getter = "CALL_DRAW_ADD_TO_RENDERER")]
    pub(crate) fn call_draw_add_to_renderer(this: &ApiData) -> JsValue;

    #[wasm_bindgen(method, getter = "UPDATE_OTHER")]
    pub(crate) fn update_other(this: &ApiData) -> JsValue;

}

impl GameApi {
    pub(crate) fn observe_call_event(&self) -> JsResult<()> {
        let api_data = get_engine().api_data().ok_or_else(|| err_code!())?;

        // TODO: Some way to await this?
        // let after_interface_start_event = api_data.after_interface_start();
        // let callback = Rc::new(RefCell::new(None));
        // let after_interface_start = closure!(@once
        //     { let callback = callback.clone() },
        //     move || {
        //         if let Some(_callback) = callback.take() {
        //             emitter::Emitter::get().emit(&emitter::EmitterEvent::AfterInterfaceStart, &mut JsValue::undefined());
        //         }
        //     },
        // );
        // self.add_callback_to_event(&after_interface_start_event, &after_interface_start)
        //     .map_err(map_err!())?;
        // *callback.borrow_mut() = Some(after_interface_start);

        let renderer = get_engine().renderer().ok_or_else(|| err_code!())?;
        let after_call_draw_add_to_renderer = closure!(move || -> JsResult<()> {
            crate::color_mark::ColorMark::on_call_draw_add_to_renderer(&renderer)
            // emitter::Emitter::get().emit(
            //     &emitter::EmitterEvent::CallDrawAddToRenderer,
            //     &mut JsValue::undefined(),
            // );
        });
        self.add_callback_to_event(
            &api_data.call_draw_add_to_renderer(),
            &after_call_draw_add_to_renderer,
        )
        .map_err(map_err!())?;

        Ok(())
    }
}
