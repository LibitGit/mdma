use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type SkillsManager;
}

#[cfg(not(feature = "ni"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, getter = "skills")]
    pub fn get_skills(this: &SkillsManager) -> js_sys::Array;
}
