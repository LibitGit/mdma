use dominator::{Dom, DomBuilder, EventOptions, apply_methods, events::Click};
use futures_signals::signal::{Signal, SignalExt, not};
use proc_macros::{add_class_list, builder, with_class_list};
use web_sys::HtmlDivElement;

use crate::globals::prelude::*;
use crate::interface::tips_parser::tip;

use super::{ClassList, DecorButton, MdmaAddonWindow};

#[add_class_list]
pub(crate) struct CloseButton {
    inner: DomBuilder<HtmlDivElement>,
    hide_window: bool,
}

#[with_class_list]
impl CloseButton {
    pub(crate) fn new() -> Self {
        Self::builder().build()
    }

    pub(crate) fn builder() -> CloseButtonBuilder {
        CloseButtonBuilder::new()
    }

    pub(super) fn render(self, addon_window: &'static AddonWindowDetails) -> Dom {
        use crate::interface::tips_parser::tip;

        let close_button = apply_methods!(self.inner, {
            .disable_dragging()
            .apply_if(self.hide_window, |b| b.event(move |_: Click| {
                addon_window.active.set_neq(false);
            }))
            .tip!({
                .text("Zamknij")
            })
        });

        close_button.into_dom()
    }
}

impl Default for CloseButton {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl DecorButton for CloseButton {
    fn render(self: Box<Self>, window_type: WindowType, addon_data: &'static AddonData) -> Dom {
        (*self).render(addon_data.get(window_type))
    }
}

#[add_class_list]
pub(crate) struct CloseButtonBuilder {
    inner: DomBuilder<HtmlDivElement>,
    hide_window: bool,
}

#[with_class_list(in_builder)]
impl CloseButtonBuilder {
    #[builder(["decor-close-button"])]
    fn new() -> Self {
        Self {
            inner: DomBuilder::new_html("div"),
            hide_window: true,
        }
    }

    /// Whether to hide the current addon window after this `CloseButton` gets clicked.
    ///
    /// For example:
    /// If we want to shake the window instead of closing it if some incorrect data is provided we
    /// first set this to false and then use the `on_click` to implement custom behaviour.
    pub fn hide_window_on_click(self, hide_window: bool) -> Self {
        Self {
            hide_window,
            ..self
        }
    }

    pub(crate) fn on_click<B: FnMut(Click) + 'static>(self, on_click: B) -> Self {
        let inner = self.inner.event(on_click);

        Self { inner, ..self }
    }

    pub(crate) fn build(self) -> CloseButton {
        CloseButton {
            inner: self.inner,
            hide_window: self.hide_window,
            class_list: self.class_list,
        }
    }
}

#[add_class_list]
pub(crate) struct StateBubble {
    inner: DomBuilder<HtmlDivElement>,
}

impl DecorButton for StateBubble {
    fn render(self: Box<Self>, _window_type: WindowType, _addon_data: &'static AddonData) -> Dom {
        (*self).render()
    }
}

#[with_class_list]
impl StateBubble {
    #[builder(["decor-state-bubble"])]
    pub(crate) fn builder() -> Self {
        Self {
            inner: DomBuilder::new_html("div"),
        }
    }

    pub(crate) fn active_signal(self, active_signal: impl Signal<Item = bool> + 'static) -> Self {
        let inner = self.inner.class_signal("active", active_signal);

        Self { inner, ..self }
    }

    pub(crate) fn on_click<B: FnMut(Click) + 'static>(self, on_click: B) -> Self {
        let inner = self
            .inner
            .event_with_options(&EventOptions::preventable(), on_click);

        Self { inner, ..self }
    }

    pub(crate) fn mixin<B>(self, mixin: B) -> Self
    where
        B: FnOnce(DomBuilder<HtmlDivElement>) -> DomBuilder<HtmlDivElement>,
    {
        let inner = self.inner.apply(mixin);
        Self { inner, ..self }
    }

    pub(super) fn render(self) -> Dom {
        let counter = apply_methods!(self.inner, {
            .disable_dragging()
        });

        counter.into_dom()
    }
}

#[add_class_list]
pub(crate) struct CollapseButton {
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl CollapseButton {
    #[builder(["decor-collapse-button"])]
    pub(crate) fn new() -> Self {
        Self {
            inner: DomBuilder::new_html("div"),
        }
    }

    pub(super) fn render(self, addon_window_data: &'static AddonWindowDetails) -> Dom {
        let counter = apply_methods!(self.inner, {
            .class_signal("collapsed", not(addon_window_data.expanded.signal()))
            .event(|_: Click| addon_window_data.expanded.set(!addon_window_data.expanded.get()))
            .tip!({
                .text_signal(addon_window_data.expanded.signal_ref(|expanded| match expanded {
                    true => "Zwiń",
                    false => "Rozwiń",
                }))
            })
            .disable_dragging()
        });

        counter.into_dom()
    }
}

impl DecorButton for CollapseButton {
    fn render(self: Box<Self>, window_type: WindowType, addon_data: &'static AddonData) -> Dom {
        (*self).render(addon_data.get(window_type))
    }
}

#[add_class_list]
pub(crate) struct OpacityToggle {
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl OpacityToggle {
    #[builder(["decor-opacity-toggle"])]
    pub(crate) fn new() -> Self {
        Self {
            inner: DomBuilder::new_html("div"),
        }
    }

    pub(super) fn render(self, addon_window_data: &'static AddonWindowDetails) -> Dom {
        let counter = apply_methods!(self.inner, {
            .tip!({
                .text("Zmień przezroczystość")
            })
            .event(|_: Click| {
                addon_window_data.opacity_lvl.replace_with(|old| (*old + 1) % 6);
            })
            .disable_dragging()
        });

        counter.into_dom()
    }
}

impl DecorButton for OpacityToggle {
    fn render(self: Box<Self>, window_type: WindowType, addon_data: &'static AddonData) -> Dom {
        (*self).render(addon_data.get(window_type))
    }
}

#[add_class_list]
pub(crate) struct SizeToggle {
    /// <0; u8::MAX - 1>
    max_size: u8,
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl SizeToggle {
    #[builder(["decor-size-toggle"])]
    pub(crate) fn new(max_size: u8) -> Self {
        Self {
            max_size,
            inner: DomBuilder::new_html("div"),
        }
    }

    pub(super) fn render(self, addon_window_data: &'static AddonWindowDetails) -> Dom {
        let counter = apply_methods!(self.inner, {
            .tip!({
                .text("Zmień rozmiar")
            })
            .event(move |_: Click| {
                addon_window_data.size.replace_with(|old| (*old + 1) % (self.max_size + 1));
            })
            .disable_dragging()
        });

        counter.into_dom()
    }
}

impl DecorButton for SizeToggle {
    fn render(self: Box<Self>, window_type: WindowType, addon_data: &'static AddonData) -> Dom {
        (*self).render(addon_data.get(window_type))
    }
}

#[add_class_list]
pub(crate) struct CounterBubble {
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list]
impl CounterBubble {
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub(crate) fn builder() -> CounterBubbleBuilder {
        CounterBubbleBuilder::new()
    }

