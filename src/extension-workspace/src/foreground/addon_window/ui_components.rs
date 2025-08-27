use std::any::{Any, TypeId};
use std::ops::Not;

use common::throw_err_code;
use dominator::events::{Change, Click, Focus, Input as InputEvent, KeyDown};
use dominator::traits::OptionStr;
use dominator::{Dom, EventOptions, window_size};
use futures_signals::map_ref;
use futures_signals::signal::{MutableLockMut, MutableLockRef, MutableSignalRef};
use proc_macros::{add_class_list, builder, with_class_list};
use serde_json::{Value, json};
use wasm_bindgen::{JsCast, JsValue, intern};
use web_sys::HtmlLabelElement;

use crate::bindings::message;
use crate::globals::prelude::*;
use crate::s;
use crate::utils::dominator_helpers::{DomRectSignal, OverflowSignal};
use crate::utils::{
    SettingFromValue, SettingOption, UnwrapJsExt, generate_random_str, is_from_polish_alphabet,
};

use super::*;

const INVALID_CHARS_MSG: &str = "Tekst zawiera niedozwolone znaki!";
const INVALID_CAPITALIZATION_MSG: &str =
    "Wielka litera może znajdować się jedynie na początku słowa lub po znaku `-`!";
const REPETITION_MSG: &str = "Żaden znak nie może występować więcej niż dwa razy z rzędu!";
const SPECIAL_CHARS_MSG: &str = "Nick nie może zaczynać się ani kończyć specjalnymi znakami!";

macro_rules! make_component_container {
    ($container_name:ident) => {
        #[allow(unused)]
        impl $container_name {
            pub(crate) fn apply<C>(self, f: C) -> Self
            where
                C: FnOnce(Self) -> Self
            {
                f(self)
            }

            pub(crate) fn apply_if<C>(self, test: bool, f: C) -> Self
            where
                C: FnOnce(Self) -> Self
            {
                match test {
                    true => f(self),
                    false => self,
                }
            }

            pub fn dynamic_class_signal<B, C>(self, class_signal: C) -> Self
            where
                B: ::dominator::traits::MultiStr + 'static,
                C: Signal<Item = B> + 'static,
            {
                let mut old: Option<B> = None;
                let element = self.inner.__internal_element();
                let inner = self.inner.future(class_signal.for_each(move |cls| {
                    if let Some(old) = old.take() {
                        old.each(|old_cls| {
                            element.class_list().remove_1(old_cls).unwrap_js();
                        });
                    }

                    cls.each(|new_cls| element.class_list().add_1(new_cls).unwrap_js());

                    old = Some(cls);

                    async {}
                }));

                Self { inner, ..self }
            }

            pub(crate) fn after_inserted<F>(self, f: F) -> Self
            where
                F: FnOnce(HtmlDivElement) + 'static
            {
                Self {
                    inner: self.inner.after_inserted(f),
                    ..self
                }
            }

            pub(crate) fn after_removed<F>(self, f: F) -> Self
            where
                F: FnOnce(HtmlDivElement) + 'static
            {
                Self {
                    inner: self.inner.after_removed(f),
                    ..self
                }
            }

            pub(crate) fn mixin<B>(self, mixin: B) -> Self
            where
                B: FnOnce(DomBuilder<HtmlDivElement>) -> DomBuilder<HtmlDivElement>,
            {
                let inner = self.inner.apply(mixin);
                Self { inner, ..self }
            }

            pub(crate) fn visible_signal<B>(self, value: B) -> Self
            where
                B: Signal<Item = bool> + 'static {
                    let inner = self.inner.visible_signal(value);
                    Self { inner, ..self }
            }

            pub(crate) fn attr<B>(self, name: B, value: &str) -> Self
            where
                B: ::dominator::traits::MultiStr,
            {
                let inner = self.inner.attr(name, value);
                Self { inner, ..self }
            }

            pub(crate) fn class_signal<B, C>(self, class: B, signal: C) -> Self
            where
                B: ::dominator::traits::MultiStr + 'static,
                C: Signal<Item = bool> + 'static,
            {
                let inner = self.inner.class_signal(class, signal);

                Self { inner, ..self }
            }

            pub(crate) fn text(self, text: &str) -> Self {
                let inner = self.inner.text(text);
                Self { inner, ..self }
            }

            pub(crate) fn text_signal<B, C>(self, text: C) -> Self
            where
                B: ::dominator::traits::AsStr,
                C: Signal<Item = B> + 'static,
            {
                let inner = self.inner.text_signal(text);
                Self { inner, ..self }
            }

            pub(crate) fn global_event<T, F>(self, listener: F) -> Self
            where
                T: ::dominator::traits::StaticEvent,
                F: FnMut(T) + 'static,
            {
                Self {
                    inner: self.inner.global_event_with_options(&T::default_options(true), listener),
                    ..self
                }
            }

            pub(crate) fn event<T, F>(self, listener: F) -> Self
            where
                T: ::dominator::traits::StaticEvent,
                F: FnMut(T) + 'static,
            {
                Self {
                    inner: self.inner.event_with_options(&T::default_options(true), listener),
                    ..self
                }
            }

            pub(crate) fn custom_property_signal<B, C, D, E>(mut self, property_name: B, signal: E) -> Self
            where
                B: ::dominator::traits::MultiStr + 'static,
                C: ::dominator::traits::MultiStr,
                D: OptionStr<Output = C>,
                E: Signal<Item = D> + 'static
            {
                Self {
                    inner: self.inner.style_important_signal(property_name, signal),
                    ..self
                }
            }

            pub(crate) fn event_with_elem<T, F>(self, mut listener: F) -> Self
            where T: ::dominator::traits::StaticEvent,
                  F: FnMut(T, &HtmlDivElement) + 'static
            {
                let inner = apply_methods!(self.inner, {
                    .with_node!(container => {
                        .event_with_options(&T::default_options(true), move |event: T| listener(event, &container))
                    })
                });

                Self { inner, ..self }
            }

            pub fn event_with_options<T, F>(self, options: &EventOptions, listener: F) -> Self
                where T: ::dominator::traits::StaticEvent,
                      F: FnMut(T) + 'static {
                          Self {
                              inner: self.inner.event_with_options(options, listener),
                              ..self
                          }
            }

            pub(crate) fn global_event_with_options<T, F>(self, options: &EventOptions, listener: F) -> Self
            where
                T: ::dominator::traits::StaticEvent,
                F: FnMut(T) + 'static,
            {
                Self {
                    inner: self.inner.global_event_with_options(options, listener),
                    ..self
                }
            }


            pub(crate) fn input(self, input: Input) -> Self {
                Self {
                    inner: self.inner.child(input.render()),
                    ..self
                }
            }

            pub(crate) fn input_signal_vec<B>(self, signal: B) -> Self
            where
                B: SignalVec<Item = Input> + 'static,
            {
                Self {
                    inner: self
                        .inner
                        .children_signal_vec(signal.map(|input| input.render())),
                    ..self
                }
            }

            pub(crate) fn input_pair(self, input_pair: InputPair) -> Self {
                Self {
                    inner: self.inner.child(input_pair.render()),
                    ..self
                }
            }

            pub(crate) fn input_pair_signal<B>(self, signal: B) -> Self
            where
                B: Signal<Item = Option<InputPair>> + 'static,
            {
                Self {
                    inner: self.inner.child_signal(
                        signal.map(|input_pair_opt| input_pair_opt.map(|input_pair| input_pair.render())),
                    ),
                    ..self
                }
            }

            pub fn button(self, button: Button) -> Self {
                Self {
                    inner: self.inner.child(button.render()),
                    ..self
                }
            }

            pub(crate) fn button_pair(self, button_pair: ButtonPair) -> Self {
                Self {
                    inner: self.inner.child(button_pair.render()),
                    ..self
                }
            }

            pub(crate) fn checkbox<A>(self, checkbox: Checkbox<A>) -> Self
            where
                A: Into<bool> + Not<Output = A> + Copy + PartialEq + 'static,
            {
                Self {
                    inner: self.inner.child(checkbox.render()),
                    ..self
                }
            }

            pub(crate) fn checkbox_signal<'a, B, C>(self, checkbox_signal: C) -> Self
            where
                B: Into<bool> + Not<Output = B> + Copy + PartialEq + 'static,
                C: Signal<Item = Option<Checkbox<B>>> + 'static
            {
                Self {
                    inner: self.inner.child_signal(checkbox_signal.map(|checkbox_opt| checkbox_opt.map(|checkbox| checkbox.render()))),
                    ..self
                }
            }

            pub(crate) fn checkbox_pair<B, C>(self, first: Checkbox<B>, second: Checkbox<C>) -> Self
            where
                B: Into<bool> + Not<Output = B> + Copy + PartialEq + 'static,
                C: Into<bool> + Not<Output = C> + Copy + PartialEq + 'static,
            {
                Self {
                    inner: self.inner.child(html!("div", {
                        .class("checkbox-pair-wrapper")
                        .checkbox(first)
                        .checkbox(second)
                    })),
                    ..self
                }
            }

            //pub(crate) fn checkbox_pair_signal<D>(self, checkbox_pair_signal: D) -> Self
            //    where
            //        D: Signal<Item = Option<(Checkbox<'static>, Checkbox<'static>)>> + 'static
            //{
            //    Self {
            //        inner: self.inner.child_signal(checkbox_pair_signal.map(|checkboxes_opt| checkboxes_opt.map(|(first, second)| html!("div", {
            //            .class("checkbox-pair-wrapper")
            //            .checkbox(first)
            //            .checkbox(second)
            //        })))),
            //        ..self
            //    }
            //}

            pub(crate) fn heading(self, heading: Heading) -> Self {
                Self {
                    inner: self.inner.child(heading.render()),
                    ..self
                }
            }

            pub(crate) fn section(self, section: impl Section) -> Self {
                Self {
                    inner: self.inner.child(section.render()),
                    ..self
                }
            }

            pub(crate) fn section_signal<B, C>(self, section_signal: C) -> Self
            where
                B: Section,
                C: Signal<Item = Option<B>> + 'static,
            {
                Self {
                    inner: self.inner.child_signal(section_signal.map(|section_opt| section_opt.map(|section| section.render()))),
                    ..self
                }
            }

            pub(crate) fn section_signal_vec<B>(self, section_signal: B) -> Self
            where
                B: SignalVec<Item = ContentSection> + 'static,
            {
                Self {
                    inner: self.inner.children_signal_vec(section_signal.map(|section| section.render())),
                    ..self
                }
            }
        }
    }
}

