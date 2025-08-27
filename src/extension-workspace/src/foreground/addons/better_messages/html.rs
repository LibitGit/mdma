use std::ops::Deref;

use dominator::events::Input as InputEvent;
use dominator::{clone, stylesheet, Dom};
use futures_signals::map_ref;
use futures_signals::signal::SignalExt;
use web_sys::HtmlInputElement;

use crate::addon_window::prelude::*;
use crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT};
use crate::prelude::*;

use super::*;

impl WindowContent {
    fn font_size_setting(self, font: &Font) -> Self {
        let Font {
            size,
            active: checked,
        } = font;
        let font_size_checkbox = Checkbox::builder(checked.clone()).text("Rozmiar tekstu");
        let font_size_input = Input::builder()
            .value(font.size.get().to_string())
            .input_type(InputType::number(0.0, 999.0))
            .maxlength("3")
            .on_input(
                clone!(size => move |event: InputEvent, elem: &HtmlInputElement| {
                    event.prevent_default();
                    event.stop_immediate_propagation();

                    if elem.value().is_empty() {
                        return;
                    }
                    if elem.value().chars().any(|c| !c.is_ascii_digit()) {
                        return;
                    }

                    match elem.value().parse::<u16>() {
                        Ok(value) => size.set(value),
                        _ => console_error!()
                    }
                }),
            );
        let font_size_section = ContentSection::new()
            .class_list("label j-c[space-between]")
            .checkbox(font_size_checkbox)
            .input(font_size_input);

        self.section(font_size_section)
    }

    fn color_setting(self, color: &Color) -> Self {
        let Color {
            code,
            active: checked,
        } = color;
        let color_checkbox = Checkbox::builder(checked.clone()).text("Kolor tekstu");
        let color_input = Input::builder()
            .input_type(InputType::color())
            .size(InputSize::Color)
            .value(code.lock_ref().deref())
            .on_input(clone!(code => move |_, input_elem: &HtmlInputElement| {
                code.set(input_elem.value());
            }));
        let color_section = ContentSection::new()
            .class_list("label j-c[space-between]")
            .checkbox(color_checkbox)
            .input(color_input);

        self.section(color_section)
    }

    fn pointer_events_setting(self, pointer_events: &Mutable<bool>) -> Self {
        let pointer_events_checkbox =
            Checkbox::builder(pointer_events.clone()).text("Interakcja z tekstem");
        let pointer_events_section = ContentSection::new()
            .class_list("label j-c[space-between]")
            .checkbox(pointer_events_checkbox);

        self.section(pointer_events_section)
    }

    fn test_button(self, active_settings: &'static ActiveSettings) -> Self {
        let test_button = Button::builder()
            .class_list("w[80]")
            .text_signal(
                active_settings
                    .testing
                    .active
                    .signal_ref(|&active| match active {
                        true => "Stop test",
                        false => "Test",
                    }),
            )
            .on_click(move |_| {
                match active_settings
                    .testing
                    .active
                    .replace_with(|active| !*active)
                {
                    false => {
                        if let Err(err) = ActiveSettings::init_tests(&active_settings) {
                            console_error!(err);
                        }
                    }
                    true if active_settings.testing.interval_id.get().is_none() => {
                        console_error!()
                    }
                    true => {
                        if let Some(interval_id) = active_settings.testing.interval_id.get() {
                            window().clear_interval_with_handle(interval_id);
                            active_settings.testing.interval_id.set(None);
                            active_settings.testing.active.set(false);
                        }
                    }
                }
            });
        let test_section = ContentSection::new()
            .class_list("label j-c[center]")
            .button(test_button);

        self.section(test_section)
    }
}

impl ActiveSettings {
    pub(super) fn render(&'static self) -> JsResult<Dom> {
        let close_button = decors::CloseButton::builder()
            .on_click(move |_event: ::dominator::events::Click| {
                if let Some(interval_id) = self.testing.interval_id.get() {
                    window().clear_interval_with_handle(interval_id);
                    self.testing.interval_id.set(None);
                    self.testing.active.set(false);
                }
            })
            .build();
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_right(close_button)
            .push_right(decors::CollapseButton::new())
            .build();
        let addon_window_header = WindowHeader::new(decor);

        let addon_window_content = WindowContent::builder()
            .heading(
                Heading::builder()
                    .text("Ustawienia tekstu")
                    .class_list("m[0]"),
            )
            .font_size_setting(&self.font)
            .color_setting(&self.color)
            .pointer_events_setting(&self.pointer_events)
            .test_button(self);

        AddonWindow::builder(AddonName::BetterMessages)
            .header(addon_window_header)
            .content(addon_window_content)
            .build()
    }
}

pub(super) fn init(active_settings: &'static ActiveSettings) -> JsResult<()> {
    let addon_data = Addons::get_addon(ADDON_NAME).ok_or_else(|| err_code!())?;
    let font_size_signal = map_ref! {
        let addon_active = addon_data.active.signal(),
        let font_size_active = active_settings.font.active.signal(),
        let font_size = active_settings.font.size.signal() => {
            match *addon_active && *font_size_active {
                false => None,
                true => Some(*font_size),
            }
        }
    };
    let pointer_events_signal = map_ref! {
        let addon_active = addon_data.active.signal(),
        let pointer_events = active_settings.pointer_events.signal() => {
            match *addon_active {
                false => None,
                true => match *pointer_events {
                    true => None,
                    false => Some("none"),
                }
            }
        }
    };
    let color_signal = addon_data
        .active
        .signal()
        .switch(|window_active| {
            active_settings
                .color
                .active
                .signal()
                .map(move |color_active| color_active && window_active)
        })
        .switch(|display_color| {
            active_settings
                .color
                .code
                .signal_cloned()
                .map(move |color_code| display_color.then_some(color_code))
        });

    #[cfg(feature = "ni")]
    {
        stylesheet!(s!(".message .inner"), {
            .style_important_signal("font-size", font_size_signal.map(|size| {
                size.map(|size| format!("{}{}", size, s!("px")))
            }))
            .style_important_signal("color", color_signal)
        });
        stylesheet!(s!(".message"), {
            .style_important_signal("pointer-events" , pointer_events_signal)
        });
    }

    #[cfg(not(feature = "ni"))]
    {
        stylesheet!(s!("#msg > div"), {
            .style_important_signal("font-size", font_size_signal.map(|size| {
                size.map(|size| format!("{}{}", size, s!("px")))
            }))
            .style_important_signal("color", color_signal)
        });
        stylesheet!(s!("#msg"), {
            .style_important_signal("pointer-events" , pointer_events_signal)
        });
    }

    //TODO: Remove from DOM if hidden?
    let _handle = WINDOWS_ROOT
        .try_append_dom(active_settings.render()?)
        .ok_or_else(|| err_code!())?;

    Ok(())
}
