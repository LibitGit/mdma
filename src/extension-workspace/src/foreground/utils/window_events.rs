use common::err_code;
use dominator::events::KeyDown;
use wasm_bindgen::JsCast;
use web_sys::{Element, Node};

use crate::{make_keyboard_event, make_mouse_event, s, static_event_impl};

use super::{JsResult, polish_to_ascii};

make_keyboard_event!(KeyPress);
static_event_impl!(KeyPress => "keypress");

impl KeyPress {
    #[inline]
    pub(crate) fn stop_immediate_propagation(&self) {
        self.event.stop_immediate_propagation();
    }
}

make_mouse_event!(MouseOver => web_sys::MouseEvent);
static_event_impl!(MouseOver => "mouseover");

pub(crate) fn verify_keyboard_event(event: &web_sys::KeyboardEvent, key: &str) -> JsResult<bool> {
    //let value = event
    //    .key()
    //    .chars()
    //    .map(polish_to_ascii)
    //    .collect::<String>()
    //    .to_ascii_uppercase();
    if event.key().to_lowercase() != key.to_lowercase() {
        return Ok(false);
    }

    if event.repeat() {
        return Ok(false);
    }

    let target = event.target().ok_or_else(|| err_code!())?;
    let target_node: &Node = target.dyn_ref().ok_or_else(|| err_code!())?;
    let target_elem: &Element = target.dyn_ref().ok_or_else(|| err_code!())?;

    if target_node.node_type() != 1 {
        return Ok(false);
    }
    if !target_node.has_child_nodes()
        && target_node.node_name() == s!("DIV")
        && target_elem.class_list().length() == 0
    {
        return Ok(false);
    }

    let target_tag_name = target_elem.tag_name().to_uppercase();

    if [s!("INPUT"), s!("TEXTAREA"), s!("SELECT")]
        .iter()
        .any(|&s| target_tag_name.contains(s))
    {
        return Ok(false);
    }

    Ok(true)
}

pub trait Hotkey {
    fn value(&self) -> &str;
    fn alt_key(&self) -> bool;
    fn ctrl_key(&self) -> bool;
    fn shift_key(&self) -> bool;
}

// The hotkey has to be an uppercase ascii char.
pub(crate) fn validate_keydown_event<B: Hotkey>(event: &KeyDown, hotkey: &B) -> JsResult<bool> {
    if event.repeat() {
        return Ok(false);
    }

    let value = event
        .key()
        .chars()
        .map(polish_to_ascii)
        .collect::<String>()
        .to_ascii_uppercase();
    if value.as_str() != hotkey.value() {
        return Ok(false);
    }
    if hotkey.shift_key() != event.shift_key()
        || hotkey.alt_key() != event.alt_key()
        || hotkey.ctrl_key() != event.ctrl_key()
    {
        return Ok(false);
    }

    let target_node: Node = event.dyn_target().ok_or_else(|| err_code!())?;

    if target_node.node_type() != 1 {
        return Ok(false);
    }

    let target_elem: Element = event.dyn_target().ok_or_else(|| err_code!())?;

    if !target_node.has_child_nodes()
        && target_node.node_name() == s!("DIV")
        && target_elem.class_list().length() == 0
    {
        return Ok(false);
    }

    let target_tag_name = target_elem.tag_name().to_uppercase();

    if [s!("INPUT"), s!("TEXTAREA"), s!("SELECT")]
        .iter()
        .any(|&s| target_tag_name.contains(s))
    {
        return Ok(false);
    }

    Ok(true)
}

//pub(crate) fn verify_dominator_event(event: &KeyDown, key: &str) -> JsResult<bool> {
//    if event.key().to_lowercase() != key.to_lowercase() {
//        return Ok(false);
//    }
//
//    let target_node: Node = event.dyn_target().ok_or_else(|| err_code!())?;
//
//    if target_node.node_type() != 1 {
//        return Ok(false);
//    }
//
//    let target_elem: Element = event.dyn_target().ok_or_else(|| err_code!())?;
//
//    if !target_node.has_child_nodes()
//        && target_node.node_name() == s!("DIV")
//        && target_elem.class_list().length() == 0
//    {
//        return Ok(false);
//    }
//
//    let target_tag_name = target_elem.tag_name().to_uppercase();
//
//    if [s!("INPUT"), s!("TEXTAREA"), s!("SELECT")]
//        .iter()
//        .any(|&s| target_tag_name.contains(s))
//    {
//        return Ok(false);
//    }
//
//    Ok(true)
//}