pub trait Section {
    fn render(self) -> Dom;
}

// TODO: Make this a DomBuilder<T> wrapper ?
#[add_class_list]
pub(crate) struct ContentSection {
    inner: DomBuilder<HtmlDivElement>,
}

make_component_container!(ContentSection);

#[with_class_list]
impl ContentSection {
    #[builder]
    pub(crate) fn new() -> Self {
        Self {
            inner: DomBuilder::new_html("div"),
        }
    }

    pub(crate) fn render(self) -> Dom {
        let section = apply_methods!(self.inner, {});

        section.into_dom()
    }
}

impl Section for ContentSection {
    fn render(self) -> Dom {
        self.render()
    }
}

pub(crate) struct AddonWindow;

impl AddonWindow {
    pub(crate) fn builder(addon_name: AddonName) -> Window {
        Window::new(addon_name, WindowType::AddonWindow).class_list(addon_name.to_class())
    }
}

pub(crate) struct SettingsWindow;

impl SettingsWindow {
    pub(crate) fn builder(addon_name: AddonName) -> Window {
        Window::new(addon_name, WindowType::SettingsWindow).class_list(addon_name.to_class())
    }
}

#[add_class_list]
pub(crate) struct Window {
    header: Option<WindowHeader>,
    content: Option<WindowContent>,
    window_type: WindowType,
    addon_name: AddonName,
    item_slots: bool,
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl Window {
    #[builder(["window"])]
    fn new(addon_name: AddonName, window_type: WindowType) -> Self {
        Self {
            header: None,
            content: None,
            addon_name,
            inner: DomBuilder::new_html("div").build_draggable_window(),
            item_slots: false,
            window_type,
        }
    }

    pub fn has_item_slots(self, item_slots: bool) -> Self {
        Self { item_slots, ..self }
    }

    pub(crate) fn header(self, header: WindowHeader) -> Self {
        Self {
            header: Some(header),
            ..self
        }
    }

    //pub fn dynamic_class_signal<B, C>(self, class_signal: C) -> Self
    //where
    //    B: MultiStr + 'static,
    //    C: Signal<Item = B> + 'static,
    //{
    //    let mut old: Option<B> = None;
    //    let element = self.inner.__internal_element();
    //    let inner = self.inner.future(class_signal.for_each(move |cls| {
    //        if let Some(old) = old.take() {
    //            old.each(|old_cls| {
    //                element.class_list().remove_1(old_cls).unwrap_js();
    //            });
    //        }
    //
    //        cls.each(|new_cls| element.class_list().add_1(new_cls).unwrap_js());
    //
    //        old = Some(cls);
    //
    //        async {}
    //    }));
    //
    //    Self { inner, ..self }
    //}

    pub(crate) fn content(self, content: WindowContent) -> Self {
        Self {
            content: Some(content),
            ..self
        }
    }

    fn init_settings_toggle(addon_data: &'static AddonData) -> JsResult<()> {
        let try_update_root_on_peak = move |window_type: WindowType| {
            let data_lock = addon_data.get(window_type);
            let root_lock = data_lock.root.borrow();
            let root = root_lock.as_ref().ok_or_else(|| err_code!())?;
            // This approach won't work :(
            //common::debug_log!("HAS ON PEAK:", root.class_list().contains("last-on-peak"));

            try_update_on_peak(root, true)
        };
        // FIXME: On settings window close via it's close button the addon window get's updated.
        // Also both the addon window and the settings window should update after settings window
        // get's open.
        let update_on_each_setting_button_click = addon_data
            .settings_window
            .active
            .signal()
            .for_each(move |active| {
                // TODO: Don't update when settings window close button was clicked to receive the
                // active state.
                if let Err(err) = try_update_root_on_peak(WindowType::AddonWindow) {
                    console_error!(err);
                }
                if !active {
                    if let Err(err) = try_update_root_on_peak(WindowType::SettingsWindow) {
                        console_error!(err);
                    }
                }

                async {}
            });

        wasm_bindgen_futures::spawn_local(update_on_each_setting_button_click);

        Ok(())
    }

    pub(crate) fn build(self) -> JsResult<Dom> {
        let header = self.header.ok_or_else(|| err_code!())?;
        let content = self.content.ok_or_else(|| err_code!())?;

        let Some(addon_data) = Addons::get()[self.addon_name].as_ref() else {
            debug_log!(&format!("Could not init root for: {:?}", self.addon_name));
            return Err(err_code!());
        };

        if header.has_settings_button() {
            Self::init_settings_toggle(addon_data)?;
        }

        let addon_window_data = addon_data.get(self.window_type);
        let root = &addon_window_data.root;

        let window_builder = apply_methods!(self.inner, {
            .style_signal("left", addon_window_data.left.signal_ref(|pos| format!("{pos}px")))
            .style_signal("top", addon_window_data.top.signal_ref(|pos| format!("{pos}px")))
            .attr_signal("data-opacity-lvl", addon_window_data.opacity_lvl.signal_ref(|opacity| opacity.to_string()))
            .apply_if(header.has_size_toggle(), |b| b.attr_signal("data-size", addon_window_data.size.signal_ref(|size| size.to_string())))
            .apply(|b| {
                match self.window_type {
                    WindowType::AddonWindow => {
                        b.visible_signal(map_ref! {
                            let window_active = addon_window_data.active.signal(),
                            let addon_active = addon_data.active.signal() =>
                                *window_active && *addon_active
                        })
                    },
                    WindowType::SettingsWindow => b.visible_signal(addon_window_data.active.signal()),
                }
            })
            .with_node!(window_elem => {
                .apply(|builder| {
                    *root.borrow_mut() = Some(window_elem.clone().into());

                    builder
                })
                .child(header.build(window_elem.clone().into(), addon_data, self.window_type))
                .child(content.build(&self.addon_name, self.item_slots, addon_window_data))
                // TODO: Bounce-back mechanism still needs reworking.
                .future(
                    window_size()
                    .switch(|size| DomRectSignal::new(root.borrow().as_ref().unwrap_js()).map(move |rect| (rect, size)))
                    .for_each(|(rect, size)| {
                        let ret = async {};
                        if nearly_equal(rect.width(), size.width, 1.0)
                            || nearly_equal(rect.height(), size.height, 1.0)
                        {
                            return ret;
                        }

                        addon_window_data.left.set_neq(
                            addon_window_data
                                .left
                                .get()
                                .clamp(1.0, (size.width - rect.width() - 1.0).max(1.0)),
                        );
                        addon_window_data.top.set_neq(
                            addon_window_data
                                .top
                                .get()
                                .clamp(1.0, (size.height - rect.height() - 1.0).max(1.0)),
                        );

                        ret
                    })
                )
            })
        });

        Ok(window_builder.into_dom())
    }
}

fn nearly_equal(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

pub(crate) trait DecorButton: Any {
    fn render(self: Box<Self>, window_type: WindowType, addon_data: &'static AddonData) -> Dom;
}

/// The decors are always rendered outside-in, meaning the the last decor pushed to the right will
/// be the closest to the header text.
pub(crate) struct HeaderDecor {
    left_decor: Vec<Box<dyn DecorButton>>,
    right_decor: Vec<Box<dyn DecorButton>>,
}

impl HeaderDecor {
    const LEFT_DECOR_CLASS: &str = "mdma-left-decor";
    const RIGHT_DECOR_CLASS: &str = "mdma-right-decor";

    pub(crate) fn builder() -> HeaderDecorBuilder {
        HeaderDecorBuilder::new()
    }

    fn has_settings_button(&self) -> bool {
        self.left_decor
            .iter()
            .chain(self.right_decor.iter())
            .any(|button| (**button).type_id() == TypeId::of::<decors::SettingsButton>())
    }

    fn has_size_toggle(&self) -> bool {
        self.left_decor
            .iter()
            .chain(self.right_decor.iter())
            .any(|button| (**button).type_id() == TypeId::of::<decors::SizeToggle>())
    }
}

pub(crate) struct HeaderDecorBuilder {
    left_decor: Vec<Box<dyn DecorButton>>,
    right_decor: Vec<Box<dyn DecorButton>>,
}

impl HeaderDecorBuilder {
    fn new() -> Self {
        Self {
            left_decor: Vec::with_capacity(4),
            right_decor: Vec::with_capacity(4),
        }
    }

    pub(crate) fn push_left<T: DecorButton + 'static>(mut self, button: T) -> Self {
        self.left_decor.push(Box::new(button));

        Self { ..self }
    }

    pub(crate) fn push_right<T: DecorButton + 'static>(mut self, button: T) -> Self {
        self.right_decor.push(Box::new(button));

        Self { ..self }
    }

    pub(crate) fn build(self) -> HeaderDecor {
        HeaderDecor {
            left_decor: self.left_decor,
            right_decor: self.right_decor,
        }
    }
}

#[add_class_list]
pub(crate) struct WindowHeader {
    decor: HeaderDecor,
}

#[with_class_list]
impl WindowHeader {
    #[builder(["mdma-header-label"])]
    pub(crate) fn new(decor: HeaderDecor) -> Self {
        Self { decor }
    }

