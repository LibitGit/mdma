use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type Battle;

    #[wasm_bindgen(method, getter = "show")]
    pub fn show(this: &Battle) -> Option<bool>;

    #[wasm_bindgen(method, getter = "endBattleForMe")]
    pub fn end_battle_for_me(this: &Battle) -> Option<bool>;
}
