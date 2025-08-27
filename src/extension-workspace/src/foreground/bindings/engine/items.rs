use js_sys::{Function, Object};
use wasm_bindgen::prelude::*;

use crate::{bindings::engine::communication::Id, utils::JsResult};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type ItemsManager;

    #[wasm_bindgen(method, js_name = "getItemById")]
    pub(crate) fn get_item_by_id(this: &ItemsManager, id: Id) -> Option<Object>;

    #[wasm_bindgen(catch, method, js_name = "addCallback")]
    pub(crate) fn add_callback(
        this: &ItemsManager,
        loc: &str,
        name: &str,
        callback: &Function,
    ) -> JsResult<JsValue>;

    #[wasm_bindgen(catch, method, js_name = "removeCallback")]
    pub(crate) fn remove_callback(this: &ItemsManager, data: &Object) -> JsResult<JsValue>;

    #[wasm_bindgen(method, getter = "init")]
    pub(crate) fn get_init(this: &ItemsManager) -> Option<Function>;

    #[wasm_bindgen(method, setter = "init")]
    pub(crate) fn set_init(this: &ItemsManager, value: &Function);

    #[wasm_bindgen(catch, method, js_name = "init")]
    pub(crate) fn init(this: &ItemsManager) -> JsResult<JsValue>;

    #[wasm_bindgen(catch, method, js_name = "getPlaceholderItem")]
    pub(crate) fn get_placeholder_item(this: &ItemsManager) -> JsResult<JsValue>;

    #[wasm_bindgen(method, getter = "changePlaceHolderIconIntoNormalIcon")]
    pub(crate) fn get_update_placeholder(this: &ItemsManager) -> Option<Function>;

    #[wasm_bindgen(method, setter = "changePlaceHolderIconIntoNormalIcon")]
    pub(crate) fn set_update_placeholder(this: &ItemsManager, value: &Function);

    #[wasm_bindgen(catch, method, js_name = "changePlaceHolderIconIntoNormalIcon")]
    pub(crate) fn update_placeholder(this: &ItemsManager, item_id: &JsValue) -> JsResult<JsValue>;

    #[wasm_bindgen(catch, method, js_name = "getAllViewsByIdAndViewName")]
    pub(crate) fn get_all_views_by_id_and_view_name(
        this: &ItemsManager,
        item_id: Id,
        view_name: &str,
    ) -> JsResult<js_sys::Array>;
}

impl ItemsManager {
    //pub(crate) fn observe_init_items(&self, emitter: globals::Emitter) {
    //    let original_init_items = self.get_init().unwrap_js();
    //    let new_init_items = closure!(
    //        { let items_manager = self.clone() },
    //        move || -> DefaultResult {
    //            emitter.emit(&globals::EmitterEvent::InitItems, &mut JsValue::undefined());
    //            items_manager.observe_update_placeholder(emitter.clone());
    //            debug_log!("AFTER OBSERVE UPDATE PLACEHOLDER");
    //            items_manager.set_init(&original_init_items);
    //            items_manager.init()
    //        },
    //    );
    //
    //    self.set_init(&new_init_items);
    //}

    #[cfg(feature = "ni")]
    pub(crate) fn observe_update_placeholder(&self) -> JsResult<()> {
        // use crate::globals::emitter::{Emitter, EmitterEvent};
        // use common::{closure, err_code};

        // let original_update_placeholder =
        //     self.get_update_placeholder().ok_or_else(|| err_code!())?;
        // let new_update_placeholder = closure!(
        //     { let items_manager = self.clone() },
        //     // This item_id is a string, not a number!
        //     move |item_id: JsValue| -> JsResult<JsValue> {
        //         let res = original_update_placeholder.call1(&items_manager, &item_id);
        //         let mut item_id = item_id;
        //         Emitter::get().emit(&EmitterEvent::AfterUpdateItemPlaceholder, &mut item_id);
        //         res
        //     },
        // );

        // self.set_update_placeholder(&new_update_placeholder);

        Ok(())
    }
}