    fn init_drag_on_mousedown(
        event: MouseDown,
        window_element: &HtmlElement,
        addon_window: &'static AddonWindowDetails,
    ) {
        let rect = window_element.get_bounding_client_rect();
        OFFSET.with_borrow_mut(|offset| {
            *offset = (
                event.mouse_x() as f64 - rect.left(),
                event.mouse_y() as f64 - rect.top(),
            );
        });
        DRAGGING.with_borrow_mut(move |is_moving| *is_moving = Some((rect, addon_window)));
    }

    fn has_settings_button(&self) -> bool {
        self.decor.has_settings_button()
    }

    fn has_size_toggle(&self) -> bool {
        self.decor.has_size_toggle()
    }

    fn build(
        self,
        window_element: HtmlElement,
        addon_data: &'static AddonData,
        window_type: WindowType,
    ) -> Dom {
        WINDOWS.push(window_element.clone());
        let addon_window_data = addon_data.get(window_type);
        let header = DomBuilder::<HtmlDivElement>::new_html(intern("div"))
            .child(html!(intern("div"), {
                .class(HeaderDecor::LEFT_DECOR_CLASS)
                .apply(|b| {
                    b.children(self.decor
                        .left_decor
                        .into_iter()
                        .map(|d| d.render(window_type, addon_data))
                    )
                })
            }))
            .apply(|b| match window_type {
                WindowType::SettingsWindow => b.child(html!(intern("div"), {
                    .class(intern("mdma-text"))
                    // TODO: Remove `Option` from `header_description` ?
                    // SAFETY: It's safe to unwrap here since every header that get's rendered has to
                    // have a description text attached to it.
                    .text_signal(addon_window_data.header_description.as_ref().unwrap().signal_cloned())
                })),
                WindowType::AddonWindow => b.child(html!(intern("div"), {
                    .class([intern("d[flex]"), intern("f-d[row]"), intern("overflow[hidden]"), intern("a-i[center]")])
                    .child(html!(intern("div"), {
                        .class(intern("mdma-text"))
                        // SAFETY: Same as above.
                        .text_signal(addon_window_data.header_description.as_ref().unwrap().signal_cloned())
                        .with_node!(elem => {
                            .tip!(OverflowSignal::new(&elem) => {
                                // SAFETY: Same as above.
                                .text_signal(addon_window_data.header_description.as_ref().unwrap().signal_cloned())
                            })
                        })
                    }))
                    //.child(html!(intern("div"), {
                    //    .visible_signal(addon_data.get(WindowType::SettingsWindow).active.signal())
                    //    .class([intern("decor-pen-button"), intern("f[none]")])
                    //    .event(|_: Click| debug_log!("on pen click"))
                    //}))
                })),
            })
            .child(html!(intern("div"), {
                .class(HeaderDecor::RIGHT_DECOR_CLASS)
                .apply(|b| {
                    b.children(self.decor
                        .right_decor
                        .into_iter()
                        .map(|d| d.render(window_type, addon_data))
                        .rev()
                    )
                })
            }))
            .event(move |event: MouseDown| {
                Self::init_drag_on_mousedown(event, &window_element, addon_window_data)
            });

        let header = apply_methods!(header, {});

        header.into_dom()
    }
}

#[add_class_list]
pub(crate) struct WindowContent {
    inner: DomBuilder<HtmlDivElement>,
}

make_component_container!(WindowContent);

#[with_class_list]
impl WindowContent {
    #[builder(["window-content"])]
    pub(crate) fn builder() -> Self {
        Self {
            inner: DomBuilder::new_html("div"),
        }
    }

