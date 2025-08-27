use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type DisableItemsManager;

    #[wasm_bindgen(catch, method, js_name = "startSpecificItemKindDisable")]
    pub(crate) fn start_specific_item_kind_disable(
        this: &DisableItemsManager,
        item_kind: &str,
    ) -> JsResult<JsValue>;

    #[wasm_bindgen(catch, method, js_name = "endSpecificItemKindDisable")]
    pub(crate) fn end_specific_item_kind_disable(
        this: &DisableItemsManager,
        item_kind: &str,
    ) -> JsResult<JsValue>;

    #[wasm_bindgen(method, getter = "addDisableIcon")]
    pub(crate) fn get_add_disable_icon(this: &DisableItemsManager) -> Option<Function>;

    #[wasm_bindgen(method, setter = "addDisableIcon")]
    pub(crate) fn set_add_disable_icon(
        this: &DisableItemsManager,
        value: &Function,
    ) -> Option<Function>;

    #[wasm_bindgen(method, getter = "removeDisableIcon")]
    pub(crate) fn get_remove_disable_icon(this: &DisableItemsManager) -> Option<Function>;

    #[wasm_bindgen(method, setter = "removeDisableIcon")]
    pub(crate) fn set_remove_disable_icon(
        this: &DisableItemsManager,
        value: &Function,
    ) -> Option<Function>;
}
