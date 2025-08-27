use js_sys::Object;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

//#[wasm_bindgen]
//extern "C" {
//    #[wasm_bindgen(extends = ::js_sys::Object)]
//    #[derive(Clone, Debug)]
//    pub type Cookie;
//
//    #[wasm_bindgen(method, getter)]
//    pub fn value(this: &Cookie) -> String;
//}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieDetails {
    pub name: String,
    pub url: String,
}

#[wasm_bindgen]
extern "C" {
    pub type Cookies;

    //TODO: It returns Promise<Cookie|undefined>
    #[wasm_bindgen(method)]
    pub async fn get(this: &Cookies, details: &Object) -> JsValue;

}