    fn build(
        self,
        addon_name: &AddonName,
        item_slots: bool,
        addon_window_data: &AddonWindowDetails,
    ) -> Dom {
        let content = apply_methods!(self.inner, {
            .visible_signal(addon_window_data.expanded.signal())
            .class(&format!("{}-content", addon_name.to_class()))
            .apply_if(item_slots, |b| {
                b.style_important_signal(
                    "--item-highlight",
                    ITEM_FRAME.with(|frame| frame.image_url_signal()),
                )
                .style_important_signal(
                    "--item-offset",
                    ITEM_FRAME.with(|frame| frame.offset_signal()),
                )
                .style_important_signal(
                    "--item-overlay",
                    ITEM_FRAME.with(|frame| frame.overlay_image_url_signal()),
                )
            })
        });

        content.into_dom()
    }
}

#[add_class_list]
pub(crate) struct InfoBubble {
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl InfoBubble {
    pub(crate) fn builder() -> InfoBubbleBuilder {
        InfoBubbleBuilder::new()
    }

    fn render(self) -> Dom {
        let info_bubble = apply_methods!(self.inner, {});

        info_bubble.into_dom()
    }
}

//TODO: Either use visible_signal and dont remove the tip in order to be able to use this struct as a content section
//or figure out a different way of dealing with FnMut requirements when passing a DomBuilder into tip! macro
pub(crate) struct InfoBubbleBuilder {
    inner: DomBuilder<HtmlDivElement>,
}

impl InfoBubbleBuilder {
    fn new() -> Self {
        Self {
            inner: DomBuilder::new_html("div"),
        }
    }

    pub(crate) fn apply<B>(self, mixin: B) -> Self
    where
        B: Fn(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement> + 'static,
    {
        let inner = apply_methods!(self.inner, {
            .tip!({
                .apply(&mixin)
            })
        });

        Self { inner }
    }

    pub(crate) fn text(self, text: &'static str) -> Self {
        use crate::interface::tips_parser::tip;

        let inner = apply_methods!(self.inner, {
            .tip!({
                .text(text)
            })
        });

        Self { inner }
    }

    pub(crate) fn text_signal<B, C, D>(self, mut text: D) -> Self
    where
        B: AsStr,
        C: Signal<Item = B> + 'static,
        D: FnMut() -> C + 'static,
    {
        use crate::interface::tips_parser::tip;

        let inner = apply_methods!(self.inner, {
            .tip!({
                .text_signal(text())
            })
        });

        Self { inner }
    }

    pub(crate) fn mixin<B>(self, mixin: B) -> Self
    where
        B: FnOnce(DomBuilder<HtmlDivElement>) -> DomBuilder<HtmlDivElement>,
    {
        let inner = self.inner.apply(mixin);
        Self { inner }
    }

    #[builder(["info-bubble"])]
    pub(crate) fn build(self) -> InfoBubble {
        InfoBubble { inner: self.inner }
    }
}

#[add_class_list]
pub(crate) struct Checkbox<A> {
    checked: Mutable<A>,
    wrapper_builder: DomBuilder<HtmlDivElement>,
    input_builder: DomBuilder<HtmlInputElement>,
    label_builder: DomBuilder<HtmlLabelElement>,
    info_bubble: Option<InfoBubble>,
    custom_event: bool,
}

#[with_class_list]
impl<A> Checkbox<A>
where
    A: Into<bool> + Not<Output = A> + Copy + PartialEq + 'static,
{
    // TODO: Wrap checked in an Option ?
    #[builder(["checkbox-wrapper"])]
    pub(crate) fn builder(checked: Mutable<A>) -> Self {
        Self {
            checked,
            wrapper_builder: DomBuilder::new_html("div"),
            input_builder: DomBuilder::new_html("input"),
            label_builder: DomBuilder::new_html("label"),
            info_bubble: None,
            custom_event: false,
        }
    }

    pub(crate) fn text(self, text: &str) -> Self {
        Self {
            label_builder: self.label_builder.text(text),
            ..self
        }
    }

    pub(crate) fn text_signal<B, C>(self, text: C) -> Self
    where
        B: AsStr,
        C: Signal<Item = B> + 'static,
    {
        Self {
            label_builder: self.label_builder.text_signal(text),
            ..self
        }
    }

    pub(crate) fn info_bubble(self, info_bubble: InfoBubble) -> Self {
        Self {
            info_bubble: Some(info_bubble),
            ..self
        }
    }

    pub(crate) fn label_mixin<B>(self, mixin: B) -> Self
    where
        B: FnOnce(DomBuilder<HtmlLabelElement>) -> DomBuilder<HtmlLabelElement>,
    {
        let label_builder = self.label_builder.apply(mixin);
        Self {
            label_builder,
            ..self
        }
    }

    /// After using this function signal provided in the builder will not be toggled on click.
    pub(crate) fn on_click<B>(self, on_click: B) -> Self
    where
        B: FnMut(Click) + 'static,
    {
        Self {
            input_builder: self
                .input_builder
                .event_with_options(&EventOptions::preventable(), on_click),
            custom_event: true,
            ..self
        }
    }

    pub(super) fn render(self) -> Dom {
        use futures_signals::signal::SignalExt;

        let id = generate_random_str(8);
        let checked = self.checked;
        let label = self
            .label_builder
            .class("checkbox-label")
            .class("checkbox-label--highlight")
            .class_signal(
                "active",
                checked.signal().dedupe_map(|active| (*active).into()),
            )
            .attr("for", &id)
            .into_dom();
        let input = self
            .input_builder
            .class("mdma-checkbox")
            .attr("type", "checkbox")
            .attr("id", &id)
            .prop_signal(
                "checked",
                checked.signal().dedupe_map(|active| (*active).into()),
            )
            .apply_if(!self.custom_event, |b| {
                b.event(move |_: Change| checked.toggle())
            })
            .into_dom();

        let checkbox = apply_methods!(self.wrapper_builder, {
            .child(input)
            .child(label)
            // TODO: Move this between the input and text.
            .apply_if(self.info_bubble.is_some(), |builder| builder.child(self.info_bubble.unwrap().render()))
        });

        checkbox.into_dom()
    }
}

#[add_class_list]
pub(crate) struct Heading {
    info_bubble: Option<InfoBubble>,
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl Heading {
    #[builder(["heading"])]
    pub(crate) fn builder() -> Self {
        Self {
            info_bubble: None,
            inner: DomBuilder::new_html("div"),
        }
    }

    pub(crate) fn text(self, text: &str) -> Self {
        let inner = self.inner.text(text);

        Self { inner, ..self }
    }

    //pub(crate) fn class_signal<B, C>(self, class: B, signal: C) -> Self
    //where
    //    B: MultiStr + 'static,
    //    C: Signal<Item = bool> + 'static,
    //{
    //    let inner = self.inner.class_signal(class, signal);
    //
    //    Self { inner, ..self }
    //}
    //
    pub(crate) fn mixin<B>(self, mixin: B) -> Self
    where
        B: FnOnce(DomBuilder<HtmlDivElement>) -> DomBuilder<HtmlDivElement>,
    {
        let inner = self.inner.apply(mixin);
        Self { inner, ..self }
    }

    pub(crate) fn info_bubble(self, info_bubble: InfoBubble) -> Self {
        Self {
            info_bubble: Some(info_bubble),
            ..self
        }
    }

    pub(super) fn render(self) -> Dom {
        let heading = apply_methods!(self.inner, {
            .apply_if(self.info_bubble.is_some(), |builder| builder.child(self.info_bubble.unwrap().render()))
        });

        heading.into_dom()
    }
}

#[derive(Clone, PartialEq)]
pub(crate) enum ButtonColor {
    Green,
    //Red,
}

impl ButtonColor {
    fn as_str(&self) -> &'static str {
        match self {
            ButtonColor::Green => "button-green",
        }
    }
}

#[add_class_list]
pub(crate) struct Button {
    color: ButtonColor,
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl Button {
    #[builder(["mdma-button"])]
    pub(crate) fn builder() -> Self {
        Self {
            color: ButtonColor::Green,
            inner: DomBuilder::new_html("div"),
        }
    }

    pub(crate) fn no_hover(self) -> Self {
        let inner = self.inner.class("no-hover");
        Self { inner, ..self }
    }

    //pub(crate) fn disabled(self, disabled: bool) -> Self {
    //    let inner = self.inner.apply_if(disabled, |b| b.class("disabled"));
    //    Self { inner, ..self }
    //}

    pub(crate) fn disabled_signal<B>(self, disabled_signal: B) -> Self
    where
        B: Signal<Item = bool> + 'static,
    {
        let inner = self.inner.class_signal("disabled", disabled_signal);
        Self { inner, ..self }
    }

    pub(crate) fn selected_signal<B>(self, selected_signal: B) -> Self
    where
        B: Signal<Item = bool> + 'static,
    {
        let inner = self.inner.class_signal("selected", selected_signal);
        Self { inner, ..self }
    }

    //pub(crate) fn color(self, color: ButtonColor) -> Self {
    //    Self {
    //        color: color.as_str(),
    //        ..self
    //    }
    //}

    pub(crate) fn mixin<B>(self, mixin: B) -> Self
    where
        B: FnOnce(DomBuilder<HtmlDivElement>) -> DomBuilder<HtmlDivElement>,
    {
        let inner = self.inner.apply(mixin);
        Self { inner, ..self }
    }

    pub(crate) fn text(self, text: &str) -> Self {
        Self {
            inner: self.inner.text(text),
            ..self
        }
    }

    pub(crate) fn scroll_wrapper(self, scroll: ScrollWrapper) -> Self {
        Self {
            inner: self.inner.child(scroll.render()),
            ..self
        }
    }

    //pub(crate) fn visible_signal<B>(self, value: B) -> Self
    //where
    //    B: Signal<Item = bool> + 'static,
    //{
    //    let inner = self.inner.visible_signal(value);
    //    Self { inner, ..self }
    //}
    //
    pub(crate) fn text_signal<B, C>(self, signal: C) -> Self
    where
        B: AsStr,
        C: Signal<Item = B> + 'static,
    {
        Self {
            inner: self.inner.text_signal(signal),
            ..self
        }
    }

    pub(crate) fn on_mousedown<B>(self, on_mousedown: B) -> Self
    where
        B: FnMut(MouseDown) + 'static,
    {
        Self {
            inner: self
                .inner
                .event_with_options(&EventOptions::preventable(), on_mousedown),
            ..self
        }
    }

    pub(crate) fn on_click<B>(self, on_click: B) -> Self
    where
        B: FnMut(Click) + 'static,
    {
        Self {
            inner: self
                .inner
                .event_with_options(&EventOptions::preventable(), on_click),
            ..self
        }
    }

    pub(super) fn render(self) -> Dom {
        let button = apply_methods!(self.inner, {
            .class(self.color.as_str())
        });

        button.into_dom()
    }
}

#[add_class_list]
pub(crate) struct ButtonPair {
    first: Button,
    second: Button,
}

#[with_class_list]
impl ButtonPair {
    #[builder(["button-pair-wrapper"])]
    pub(crate) fn builder(first: Button, second: Button) -> Self {
        Self { first, second }
    }

