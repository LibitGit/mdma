use std::cell::RefCell;
use std::ops::Not;
use std::thread::LocalKey;

use common::{debug_log, err_code, map_err};
use dominator::events::{MouseDown, MouseMove, MouseUp};
use dominator::traits::{AsStr, MultiStr};
use dominator::{DomBuilder, apply_methods, clone, html, with_node};
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVec, SignalVecExt};
use js_sys::Array;
use wasm_bindgen::intern;
use web_sys::{
    DomRect, Element, EventTarget, HtmlDivElement, HtmlElement, HtmlInputElement, Node, ShadowRoot,
};

use crate::interface::{dom_utils::ClassListVec, tips_parser::tip};
use crate::utils::{JsResult, UnwrapJsExt, window};
use crate::{class, console_error};

pub(crate) mod ui_components;
use ui_components::*;

pub(crate) mod decors;

pub(crate) mod prelude {
    pub(crate) use super::decors;
    pub(crate) use super::ui_components::*;
    //pub(crate) use super::*;
}

type ClassList<'a> = MutableVec<&'a str>;

use crate::globals::prelude::*;

thread_local! {
    pub(crate) static WINDOWS: RefCell<Vec<HtmlElement>> = const { RefCell::new(Vec::new()) };
    //TODO: Remove this bs and pass the AddonData of addon to change left and right mutables.
    pub(crate) static DRAGGING: RefCell<Option<(DomRect, &AddonWindowDetails)>> = const { RefCell::new(None) };
    static OFFSET: RefCell<(f64,f64)> = const { RefCell::new((0.0, 0.0)) };
}

const MDMA_ON_PEAK_CLASS: &str = "last-on-peak";
const GAME_ON_PEAK_CLASS: &str = "window-on-peak";

//trait InternalMdmaAddonWindow<T> {
//    //fn input_confirm_button(self, button: InputButton, input_elem: HtmlInputElement) -> Self;
//}
//
//impl<T: AsRef<Node>> InternalMdmaAddonWindow<T> for DomBuilder<T> {
//    //fn input_confirm_button(self, button: InputButton, input_elem: HtmlInputElement) -> Self {
//    //    self.child(button.render(input_elem))
//    //}
//}

pub(crate) trait MdmaAddonWindow<T> {
    fn add_window_toggle(self, window_type: WindowType, addon_key: AddonName) -> Self;

    fn dynamic_class_signal<B, C>(self, class_signal: C) -> Self
    where
        T: Clone + AsRef<HtmlElement> + 'static,
        B: MultiStr + 'static,
        C: Signal<Item = B> + 'static;

    fn disable_dragging(self) -> Self
    where
        T: AsRef<EventTarget>;

    fn stop_input_propagation(self) -> Self
    where
        T: AsRef<EventTarget> + AsRef<HtmlInputElement>;

    fn checkbox<B>(self, checkbox: Checkbox<B>) -> Self
    where
        B: Into<bool> + Not<Output = B> + Copy + PartialEq + 'static;

    fn input(self, input: Input) -> Self;

