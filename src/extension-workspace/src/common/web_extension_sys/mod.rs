use action::Action;
use cookies::Cookies;
use debugger::Debugger;
use js_sys::Function;
use runtime::Runtime;
use scripting::Scripting;
use serde::Serialize;
use storage::Storage;
use tabs::Tabs;
use wasm_bindgen::prelude::*;
use windows::Windows;
use identity::Identity;

pub mod action;
pub mod cookies;
pub mod debugger;
pub mod runtime;
pub mod scripting;
pub mod storage;
pub mod tabs;
pub mod windows;
pub mod identity;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Debuggee {
    tab_id: i32,
}

impl Debuggee {
    pub fn new(tab_id: i32) -> Self {
        Self { tab_id }
    }
}

#[derive(Serialize)]
pub enum KeyEventType {
    KeyDown,
}

#[derive(Serialize)]
pub struct InputEvent<'a> {
    type_: KeyEventType,
    key: Option<&'a str>,
}

impl<'a> InputEvent<'a> {
    pub fn new(type_: KeyEventType, key: Option<&'a str>) -> Self {
        Self { type_, key }
    }
}

#[cfg(feature = "no_window")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = ::web_sys::WorkerGlobalScope, extends = ::web_sys::EventTarget, extends = ::js_sys::Object, js_name = ServiceWorkerGlobalScope, typescript_type = "ServiceWorkerGlobalScope")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `ServiceWorkerGlobalScope` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerGlobalScope)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `ServiceWorkerGlobalScope`*"]
    pub type ServiceWorkerGlobalScope;

    #[wasm_bindgen(method, getter)]
    pub fn chrome(this: &ServiceWorkerGlobalScope) -> Browser;

    #[wasm_bindgen(method, getter)]
    pub fn browser(this: &ServiceWorkerGlobalScope) -> Browser;
}

#[cfg(feature = "no_window")]
pub fn browser() -> Browser {
    if cfg!(feature = "firefox") {
        js_sys::global()
            .unchecked_into::<ServiceWorkerGlobalScope>()
            .browser()
    } else {
        js_sys::global()
            .unchecked_into::<ServiceWorkerGlobalScope>()
            .chrome()
    }
}

#[cfg(not(feature = "no_window"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = ::web_sys::EventTarget, extends = ::js_sys::Object, js_name = Window, typescript_type = "Window")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `Window` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Window)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Window`*"]
    pub type Window;

    #[wasm_bindgen(method, getter)]
    pub fn chrome(this: &Window) -> Browser;

    #[wasm_bindgen(method, getter)]
    pub fn browser(this: &Window) -> Browser;
}

#[cfg(not(feature = "no_window"))]
pub fn browser() -> Browser {
    if cfg!(feature = "firefox") {
        js_sys::global().unchecked_into::<Window>().browser()
    } else {
        js_sys::global().unchecked_into::<Window>().chrome()
    }
}

#[wasm_bindgen]
extern "C" {
    pub type Browser;

    #[wasm_bindgen(method, getter)]
    pub fn action(this: &Browser) -> Action;

    #[wasm_bindgen(method, getter)]
    pub fn cookies(this: &Browser) -> Cookies;

    #[wasm_bindgen(method, getter)]
    pub fn debugger(this: &Browser) -> Debugger;

    #[wasm_bindgen(method, getter)]
    pub fn runtime(this: &Browser) -> Runtime;

    #[wasm_bindgen(method, getter)]
    pub fn scripting(this: &Browser) -> Scripting;

    #[wasm_bindgen(method, getter)]
    pub fn storage(this: &Browser) -> Storage;

    #[wasm_bindgen(method, getter)]
    pub fn tabs(this: &Browser) -> Tabs;

    #[wasm_bindgen(method, getter)]
    pub fn windows(this: &Browser) -> Windows;

    #[wasm_bindgen(method, getter)]
    pub fn identity(this: &Browser) -> Identity;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::web_sys::EventTarget)]
    pub type EventTarget;

    #[wasm_bindgen(method, js_name = addListener)]
    pub fn add_listener(this: &EventTarget, listener: &Function);

    #[wasm_bindgen(method, js_name = removeListener)]
    pub fn remove_listener(this: &EventTarget, listener: &Function);

    #[wasm_bindgen(method, js_name = hasListener)]
    pub fn has_listener(this: &EventTarget, listener: &Function) -> bool;
}