    pub(super) fn render(self) -> Dom {
        html!("div", {
            .child(self.first.render())
            .child(self.second.render())
        })
    }
}

type OnInputButtonClick = Box<dyn FnMut(Click, &HtmlInputElement) + 'static>;

pub(crate) enum InputButtonType {
    Add,
    Remove,
}

#[add_class_list]
pub(crate) struct InputButton {
    text: &'static str,
    button_type: InputButtonType,
    on_click: OnInputButtonClick,
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl InputButton {
    #[builder(["input-button"])]
    pub(crate) fn builder() -> Self {
        Self {
            button_type: InputButtonType::Add,
            text: "✕",
            on_click: Box::new(
                #[inline]
                |_, _| {},
            ),
            inner: DomBuilder::new_html("div"),
        }
    }

    pub(crate) fn button_type(self, button_type: InputButtonType) -> Self {
        Self {
            button_type,
            ..self
        }
    }

    pub(crate) fn with_tooltip<B>(self, tooltip: &Mutable<B>) -> Self
    where
        B: AsRef<str> + AsStr + Clone + 'static,
    {
        use crate::interface::tips_parser::tip;

        let inner = apply_methods!(self.inner, {
            .class_signal("invalid", tooltip.signal_ref(|tip| !tip.as_ref().is_empty()))
            .tip!(
                @statements { let tooltip = tooltip.clone() },
                tooltip.signal_ref(|custom_validity| !custom_validity.as_ref().is_empty()) =>
                {
                    .text_signal(tooltip.signal_cloned())
                },
            )
        });

        Self { inner, ..self }
    }

    pub(crate) fn tip(self, tip_text: &'static str) -> Self {
        use crate::interface::tips_parser::tip;

        let inner = apply_methods!(self.inner, {
            .tip!({
                .text(tip_text)
            })
        });

        Self { inner, ..self }
    }

    pub(crate) fn on_click<B>(self, on_click: B) -> Self
    where
        B: FnMut(Click, &HtmlInputElement) + 'static,
    {
        Self {
            on_click: Box::new(on_click),
            ..self
        }
    }

    pub(super) fn render(mut self, input_elem: HtmlInputElement) -> Dom {
        let input_button = apply_methods!(self.inner, {
            .class(match self.button_type {
                InputButtonType::Add => "button-add",
                InputButtonType::Remove => "button-remove",
            })
            .child(html!("span", {
                .class("input-confirm")
                .text(self.text)
            }))
            .event(move |event: Click| { self.on_click.as_mut()(event, &input_elem);})
        });

        input_button.into_dom()
    }
}

#[derive(Clone, PartialEq)]
pub(crate) enum InputSize {
    Small,
    Big,
    Color,
    Custom(&'static str),
}

#[derive(Clone, PartialEq)]
pub(crate) enum TextAlign {
    Left,
    Center,
}

impl AsRef<str> for TextAlign {
    fn as_ref(&self) -> &'static str {
        match self {
            TextAlign::Left => "t-a[left]",
            TextAlign::Center => "t-a[center]",
        }
    }
}

pub(crate) enum InputType {
    Keybind,
    Text,
    Item { canvas: Dom },
    Number { min: f64, max: f64 },
    Slider { min: f64, max: f64 },
    Color,
}

impl InputType {
    pub(crate) fn slider(min: f64, max: f64) -> Self {
        Self::Slider { min, max }
    }

    pub(crate) fn number(min: f64, max: f64) -> Self {
        Self::Number { min, max }
    }

    pub(crate) fn item(canvas: Dom) -> Self {
        Self::Item { canvas }
    }

    pub(crate) fn keybind() -> Self {
        Self::Keybind
    }

    pub(crate) fn text() -> Self {
        Self::Text
    }

    pub(crate) fn color() -> Self {
        Self::Color
    }

