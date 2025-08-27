use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

use super::types::EquipmentSlot;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type HeroEquipment;
}

#[cfg(feature = "ni")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, js_name = "getFreeSlots")]
    pub fn get_free_slots(this: &HeroEquipment) -> u8;

    #[wasm_bindgen(catch, method, js_name = "getActiveBag")]
    fn __get_active_bag(this: &HeroEquipment) -> JsResult<u8>;

    #[wasm_bindgen(catch, method, js_name = "showBag")]
    pub fn show_bag(this: &HeroEquipment, bag: u8) -> JsResult<JsValue>;

    #[wasm_bindgen(method, getter = "newInventoryItems")]
    pub fn get_new_inventory_items(this: &HeroEquipment) -> Option<Function>;

    #[wasm_bindgen(method, setter = "newInventoryItems")]
    pub fn set_new_inventory_items(this: &HeroEquipment, value: &Function);
}

#[cfg(not(feature = "ni"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, getter = "freeSlots")]
    pub fn get_free_slots(this: &HeroEquipment) -> u8;

    #[wasm_bindgen(method, getter = "bag")]
    fn __internal_get_active_bag(this: &HeroEquipment) -> JsValue;

    #[wasm_bindgen(method, setter = "bag")]
    fn __internal_set_active_bag(this: &HeroEquipment, bag: u8);

    #[wasm_bindgen(method, getter = "newItem")]
    fn __get_new_inventory_items(this: &super::Engine) -> Option<Function>;

    #[wasm_bindgen(method, setter = "newItem")]
    fn __set_new_inventory_items(this: &super::Engine, value: &Function);
}

#[cfg(not(feature = "ni"))]
impl HeroEquipment {
    pub fn __get_active_bag(&self) -> JsResult<u8> {
        Ok(self.__internal_get_active_bag().unchecked_into_f64() as u8)
    }

    // TODO: Implement this.
    #[inline]
    pub fn show_bag(&self, bag: u8) -> JsResult<JsValue> {
        let bag = bag.wrapping_sub(20);
        
        self.__internal_set_active_bag(bag);
        crate::utils::document()
            .get_element_by_id("hlbag")
            .ok_or_else(|| common::err_code!())?
            .dyn_into::<web_sys::HtmlDivElement>()
            .map_err(|_| common::err_code!())?
            .style()
            .set_property("left", &format!("{}px", 25 + bag as i32 * 33))
            .map_err(common::map_err!())?;
        crate::utils::document()
            .get_element_by_id("bag")
            .ok_or_else(|| common::err_code!())?
            .dyn_into::<web_sys::HtmlDivElement>()
            .map_err(|_| common::err_code!())?
            .style()
            .set_property("top", &format!("{}px", -198 * bag as i32))
            .map_err(common::map_err!())?;

        Ok(JsValue::UNDEFINED)
    }

    // pub fn get_new_inventory_items(&self) -> Option<Function> {
    //     super::get_engine().__get_new_inventory_items()
    // }

    // pub fn set_new_inventory_items(&self, value: &Function) {
    //     super::get_engine().__set_new_inventory_items(value)
    // }
}
impl HeroEquipment {
    pub fn get_active_bag(&self) -> JsResult<EquipmentSlot> {
        self.__get_active_bag().map(Into::into)
    }
}