    fn class_list(self, class_list: &ClassList<'static>) -> Self
    where
        T: Into<HtmlElement> + Clone;

    fn build_draggable_window(self) -> Self
    where
        T: AsRef<HtmlElement> + AsRef<Element> + AsRef<EventTarget> + Clone + 'static;
}

#[derive(Clone, Default)]
pub(crate) struct ItemFrame {
    pub(crate) highlight_image_url: Mutable<Option<String>>,
    pub(crate) highlight_offset: Mutable<f64>,
    pub(crate) overlay_image_url: Mutable<Option<String>>,
}

impl ItemFrame {
    pub fn overlay_image_url_signal(&self) -> impl Signal<Item = Option<String>> + use<> {
        self.overlay_image_url.signal_ref(|image_url| {
            image_url
                .as_ref()
                .map(|image_url| format!(r#"url("{image_url}")"#))
        })
    }

    pub fn image_url_signal(&self) -> impl Signal<Item = Option<String>> + use<> {
        self.highlight_image_url.signal_ref(|image_url| {
            image_url
                .as_ref()
                .map(|image_url| format!(r#"url("{image_url}")"#))
        })
    }

    pub fn offset_signal(&self) -> impl Signal<Item = Option<String>> + use<> {
        self.highlight_offset.signal().map(|offset| {
            if offset == 0.0 {
                //common::debug_log!("No offset");
                return None;
            }

            Some(format!("{}px", offset * -32.0))
        })
    }
}

thread_local! {
    pub(crate) static ITEM_FRAME: ItemFrame = ItemFrame::default();
}

impl<T: AsRef<Node>> MdmaAddonWindow<T> for DomBuilder<T> {
    fn input(self, input: Input) -> Self {
        self.child(input.render())
    }
    fn dynamic_class_signal<B, C>(self, class_signal: C) -> Self
    where
        T: Clone + AsRef<HtmlElement> + 'static,
        B: MultiStr + 'static,
        C: Signal<Item = B> + 'static,
    {
        let mut old: Option<B> = None;
        let element = self.__internal_element();
        self.future(class_signal.for_each(move |cls| {
            if let Some(old) = old.take() {
                old.each(|old_cls| {
                    <T as std::convert::AsRef<HtmlElement>>::as_ref(&element)
                        .class_list()
                        .remove_1(old_cls)
                        .unwrap_js();
                });
            }

            cls.each(|new_cls| {
                <T as std::convert::AsRef<HtmlElement>>::as_ref(&element)
                    .class_list()
                    .add_1(new_cls)
                    .unwrap_js()
            });

            old = Some(cls);

            async {}
        }))
    }
    //fn section(self, content_section: ContentSection) -> Self {
    //    self.child(content_section.render())
    //}

    //fn input_signal_vec<B>(self, signal: B) -> Self
    //where
    //    B: SignalVec<Item = Input> + 'static,
    //{
    //    self.children_signal_vec(signal.map(|input| input.render()))
    //}
    //
    //fn input_pair(self, input_pair: InputPair) -> Self {
    //    self.child(input_pair.render())
    //}
    //
    //fn input_pair_signal<B>(self, signal: B) -> Self
    //where
    //    B: Signal<Item = Option<InputPair>> + 'static,
    //{
    //    self.child_signal(
    //        signal.map(|input_pair_opt| input_pair_opt.map(|input_pair| input_pair.render())),
    //    )
    //}
    //
    //fn button_pair(self, button_pair: ButtonPair) -> Self {
    //    self.child(button_pair.render())
    //}
    //
    //fn close_button(self, close_button: CloseButton<'_>) -> Self {
    //    apply_methods!(self, {
    //        .child(close_button.render())
    //    })
    //}
    //
    //fn settings_button(
    //    self,
    //    addon_data: &Rc<AddonData>,
    //    settings_button: SettingsButton<'_>,
    //) -> Self {
    //    if let Err(err) = Self::init_settings_toggle(addon_data) {
    //        console_error!(err);
    //    }
    //
    //    self.child(settings_button.render())
    //}

    //fn init_settings_toggle(addon_data: &Rc<AddonData>) -> JsResult<()> {
    //    let try_update_root_on_peak = clone!(addon_data => move |window_type: WindowType| {
    //        let root_lock = addon_data
    //            .get(&window_type)
    //            .root
    //            .borrow();
    //        let root = root_lock
    //            .as_ref()
    //            .ok_or_else(|| err_code!())?;
    //
    //        try_update_on_peak(&root, true)
    //    });
    //    let update_on_each_setting_button_click = addon_data
    //        .settings_window
    //        .is_active
    //        .signal()
    //        .for_each(move |active| {
    //            if let Err(err) = try_update_root_on_peak(match active {
    //                true => WindowType::SettingsWindow,
    //                false => WindowType::AddonWindow,
    //            }) {
    //                console_error!(err);
    //            }
    //
    //            async {}
    //        });
    //
    //    Ok(wasm_bindgen_futures::spawn_local(async move {
    //        update_on_each_setting_button_click.await
    //    }))
    //}

    fn build_draggable_window(self) -> Self
    where
        T: AsRef<HtmlElement> + AsRef<Element> + AsRef<EventTarget> + Clone + 'static,
    {
        self.class("last-on-peak")
            .event(move |event: MouseDown| {
                let Some(target) = event.dyn_target::<web_sys::HtmlElement>() else {
                    return;
                };

                let class_list = target.class_list();
                if class_list.contains("decor-settings-button")
                    || class_list.contains("decor-close-button")
                {
                    return;
                }

                let Some(target) = event.dyn_target::<web_sys::Node>() else {
                    return;
                };

                if let Err(err) = try_update_on_peak(&target, true) {
                    console_error!(err)
                }
            })
            .after_inserted(|window| {
                if let Err(err) = try_update_after_inserted(window.as_ref()) {
                    console_error!(err)
                }
            })
    }

    //fn update_on_peak(self, mutable: &Mutable<bool>) -> Self
    //where
    //    T: Clone + 'static,
    //{
    //    apply_methods!(self, {
    //        .with_node!(window => {
    //            .future(mutable.signal().for_each(move |is_visible: bool| {
    //                let ret = async {}; // This is so dumb XD
    //                if !is_visible {
    //                    return ret
    //                }
    //                if let Err(err) = try_update_on_peak(window.as_ref(), false) {
    //                    console_error!(err);
    //                }
    //                ret
    //            }))
    //        })
    //    })
    //}

    fn checkbox<B>(self, checkbox: Checkbox<B>) -> Self
    where
        B: Into<bool> + Not<Output = B> + Copy + PartialEq + 'static,
    {
        self.child(checkbox.render())
    }

    //fn checkbox_pair(self, first: Checkbox, second: Checkbox) -> Self {
    //    apply_methods!(self, {
    //        .child(html!("div", {
    //            .class("checkbox-pair-wrapper")
    //            .checkbox(first)
    //            .checkbox(second)
    //        }))
    //    })
    //}

    fn class_list(self, class_list: &ClassList<'static>) -> Self
    where
        T: Into<HtmlElement> + Clone,
    {
        apply_methods!(self, {
            .with_node!(element => {
                .apply(|builder| {
                    class_list.update_on_change(&element.into());

                    builder
                })
            })
        })
    }

    fn add_window_toggle(self, window_type: WindowType, addon_name: AddonName) -> Self {
        let toggle_button = Button::builder()
            .class_list("w[28] h[28]")
            .mixin(move |builder| {
                apply_methods!(builder, {
                    .child(html!("div", {
                        .class(window_type.to_toggle_class())
                        .class!(pos[absolute] t[6] l[7] f[none])
                    }))
                    .class_signal("selected", Addons::get_addon(addon_name).unwrap_js().get(window_type).active.signal())
                    //.tip!({
                    //    .text_signal(addons.lock_ref().get(&addon_key).unwrap_js().get(&window_type).active.signal().map(move |active| window_type.to_hover_text(active)))
                    //})
                })
            })
            .on_click(move |_| if let Err(err_code) = Addons::toggle_addon_active_state(addon_name, window_type) {
                console_error!(err_code);
            });

        self.child(toggle_button.render())
    }

    fn disable_dragging(self) -> Self
    where
        T: AsRef<EventTarget>,
    {
        apply_methods!(self, {
            .event(|_: MouseDown| {
                DRAGGING.with_borrow_mut(|is_moving| *is_moving = None);
            })
        })
    }

    //fn heading(self, heading: Heading) -> Self {
    //    apply_methods!(self, {
    //        .child(heading.render())
    //    })
    //}

    //fn init_root(self, root: &Rc<RefCell<Option<HtmlElement>>>) -> Self
    //where
    //    T: Into<HtmlElement> + Clone + 'static,
    //{
    //    let root = Rc::clone(root);
    //
    //    apply_methods!(self, {
    //        .with_node!(root_elem => {
    //            .apply(|builder| {
    //                *root.borrow_mut() = Some(root_elem.into());
    //
    //                builder
    //            })
    //        })
    //    })
    //}

    fn stop_input_propagation(self) -> Self
    where
        T: AsRef<EventTarget> + AsRef<HtmlInputElement>,
    {
        use crate::utils::window_events::KeyPress;
        use dominator::EventOptions;
        use dominator::events::{KeyDown, KeyUp};

        apply_methods!(self, {
            .event_with_options(&EventOptions::preventable(), move |event: KeyDown| {
                event.stop_immediate_propagation();
            })
            .event_with_options(&EventOptions::preventable(), move |event: KeyUp| {
                event.stop_immediate_propagation();
            })
            .event_with_options(&EventOptions::preventable(), move |event: KeyPress| {
                event.stop_immediate_propagation();
            })
        })
    }
}

pub(crate) trait Windows {
    fn get_element_with_child_node(&'static self, child: &Node) -> Option<HtmlElement>;
    fn push(&'static self, element: HtmlElement);
    fn remove_class(&'static self, class: &'static str);
}

impl Windows for LocalKey<RefCell<Vec<HtmlElement>>> {
    fn get_element_with_child_node(&'static self, child: &Node) -> Option<HtmlElement> {
        self.with_borrow(|windows| {
            windows
                .iter()
                .find(|window| window.contains(Some(child)))
                .cloned()
        })
    }

    fn push(&'static self, element: HtmlElement) {
        self.with_borrow_mut(|windows| {
            windows.push(element);
        });
    }

    fn remove_class(&'static self, class: &'static str) {
        self.with_borrow(|windows| {
            if let Some(window) = windows
                .iter()
                .find(|window| window.class_list().contains(class))
            {
                if let Err(_err) = window.class_list().remove(&Array::of1(&class.into())) {
                    debug_log!(_err);
                    console_error!();
                }
            }
        })
    }
}

pub(crate) trait BuildWindowsLayerEvents {
    fn build_windows_layer_events(self) -> Self;
}

impl BuildWindowsLayerEvents for DomBuilder<ShadowRoot> {
    fn build_windows_layer_events(self) -> Self {
        let this = self
            .global_event(move |event: MouseMove| {
                DRAGGING.with_borrow(|dragging| {
                    if let Some((rect, addon_window)) = dragging.as_ref() {
                        drag_window(rect, addon_window, event);
                    }
                });
            })
            // Remove last-on-peak class from window if user clicked on a border-window node.
            .global_event(|event: MouseDown| {
                let Some(target) = event.dyn_target::<Node>() else {
                    return;
                };

                match is_in_border_window(&target) {
                    Ok(true) => WINDOWS.remove_class(MDMA_ON_PEAK_CLASS),
                    Ok(false) => {}
                    Err(err) => console_error!(err),
                }
            })
            .global_event(|_: MouseUp| {
                DRAGGING.with_borrow_mut(|is_moving| *is_moving = None);
            });

        #[cfg(not(feature = "ni"))]
        let this =
            this.global_event_with_options(&dominator::EventOptions::bubbles(), |_: MouseDown| {
                if let Err(err_code) = tmp_disable_mouse_hero_move() {
                    console_error!(err_code);
                }
            });

        this
    }
}

/// Disables moving a player by holding down the left mouse button for the duration of the hold.
/// Re-enabling the player movement is done by the game automatically on the next mousedown event.
#[cfg(not(feature = "ni"))]
fn tmp_disable_mouse_hero_move() -> JsResult<()> {
    crate::bindings::get_engine()
        .g()
        .ok_or_else(|| err_code!())?
        .get_mouse_move()
        .ok_or_else(|| err_code!())?
        .set_active(false);

    Ok(())
}

#[inline]
fn drag_window(rect: &DomRect, addon_window: &AddonWindowDetails, event: MouseMove) {
    let mut max_x = window().inner_width().unwrap_js().as_f64().unwrap_js() - rect.width();
    max_x = (max_x - 1.0).max(1.0);
    let mut max_y = window().inner_height().unwrap_js().as_f64().unwrap_js() - rect.height();
    max_y = (max_y - 1.0).max(1.0);

    OFFSET.with_borrow(|(dx, dy)| {
        let new_x = (event.mouse_x() as f64 - *dx).clamp(1.0, max_x);
        let new_y = (event.mouse_y() as f64 - *dy).clamp(1.0, max_y);

        addon_window.left.set_neq(new_x);
        addon_window.top.set_neq(new_y);
    });
}

fn is_in_border_window(node: &Node) -> JsResult<bool> {
    use crate::utils::document;

    let alerts_layer = match cfg!(feature = "ni") {
        true => document()
            .get_elements_by_class_name("alerts-layer layer")
            .get_with_index(0)
            .ok_or_else(|| err_code!())?,
        false => document().document_element().ok_or_else(|| err_code!())?,
    };

    // Clicked inside mdma addon window.
    if !node.has_child_nodes() && node.parent_element().as_ref() == Some(&alerts_layer) {
        return Ok(false);
    }

    Ok(alerts_layer.contains(Some(node)))
}

pub(crate) fn try_update_on_peak(target: &Node, update_border_windows: bool) -> JsResult<()> {
    // Get the addon window that was clicked or return otherwise.
    let Some(new_on_peak) = WINDOWS.get_element_with_child_node(target) else {
        return Ok(());
    };

    //common::debug_log!("TRYING TO SET ON PEAK");
    try_set_on_peak(&new_on_peak)?;

    if update_border_windows {
        try_update_border_windows()?;
    }

    Ok(())
}

pub(crate) fn try_set_on_peak(window: &HtmlElement) -> JsResult<bool> {
    if window.class_list().contains(MDMA_ON_PEAK_CLASS)
        && !get_border_windows()?
            .iter()
            .any(|window| window.class_list().contains(GAME_ON_PEAK_CLASS))
    {
        return Ok(false);
    }

    WINDOWS.with_borrow(|windows| {
        if let Some(old_on_top) = windows.iter().find(|window_ref| {
            window_ref.class_list().contains(MDMA_ON_PEAK_CLASS) && window_ref != &window
        }) {
            if let Err(_err) = old_on_top
                .class_list()
                .remove(&Array::of1(&MDMA_ON_PEAK_CLASS.into()))
            {
                debug_log!(_err);
                console_error!();
            }
        }
    });

    update_z_index(window)?;

    window
        .class_list()
        .add(&Array::of1(&MDMA_ON_PEAK_CLASS.into()))
        .map_err(map_err!())?;

    Ok(true)
}

//pub(crate) fn try_remove_from_peak(window: &HtmlElement) -> Result<bool> {
//    if !window.class_list().contains(MDMA_ON_PEAK_CLASS) {
//        return Ok(false);
//    }
//
//    window
//        .class_list()
//        .remove(&Array::of1(&MDMA_ON_PEAK_CLASS.into()))
//        .map_err(|_| error::std::obf_delete!(MDMA_ON_PEAK_CLASS))?;
//
//    Ok(true)
//}

fn try_update_border_windows() -> JsResult<bool> {
    let border_windows = get_border_windows()?;
    let Some(old_on_peak) = border_windows
        .iter()
        .find(|window| window.class_list().contains(GAME_ON_PEAK_CLASS))
    else {
        return Ok(false);
    };

    old_on_peak
        .class_list()
        .remove(&Array::of1(&GAME_ON_PEAK_CLASS.into()))
        .map_err(map_err!())?;

    Ok(true)
}

pub(crate) fn try_update_after_inserted(inserted_window: &HtmlElement) -> JsResult<()> {
    update_z_index(inserted_window)?;
    try_update_border_windows()?;

    let Some(old_on_peak) = WINDOWS.with_borrow(|windows| {
        windows
            .iter()
            .find(|window| {
                window.class_list().contains(MDMA_ON_PEAK_CLASS) && *window != inserted_window
            })
            .cloned()
    }) else {
        return Ok(()); // First inserted window.
    };

    old_on_peak
        .class_list()
        .remove(&Array::of1(&MDMA_ON_PEAK_CLASS.into()))
        .map_err(map_err!())
}

fn get_border_windows() -> JsResult<Vec<Element>> {
    use crate::utils::document;

    let windows_collection = document().get_elements_by_class_name("border-window");
    let mut windows_vector: Vec<Element> = Vec::new();
    for i in 0..windows_collection.length() {
        let border_window = windows_collection
            .get_with_index(i)
            .ok_or_else(|| err_code!())?;
        windows_vector.push(border_window);
    }

    Ok(windows_vector)
}

// TODO: Signal for this ?
fn update_z_index(element: &HtmlElement) -> JsResult<()> {
    let engine = crate::bindings::get_engine();

    element
        .style()
        .set_property(
            intern("z-index"),
            &format!(
                "{}",
                engine.window_max_z_index().ok_or_else(|| err_code!())?
            ),
        )
        .map_err(map_err!())?;

    let new_window_max_z_index = engine.window_max_z_index().ok_or_else(|| err_code!())? + 1.0;

    engine.set_window_max_z_index(new_window_max_z_index);

    Ok(())
}