    fn is_item_input(&self) -> bool {
        matches!(self, Self::Item { .. })
    }
}

#[add_class_list]
pub(crate) struct Input {
    title: &'static str,
    text_align: TextAlign,
    size: InputSize,
    input_type: InputType,
    confirm_button: Option<InputButton>,
    inner: DomBuilder<HtmlInputElement>,
    text_builder: Option<DomBuilder<HtmlDivElement>>,
}

#[with_class_list]
impl Input {
    #[builder]
    pub(crate) fn builder() -> Self {
        Self {
            title: "",
            text_align: TextAlign::Center,
            size: InputSize::Small,
            confirm_button: None,
            inner: DomBuilder::new_html("input"),
            input_type: InputType::text(),
            text_builder: None,
        }
    }

    pub(crate) fn disabled(self) -> Self {
        let inner = self.inner.class("disabled").attr("disabled", "true");

        Self { inner, ..self }
    }

    pub(crate) fn placeholder<B: AsRef<str>>(self, placeholder: B) -> Self {
        let inner = self.inner.attr("placeholder", placeholder.as_ref());

        Self { inner, ..self }
    }

    pub(crate) fn placeholder_signal<B, C, D>(self, placeholder_signal: D) -> Self
    where
        B: AsStr,
        C: OptionStr<Output = B>,
        D: Signal<Item = C> + 'static,
    {
        let inner = self.inner.attr_signal("placeholder", placeholder_signal);

        Self { inner, ..self }
    }

    //pub(crate) fn maybe_value<B: AsRef<str>>(self, value: Option<B>) -> Self {
    //    let Some(value) = value else {
    //        return self;
    //    };
    //
    //    let inner = self.inner.attr("value", value.as_ref());
    //
    //    Self { inner, ..self }
    //}

    pub(crate) fn value<B: AsRef<str>>(self, value: B) -> Self {
        let inner = self.inner.attr("value", value.as_ref());

        Self { inner, ..self }
    }

    pub(crate) fn value_signal<B, C, D>(self, value_signal: D) -> Self
    where
        B: AsStr,
        C: OptionStr<Output = B>,
        D: Signal<Item = C> + 'static,
    {
        let inner = self.inner.attr_signal("value", value_signal);

        Self { inner, ..self }
    }

    //pub(crate) fn title(self, title: &'static str) -> Self {
    //    Self { title, ..self }
    //}

    pub(crate) fn with_tooltip<B>(self, tooltip: &Mutable<B>) -> Self
    where
        B: AsRef<str> + AsStr + Clone + 'static,
    {
        use crate::interface::tips_parser::tip;

        let inner = apply_methods!(self.inner, {
            .tip!(
                @statements { let tooltip = tooltip.clone() },
                tooltip.signal_ref(|custom_validity| !custom_validity.as_ref().is_empty()) =>
                {
                    .text_signal(tooltip.signal_cloned())
                },
            )
        });

        Self { inner, ..self }
    }

    //pub(crate) fn tip(self, tip_text: &'static str) -> Self {
    //    use crate::interface::tips_parser::tip;
    //
    //    let inner = apply_methods!(self.inner, {
    //        .tip!({
    //            .text(tip_text)
    //        })
    //    });
    //
    //    Self { inner, ..self }
    //}

    pub(crate) fn mixin<B>(self, apply_closure: B) -> Self
    where
        B: FnOnce(DomBuilder<HtmlInputElement>, HtmlInputElement) -> DomBuilder<HtmlInputElement>,
    {
        let inner = apply_methods!(self.inner, {
            .with_node!(input_elem => {
                .apply(|builder| apply_closure(builder, input_elem))
            })
        });

        Self { inner, ..self }
    }

    pub(crate) fn store_root(self, container: &Mutable<Option<HtmlInputElement>>) -> Self {
        let inner = apply_methods!(self.inner, {
            .with_node!(root => {
                .apply(|_builder| {
                    container.set_neq(Some(root));
                    _builder
                })
            })
        });

        Self { inner, ..self }
    }

    pub(crate) fn maxlength(self, maxlength: &str) -> Self {
        let inner = self.inner.attr("maxlength", maxlength);

        Self { inner, ..self }
    }

    pub(crate) fn confirm_button(self, confirm_button: InputButton) -> Self {
        Self {
            confirm_button: Some(confirm_button),
            ..self
        }
    }

    pub(crate) fn size(self, size: InputSize) -> Self {
        Self { size, ..self }
    }

    pub(crate) fn input_type(self, input_type: InputType) -> Self {
        Self { input_type, ..self }
    }

    pub(crate) fn text_align(self, text_align: TextAlign) -> Self {
        Self { text_align, ..self }
    }

    pub(crate) fn text(self, text: &'static str) -> Self {
        if self.text_builder.is_some() {
            throw_err_code!("Can not define multiple text nodes inside single input.")
        }

        let text_builder = Some(DomBuilder::new_html("div").text(text));

        Self {
            text_builder,
            ..self
        }
    }

    pub(crate) fn text_signal<B, C, D>(self, text_signal: D) -> Self
    where
        B: AsStr,
        C: Signal<Item = B> + 'static,
        D: Fn() -> C,
    {
        if self.text_builder.is_some() {
            throw_err_code!("Can not define multiple text nodes inside single input.")
        }

        let text_builder = Some(
            DomBuilder::new_html("div")
                .text_signal(text_signal())
                .class_signal("p[0]", text_signal().map(|s| s.with_str(|s| s.is_empty()))),
        );

        Self {
            text_builder,
            ..self
        }
    }

    pub(crate) fn on_input<B: FnMut(InputEvent, &HtmlInputElement) + 'static>(
        self,
        mut on_input: B,
    ) -> Self {
        let inner = apply_methods!(self.inner, {
            .with_node!(input_elem => {
                .event_with_options(&EventOptions::preventable(), move |event: InputEvent| on_input(event, &input_elem))
            })
        });

        Self { inner, ..self }
    }

    pub(crate) fn on_key_down<B: FnMut(KeyDown, &HtmlInputElement) + 'static>(
        self,
        mut on_key_down: B,
    ) -> Self {
        let inner = apply_methods!(self.inner, {
            .with_node!(input_elem => {
                .event_with_options(&EventOptions::preventable(), move |event: KeyDown| on_key_down(event, &input_elem))
            })
        });

        Self { inner, ..self }
    }

    pub(crate) fn on_click<B: FnMut(Click, &HtmlInputElement) + 'static>(
        self,
        mut on_click: B,
    ) -> Self {
        let inner = apply_methods!(self.inner, {
            .with_node!(input_elem => {
                .event_with_options(&EventOptions::preventable(), move |event: Click| on_click(event, &input_elem))
            })
        });

        Self { inner, ..self }
    }