    pub(super) fn render(self) -> Dom {
        let counter = apply_methods!(self.inner, {
            .disable_dragging()
        });

        counter.into_dom()
    }
}

impl DecorButton for CounterBubble {
    fn render(self: Box<Self>, _window_type: WindowType, _addon_data: &'static AddonData) -> Dom {
        (*self).render()
    }
}

#[add_class_list]
pub(crate) struct CounterBubbleBuilder {
    inner: DomBuilder<HtmlDivElement>,
}

#[with_class_list(in_builder)]
impl CounterBubbleBuilder {
    #[builder(["decor-counter-bubble"])]
    pub(crate) fn new() -> Self {
        Self {
            inner: DomBuilder::new_html("div"),
        }
    }

    pub fn counter_signal(self, signal: impl Signal<Item = usize> + 'static) -> Self {
        let inner = self.inner.text_signal(signal.map(|c| format!("({c})")));

        Self { inner, ..self }
    }

    pub(crate) fn mixin<B>(self, mixin: B) -> Self
    where
        B: FnOnce(DomBuilder<HtmlDivElement>) -> DomBuilder<HtmlDivElement>,
    {
        let inner = self.inner.apply(mixin);

        Self { inner, ..self }
    }

    pub fn build(self) -> CounterBubble {
        CounterBubble {
            inner: self.inner,
            class_list: self.class_list,
        }
    }
}

#[add_class_list]
pub(crate) struct SettingsButton {
    inner: DomBuilder<HtmlDivElement>,
    hide_window: bool,
}

#[with_class_list]
impl SettingsButton {
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub(crate) fn builder() -> SettingsButtonBuilder {
        SettingsButtonBuilder::new()
    }

    pub(super) fn render(self, addon_window: &'static AddonWindowDetails) -> Dom {
        let settings_button = apply_methods!(self.inner, {
            .disable_dragging()
            .apply_if(self.hide_window, |b| b.event(move |_: Click| {
                //common::debug_log!("SETTING TO:", !addon_window.active.get());
                addon_window.active.set(!addon_window.active.get());
            }))
            .tip!({
                .text_signal(addon_window.active.signal_ref(|active| match *active {
                    true => "Zamknij ustawienia",
                    false => "Otwórz ustawienia",
                }))
            })
        });

        settings_button.into_dom()
    }
}

impl Default for SettingsButton {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl DecorButton for SettingsButton {
    fn render(self: Box<Self>, _window_type: WindowType, addon_data: &'static AddonData) -> Dom {
        (*self).render(addon_data.get(WindowType::SettingsWindow))
    }
}

#[add_class_list]
pub(crate) struct SettingsButtonBuilder {
    inner: DomBuilder<HtmlDivElement>,
    hide_window: bool,
}

#[with_class_list(in_builder)]
impl SettingsButtonBuilder {
    #[builder(["decor-settings-button"])]
    fn new() -> Self {
        Self {
            inner: DomBuilder::new_html("div"),
            hide_window: true,
        }
    }

    pub(crate) fn on_click<B: FnMut(Click) + 'static>(self, on_click: B) -> Self {
        let inner = self
            .inner
            .event_with_options(&EventOptions::preventable(), on_click);

        Self { inner, ..self }
    }

    pub(crate) fn mixin<B>(self, mixin: B) -> Self
    where
        B: FnOnce(DomBuilder<HtmlDivElement>) -> DomBuilder<HtmlDivElement>,
    {
        let inner = self.inner.apply(mixin);
        Self { inner, ..self }
    }

    /// Whether to hide the current addon window after this `CloseButton` gets clicked.
    ///
    /// For example:
    /// If we want to shake the window instead of closing it if some incorrect data is provided we
    /// first set this to false and then use the `on_click` to implement custom behaviour.
    pub fn hide_window_on_click(self, hide_window: bool) -> Self {
        Self {
            hide_window,
            ..self
        }
    }

    pub(crate) fn build(self) -> SettingsButton {
        SettingsButton {
            inner: self.inner,
            hide_window: self.hide_window,
            class_list: self.class_list,
        }
    }
}
