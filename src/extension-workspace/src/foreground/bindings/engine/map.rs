use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

use super::types::MapMode;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type Map;

    #[wasm_bindgen(method, getter = "offset")]
    pub(crate) fn get_offset(this: &Map) -> Option<Box<[f64]>>;

    #[wasm_bindgen(method, getter = "groundItems")]
    pub(crate) fn ground_items(this: &Map) -> Option<GroundItems>;

    #[cfg(feature = "ni")]
    #[wasm_bindgen(method, getter = "d")]
    pub(crate) fn map_data(this: &Map) -> Option<MapData>;
}

#[cfg(not(feature = "ni"))]
impl Map {
    #[inline]
    pub(crate) fn map_data(&self) -> Option<MapData> {
        Some(self.clone().unchecked_into())
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type MapData;

    #[wasm_bindgen(method, getter = "pvp")]
    fn __get_pvp(this: &MapData) -> Option<u8>;
}

impl MapData {
    pub(crate) fn get_pvp(&self) -> Option<MapMode> {
        self.__get_pvp().and_then(|pvp| pvp.try_into().ok())
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type GroundItems;
    #[wasm_bindgen(method, getter = "changeOverlays")]
    pub(crate) fn get_change_overlays(this: &GroundItems) -> Option<Function>;

    #[wasm_bindgen(method, setter, js_name = "changeOverlays")]
    pub(crate) fn set_change_overlays(this: &GroundItems, value: &Function);

    #[wasm_bindgen(catch, method, js_name = "changeOverlays")]
    pub(crate) fn change_overlays(
        this: &GroundItems,
        image_url: &JsValue,
        bg_pos_y: &JsValue,
    ) -> JsResult<JsValue>;

    #[wasm_bindgen(method, getter = "changeFrames")]
    pub(crate) fn get_change_frames(this: &GroundItems) -> Option<Function>;

    #[wasm_bindgen(method, setter, js_name = "changeFrames")]
    pub(crate) fn set_change_frames(this: &GroundItems, value: &Function);

    #[wasm_bindgen(catch, method, js_name = "changeFrames")]
    pub(crate) fn change_frames(
        this: &GroundItems,
        image_url: &JsValue,
        image_offset: &JsValue,
    ) -> JsResult<JsValue>;
}