    pub(super) fn render(self) -> Dom {
        use dominator::attrs;

        let mut root = JsValue::UNDEFINED.unchecked_into::<HtmlInputElement>();
        let Self {
            confirm_button,
            size,
            text_align,
            text_builder,
            inner,
            title,
            input_type,
            ..
        } = self;

        let input = apply_methods!(inner, {
            .with_node!(input_elem => {
                .apply(|builder| {
                    root = input_elem.clone();
                    builder
                })
            })
            .class("mdma-input")
            .class(text_align.as_ref())
            .apply(|builder| match &input_type {
                InputType::Slider { min, max } => apply_methods!(builder, {
                    .attrs! {
                        min: min.to_string().as_str(),
                        max: max.to_string().as_str(),
                        "type": "range",
                    }
                    .class("slider-input")
                }),
                InputType::Number { min, max } => apply_methods!(builder, {
                    .attrs! {
                        min: min.to_string().as_str(),
                        max: max.to_string().as_str(),
                        "type": "number",
                    }
                }),
                InputType::Color => apply_methods!(builder, {
                    .attrs! {
                        "type": "color",
                    }
                }),
                InputType::Item { .. } => {
                    builder.attr("maxlength", "0")
                        .class("item-input")
                        .event_with_options(&EventOptions::preventable(), |event: Focus| {
                            event.prevent_default();
                            event.dyn_target::<HtmlInputElement>().unwrap_js().blur().unwrap_js();
                        })
                }
                InputType::Text => builder,
                InputType::Keybind => apply_methods!(builder, { .class!(keybind t-a[center]) }),
            })
            .apply_if(!input_type.is_item_input(), |builder| builder.class(match size {
                InputSize::Small => "small-input",
                InputSize::Big => "big-input",
                InputSize::Color => "color-input",
                InputSize::Custom(size) => size,
            }))
            .apply_if(confirm_button.is_some(), |builder| {
                builder.class("b-r-top-right[0]").class("b-r-bottom-right[0]")
            })
            .attr("spellcheck", "false")
            .attr("title", title)
            .attr("name", &generate_random_str(8))
            .stop_input_propagation()
        });

        html!("div", {
            .class("d[flex]")
            .apply_if(input_type.is_item_input(), |builder| builder.class("pos[relative]"))
            .apply_if(text_builder.is_some(), |builder| builder.child(text_builder.unwrap()
                .class("mdma-text")
                .class("l-h[24]")
                .class("f-s[12]")
                .into_dom()
            ))
            .child(input.into_dom())
            .apply(|builder| match input_type {
                InputType::Item { canvas } => builder.child(canvas),
                _ => builder,
            })
            .apply(|builder| match confirm_button {
                Some(confirm_button) => builder.child(confirm_button.render(root)),
                None => builder
            })
        })
    }
}

#[add_class_list]
#[derive(Clone)]
struct InputDelimiter {
    text: &'static str,
}

#[with_class_list]
impl InputDelimiter {
    #[builder(["mdma-text", "f[none]", "l-h[24]"])]
    fn builder(text: &'static str) -> Self {
        Self { text }
    }

    pub(super) fn render(self) -> Dom {
        html!("div", {
            .text(self.text)
        })
    }
}

#[add_class_list]
pub(crate) struct InputPair {
    delimiter: Option<InputDelimiter>,
    first: Input,
    second: Input,
}

#[with_class_list]
impl InputPair {
    #[builder(["input-pair-wrapper"])]
    pub(crate) fn builder(first: Input, second: Input) -> Self {
        Self {
            delimiter: Some(InputDelimiter::builder("-")),
            first,
            second,
        }
    }

    pub(crate) fn no_delimiter(self) -> Self {
        Self {
            delimiter: None,
            ..self
        }
    }

    pub(super) fn render(self) -> Dom {
        html!("div", {
            .input(self.first)
            .apply(|builder| match self.delimiter {
                Some(delimiter) => builder.child(delimiter.render()),
                None => builder,
            })
            .input(self.second)
        })
    }
}

pub(crate) trait SetNeqable<T> {
    fn set_neq(&self, value: T);
}

impl<T: PartialEq> SetNeqable<T> for Mutable<T> {
    fn set_neq(&self, value: T) {
        Mutable::set_neq(self, value)
    }
}

impl<T: PartialEq> SetNeqable<T> for &Mutable<T> {
    fn set_neq(&self, value: T) {
        Mutable::set_neq(self, value)
    }
}

pub(crate) trait ChangeValidity<T>: SetNeqable<T>
where
    T: AsRef<str>,
{
    #[inline]
    fn set_invalid(&self, input_elem: &HtmlInputElement, validation_error: T) {
        input_elem.set_custom_validity(validation_error.as_ref());
        self.set_neq(validation_error);
    }

    fn set_valid(&self, input_elem: &HtmlInputElement);
}

impl ChangeValidity<String> for Mutable<String> {
    #[inline]
    fn set_valid(&self, input_elem: &HtmlInputElement) {
        input_elem.set_custom_validity("");
        self.set_neq(String::new());
    }
}
impl ChangeValidity<String> for &Mutable<String> {
    #[inline]
    fn set_valid(&self, input_elem: &HtmlInputElement) {
        input_elem.set_custom_validity("");
        self.set_neq(String::new());
    }
}
impl ChangeValidity<&'static str> for Mutable<&'static str> {
    #[inline]
    fn set_valid(&self, input_elem: &HtmlInputElement) {
        input_elem.set_custom_validity("");
        self.set_neq("");
    }
}
impl ChangeValidity<&'static str> for &Mutable<&'static str> {
    #[inline]
    fn set_valid(&self, input_elem: &HtmlInputElement) {
        input_elem.set_custom_validity("");
        self.set_neq("");
    }
}

#[derive(Debug, Default)]
pub(crate) struct NickInput {
    pub(crate) nicks: Mutable<Vec<String>>,
    pub(crate) custom_validity: Mutable<&'static str>,
    pub(crate) root: Mutable<Option<HtmlInputElement>>,
}

impl SettingOption for NickInput {
    fn as_option_signal(&self, f: fn(Value) -> Value) -> impl Signal<Item = Value> {
        self.signal_ref(move |data| f(json!(data)))
    }
}

impl SettingFromValue for NickInput {
    fn update(&self, value: Value) {
        if let Some(values) = value.as_array() {
            self.replace(
                values
                    .iter()
                    .map(|nick| nick.as_str().unwrap_js().to_owned())
                    .collect(),
            );
        }
    }
}

impl NickInput {
    pub(crate) fn replace(&self, nicks: Vec<String>) -> Vec<String> {
        self.nicks.replace(nicks)
    }

    pub(crate) fn signal_ref<B, F>(&self, f: F) -> MutableSignalRef<Vec<String>, F>
    where
        F: FnMut(&Vec<String>) -> B,
    {
        self.nicks.signal_ref(f)
    }

    pub(crate) fn lock_ref(&self) -> MutableLockRef<'_, Vec<String>> {
        self.nicks.lock_ref()
    }

    pub(crate) fn lock_mut(&self) -> MutableLockMut<'_, Vec<String>> {
        self.nicks.lock_mut()
    }

    pub(crate) fn on_input_factory(
        nicks: &'static Self,
    ) -> impl FnMut(InputEvent, &HtmlInputElement) + 'static {
        let NickInput {
            nicks,
            custom_validity,
            ..
        } = nicks;

