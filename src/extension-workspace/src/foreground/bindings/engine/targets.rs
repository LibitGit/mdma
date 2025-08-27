use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type Targets;

    #[wasm_bindgen(catch, method, js_name = "addArrow")]
    pub(crate) fn add_arrow(
        this: &Targets,
        id_arrow: bool,
        name: &str,
        obj_parent: &JsValue,
        type_parent: &str,
        type_arrow: &str,
    ) -> JsResult<()>;

    #[wasm_bindgen(catch, method, js_name = "deleteArrow")]
    pub(crate) fn delete_arrow(this: &Targets, id: &str) -> JsResult<()>;
}