        #[inline]
        move |_, input_elem| {
            let nick = input_elem.value().trim().to_string();

            if nick.is_empty() {
                return custom_validity.set_valid(input_elem);
            }
            if nick.chars().count() < 3 {
                return custom_validity.set_invalid(input_elem, "Nick jest zbyt krótki.");
            }
            if nick.starts_with('-') || nick.ends_with('-') {
                return custom_validity.set_invalid(input_elem, SPECIAL_CHARS_MSG);
            }
            if nicks.lock_ref().contains(&nick) {
                return custom_validity
                    .set_invalid(input_elem, "Ten nick znajduje się już na liście.");
            }

            let has_invalid_char = nick
                .chars()
                .any(|char| !is_from_polish_alphabet(char) && !char.is_whitespace() && char != '-');
            if has_invalid_char {
                return custom_validity.set_invalid(input_elem, INVALID_CHARS_MSG);
            }

            let correct_capitalization = nick.chars().enumerate().all(|(i, c)| {
                !c.is_uppercase()
                    || i == 0
                    || nick
                        .chars()
                        .nth(i - 1)
                        .is_some_and(|c| c == '-' || c.is_whitespace())
            });
            if !correct_capitalization {
                return custom_validity.set_invalid(input_elem, INVALID_CAPITALIZATION_MSG);
            }

            let has_repetition = nick
                .chars()
                .zip(nick.chars().skip(1))
                .zip(nick.chars().skip(2))
                .any(|((a, b), c)| a == b && b == c);
            if has_repetition {
                return custom_validity.set_invalid(input_elem, REPETITION_MSG);
            }

            custom_validity.set_valid(input_elem);
        }
    }

    pub(crate) fn on_click_factory(
        nicks: &'static Self,
    ) -> impl FnMut(Click, &HtmlInputElement) + 'static {
        move |_, input_elem| {
            input_elem.set_value(input_elem.value().trim());

            let Ok(validation_message) = input_elem.validation_message() else {
                return console_error!();
            };

            if !validation_message.is_empty() {
                message(&validation_message).unwrap_js();
                return;
            }

            let nick = input_elem.value().trim().to_lowercase();

            if nick.is_empty() {
                message("Nick jest zbyt krótki.").unwrap_js();
                return;
            }

            nicks.lock_mut().push(nick);
            input_elem.set_value("");
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ItemNameInput {
    pub(crate) item_names: Mutable<Vec<String>>,
    pub(crate) custom_validity: Mutable<&'static str>,
    pub(crate) root: Mutable<Option<HtmlInputElement>>,
}

impl SettingOption for ItemNameInput {
    fn as_option_signal(&self, f: fn(Value) -> Value) -> impl Signal<Item = Value> {
        self.signal_ref(move |data| f(json!(data)))
    }
}

impl SettingFromValue for ItemNameInput {
    fn update(&self, value: Value) {
        if let Some(values) = value.as_array() {
            self.replace(
                values
                    .iter()
                    .map(|nick| nick.as_str().unwrap_js().to_owned())
                    .collect(),
            );
        }
    }
}

impl ItemNameInput {
    pub(crate) fn replace(&self, nicks: Vec<String>) -> Vec<String> {
        self.item_names.replace(nicks)
    }

    pub(crate) fn signal_ref<B, F>(&self, f: F) -> MutableSignalRef<Vec<String>, F>
    where
        F: FnMut(&Vec<String>) -> B,
    {
        self.item_names.signal_ref(f)
    }

    pub(crate) fn lock_ref(&self) -> MutableLockRef<'_, Vec<String>> {
        self.item_names.lock_ref()
    }

    pub(crate) fn lock_mut(&self) -> MutableLockMut<'_, Vec<String>> {
        self.item_names.lock_mut()
    }

    pub(crate) fn on_input_factory(
        names_input: &'static Self,
    ) -> impl FnMut(InputEvent, &HtmlInputElement) + 'static {
        let ItemNameInput {
            item_names,
            custom_validity,
            ..
        } = names_input;

        #[inline]
        move |_, input_elem| {
            let item_name = input_elem.value().trim().to_lowercase().to_string();

            if item_name.is_empty() {
                return custom_validity.set_valid(input_elem);
            }
            if item_name.chars().count() > 60 {
                return custom_validity
                    .set_invalid(input_elem, "Nazwa przedmiotu jest zbyt długa.");
            }
            if item_names.lock_ref().contains(&item_name) {
                return custom_validity.set_invalid(
                    input_elem,
                    "Ta nazwa przedmiotu znajduje się już na liście.",
                );
            }

            let has_invalid_char = item_name.chars().any(|char| {
                !is_from_polish_alphabet(char)
                    && !char.is_whitespace()
                    && char != '-'
                    && char != '\''
            });
            if has_invalid_char {
                return custom_validity.set_invalid(input_elem, INVALID_CHARS_MSG);
            }

            custom_validity.set_valid(input_elem);
        }
    }

    pub(crate) fn on_click_factory(
        names_input: &'static Self,
    ) -> impl FnMut(Click, &HtmlInputElement) + 'static {
        move |_, input_elem| {
            input_elem.set_value(input_elem.value().trim());

            let Ok(validation_message) = input_elem.validation_message() else {
                return console_error!();
            };

            if !validation_message.is_empty() {
                message(&validation_message).unwrap_js();
                return;
            }

            let item_name = input_elem.value().trim().to_lowercase();

            if item_name.is_empty() {
                message("Nick jest zbyt krótki.").unwrap_js();
                return;
            }

            names_input.lock_mut().push(item_name);
            input_elem.set_value("");
        }
    }
}

#[add_class_list]
pub struct ScrollWrapper {
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl ScrollWrapper {
    pub fn builder<B, C>(on_blur_factory: C) -> ScrollWrapperBuilder<C>
    where
        B: FnMut() + 'static,
        C: FnMut() -> B,
    {
        ScrollWrapperBuilder::new(on_blur_factory)
    }

    fn render(self) -> Dom {
        let scroll_wrapper = apply_methods!(self.inner, {});

        scroll_wrapper.into_dom()
    }
}

impl Section for ScrollWrapper {
    fn render(self) -> Dom {
        self.render()
    }
}

#[add_class_list]
pub struct ScrollWrapperBuilder<C> {
    inner: DomBuilder<HtmlDivElement>,
    on_blur_factory: C,
}

#[with_class_list(in_builder)]
impl<B, C> ScrollWrapperBuilder<C>
where
    B: FnMut() + 'static,
    C: FnMut() -> B,
{
    #[builder(["scroll-wrapper"])]
    fn new(mut on_blur_factory: C) -> Self {
        let mut clb = on_blur_factory();
        let inner = DomBuilder::new_html("div")
            .global_event_with_options(&EventOptions::bubbles(), move |_: MouseDown| clb());

        Self {
            inner,
            on_blur_factory,
        }
    }

    // TODO: Should this take in the option closure?
    pub fn option_if(mut self, test: bool, option: impl FnOnce() -> ScrollWrapperOption) -> Self {
        let clb = (self.on_blur_factory)();
        let inner = self.inner.apply_if(test, |b| b.child(option().render(clb)));

        Self { inner, ..self }
    }

    pub fn option(mut self, option: ScrollWrapperOption) -> Self {
        let clb = (self.on_blur_factory)();
        let inner = self.inner.child(option.render(clb));

        Self { inner, ..self }
    }

    pub fn visible_signal(self, signal: impl Signal<Item = bool> + 'static) -> Self {
        let inner = self.inner.visible_signal(signal);

        Self { inner, ..self }
    }

    pub fn build(self) -> ScrollWrapper {
        ScrollWrapper {
            inner: self.inner,
            class_list: self.class_list,
        }
    }
}

#[add_class_list]
pub struct ScrollWrapperOption {
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl ScrollWrapperOption {
    pub fn builder() -> ScrollWrapperOptionBuilder {
        ScrollWrapperOptionBuilder::new()
    }

    fn render<B>(self, mut clb: B) -> Dom
    where
        B: FnMut() + 'static,
    {
        let option = apply_methods!(self.inner, {
            .event(move |_: Click| clb())
        });

        option.into_dom()
    }
}

#[add_class_list]
pub struct ScrollWrapperOptionBuilder {
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list(in_builder)]
impl ScrollWrapperOptionBuilder {
    #[builder(["scroll-option"])]
    fn new() -> Self {
        Self {
            inner: DomBuilder::new_html("div")
                .event_with_options(&EventOptions::bubbles(), |event: MouseDown| {
                    event.stop_propagation()
                }),
        }
    }

    pub fn text(self, text: &str) -> Self {
        let inner = self.inner.text(text);

        Self { inner, ..self }
    }

    //pub fn text_signal<B, C>(self, signal: C) -> Self
    //where
    //    B: AsStr,
    //    C: Signal<Item = B> + 'static,
    //{
    //    let inner = self.inner.text_signal(signal);
    //
    //    Self { inner, ..self }
    //}

    pub fn on_click<F: FnMut(Click) + 'static>(self, clb: F) -> Self {
        let inner = self.inner.event(clb);

        Self { inner, ..self }
    }

    pub fn build(self) -> ScrollWrapperOption {
        ScrollWrapperOption {
            inner: self.inner,
            class_list: self.class_list,
        }
    }
}
