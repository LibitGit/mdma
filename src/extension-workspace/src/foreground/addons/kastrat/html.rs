use std::iter;
use std::ops::Deref;

use common::err_code;
use dominator::events::KeyDown;
use dominator::{apply_methods, clone, html, Dom, EventOptions};
use futures_signals::map_ref;
use futures_signals::signal::SignalExt;
use wasm_bindgen::intern;

use crate::addon_window::prelude::*;
use crate::bindings::engine::hero::AutoGoToData;
use crate::interface::tips_parser::tip;
use crate::interface::{ThreadLocalShadowRoot, ALLOWED_CHARS, WINDOWS_ROOT};
use crate::prelude::*;

use super::{get_engine, message, ActiveSettings, HotkeyValue, Settings, ADDON_NAME};

impl ActiveSettings {
    fn render(&'static self, settings: &'static Settings) -> JsResult<Dom> {
        let window_size =
            Addons::get_window_size(ADDON_NAME, Self::WINDOW_TYPE).ok_or_else(|| err_code!())?;
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_left(decors::SettingsButton::new())
            .push_left(self.build_state_bubble())
            .push_right(decors::CloseButton::new())
            .push_right(decors::SizeToggle::new(1))
            .push_right(decors::CollapseButton::new())
            .build();
        let header = WindowHeader::new(decor);

        let content = WindowContent::builder()
            .class_list("g[5]")
            .input_pair(
                InputPair::builder(
                    Input::builder()
                        .placeholder_signal(self.lvl.min.signal_ref(|min| min.to_string()))
                        .maxlength(s!("3"))
                        .text_signal(|| {
                            window_size.signal_ref(|size| match size {
                                0 => "",
                                _ => "Min",
                            })
                        })
                        .input_type(InputType::number(1.0, 500.0))
                        .on_input(move |event, input| {
                            let value = input.value_as_number();

                            self.lvl.min.set_neq(value as u16);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                    Input::builder()
                        .placeholder_signal(self.lvl.max.signal_ref(|max| max.to_string()))
                        .maxlength("3")
                        .text_signal(|| {
                            window_size.signal_ref(|size| match size {
                                0 => "",
                                _ => "Max",
                            })
                        })
                        .input_type(InputType::number(1.0, 500.0))
                        .on_input(move |event, input| {
                            let value = input.value_as_number();

                            self.lvl.max.set_neq(value as u16);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                )
                .class_list("j-c[space-around]"),
            )
            .section(self.target_setting(settings)?);

        AddonWindow::builder(ADDON_NAME)
            .header(header)
            .content(content)
            .build()
    }

    fn build_state_bubble(&'static self) -> decors::StateBubble {
        decors::StateBubble::builder()
            .active_signal(self.attack_toggle.signal())
            .on_click(|_| {
                common::debug_log!("new attack toggle:", !self.attack_toggle.get());
                self.attack_toggle.set(!self.attack_toggle.get())
            })
            .mixin(|b| {
                apply_methods!(b, {
                    .tip!({
                        .text_signal(self.attack_toggle.signal_ref(|active| match active {
                            true => "Wyłącz atakowanie",
                            false => "Włącz atakowanie",
                        }))
                    })
                })
            })
    }

    fn target_setting(&'static self, settings: &Settings) -> JsResult<ContentSection> {
        let button_text_signal = Addons::get_window_size(ADDON_NAME, Self::WINDOW_TYPE)
            .ok_or_else(|| err_code!())?
            .signal()
            .map(|window_size| match window_size {
                0 => "Podejdź",
                _ => "Podejdź do celu",
            });
        let target_button = Button::builder()
            .class_list("w[120] overflow[hidden] t-o[ellipsis] w-s[nowrap]")
            .disabled_signal(self.target.signal_ref(|t| t.is_none()).dedupe())
            .text_signal(button_text_signal)
            .on_click(move |_| {
                let target_lock = self.target.lock_ref();
                let Some(target) = target_lock.as_ref() else {
                    return;
                };

                if let Some(target_x) = target.x.get()
                    && let Some(target_y) = target.y.get()
                {
                    let dest = AutoGoToData::new(target_x, target_y);
                    let _ = message(&format!(
                        "[MDMA::RS] Podchodzę do \"{}\"...",
                        target.nick.lock_ref().deref()
                    ));

                    get_engine()
                        .hero()
                        .unwrap_js()
                        .auto_go_to(&dest)
                        .unwrap_js();
                }
            });

        Ok(ContentSection::new()
            .visible_signal(settings.track_button.signal())
            .class_list("d[flex] f-d[row] j-c[center] a-i[center]")
            .button(target_button))
    }
}

impl Settings {
    fn render(&'static self, active_settings: &'static ActiveSettings) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_right(decors::CloseButton::new())
            .build();
        let header = WindowHeader::new(decor);
        let window_size = Addons::get_window_size(ADDON_NAME, ActiveSettings::WINDOW_TYPE)
            .ok_or_else(|| err_code!())?;
        let content = WindowContent::builder()
            .heading(
                Heading::builder()
                    .class_list("first-heading")
                    .text("Ustawienia ogólne"),
            )
            .section(
                ContentSection::new()
                    .checkbox(
                        Checkbox::builder(self.track_button.clone()).label_mixin(|builder| {
                            builder.text("Wyświetlaj przycisk ").child(html!("b", {
                                .text_signal(
                                    window_size.signal()
                                        .map(|window_size| match window_size {
                                            0 => "Podejdź",
                                            _ => "Podejdź do celu",
                                        })
                                )
                            }))
                        }),
                    )
                    .checkbox(Checkbox::builder(self.msg.clone()).text(s!("Wiadomość o ataku")))
                    .checkbox(
                        Checkbox::builder(self.wanted_targetting.clone())
                            .class_list("w-s[pre-line] l-h[16]")
                            .text("Atakuj poszukiwanych na\nmapach z warunkowym PvP"),
                    ),
            )
            .heading(Heading::builder().text("Skróty klawiszowe"))
            .section(self.hotkey_setting())
            .global_event_with_options(&EventOptions::preventable(), move |event: KeyDown| {
                self.init(active_settings, event)
            });

        SettingsWindow::builder(ADDON_NAME)
            .header(header)
            .content(content)
            .build()
    }
    fn hotkey_setting(&'static self) -> ContentSection {
        let attack_toggle_checkbox = Checkbox::builder(self.attack_toggle_hotkey.active.clone())
            .class_list("w-s[pre-line] l-h[16]")
            .text("Klawisz do przełączania\nautomatycznego ataku");

        let hotkey_lock = self.attack_toggle_hotkey.value.lock_ref();
        let display_value = hotkey_lock
            .ctrl_key
            .then_some("Ctrl")
            .into_iter()
            .chain(hotkey_lock.alt_key.then_some("Alt"))
            .chain(hotkey_lock.shift_key.then_some("Shift"))
            .chain(iter::once(hotkey_lock.value.as_str()))
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(" + ");
        drop(hotkey_lock);

        let attack_toggle_input = Input::builder()
            .input_type(InputType::keybind())
            .value(display_value)
            .maxlength("1")
            .size(InputSize::Custom("w[100]"))
            .on_key_down(move |event, input_elem| {
                if event.repeat() {
                    return;
                }

                let value = event.key();
                let mut chars = value.chars();

                if chars
                    .next()
                    .is_none_or(|c| !is_from_polish_alphabet(c) && !ALLOWED_CHARS.contains(c))
                {
                    event.prevent_default();
                    event.stop_propagation();
                    input_elem.blur().unwrap_js();
                    return;
                }
                if chars.next().is_some() {
                    event.prevent_default();
                    event.stop_propagation();

                    if value != intern("Tab") {
                        if value == intern("Escape")
                            || !matches!(value.as_str(), "Control" | "Alt" | "Shift")
                        {
                            input_elem.blur().unwrap_js();
                        }
                        return;
                    }
                }

                let value = value
                    .chars()
                    .map(polish_to_ascii)
                    .collect::<String>()
                    .to_ascii_uppercase();
                let mut new_hotkey = HotkeyValue::default();
                let display_value = event
                    .ctrl_key()
                    .then(|| {
                        new_hotkey.ctrl_key = true;
                        "Ctrl"
                    })
                    .into_iter()
                    .chain(event.alt_key().then(|| {
                        new_hotkey.alt_key = true;
                        "Alt"
                    }))
                    .chain(event.shift_key().then(|| {
                        new_hotkey.shift_key = true;
                        "Shift"
                    }))
                    .chain(iter::once(value.as_str()))
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(" + ");

                event.prevent_default();
                event.stop_propagation();
                input_elem.blur().unwrap_js();
                input_elem.set_value(&display_value);
                new_hotkey.value = value;
                self.attack_toggle_hotkey.value.set(new_hotkey);
            });
        let attack_toggle_section = ContentSection::new()
            .class_list("d[flex] f-d[row] g[10] a-i[center]")
            .checkbox(attack_toggle_checkbox)
            .input(attack_toggle_input);

        let track_checkbox = Checkbox::builder(self.track_hotkey.active.clone())
            .class_list("w-s[pre-line] l-h[16]")
            .text("Klawisz od podchodzenia\ndo celu")
            .on_click(|_| {
                self.track_hotkey
                    .active
                    .set(!self.track_hotkey.active.get())
            });

        let hotkey_lock = self.track_hotkey.value.lock_ref();
        let display_value = hotkey_lock
            .ctrl_key
            .then_some("Ctrl")
            .into_iter()
            .chain(hotkey_lock.alt_key.then_some("Alt"))
            .chain(hotkey_lock.shift_key.then_some("Shift"))
            .chain(iter::once(hotkey_lock.value.as_str()))
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(" + ");
        drop(hotkey_lock);

        let track_input = Input::builder()
            .input_type(InputType::keybind())
            .value(display_value)
            .maxlength("1")
            .size(InputSize::Custom("w[100]"))
            .on_key_down(move |event, input_elem| {
                if event.repeat() {
                    return;
                }

                let value = event.key();
                let mut chars = value.chars();

                if chars
                    .next()
                    .is_none_or(|c| !is_from_polish_alphabet(c) && !ALLOWED_CHARS.contains(c))
                {
                    event.prevent_default();
                    event.stop_propagation();
                    input_elem.blur().unwrap_js();
                    return;
                }
                if chars.next().is_some() {
                    event.prevent_default();
                    event.stop_propagation();

                    if value != intern("Tab") {
                        if value == intern("Escape")
                            || !matches!(value.as_str(), "Control" | "Alt" | "Shift")
                        {
                            input_elem.blur().unwrap_js();
                        }
                        return;
                    }
                }

                let value = value
                    .chars()
                    .map(polish_to_ascii)
                    .collect::<String>()
                    .to_ascii_uppercase();
                let mut new_hotkey = HotkeyValue::default();
                let display_value = event
                    .ctrl_key()
                    .then(|| {
                        new_hotkey.ctrl_key = true;
                        "Ctrl"
                    })
                    .into_iter()
                    .chain(event.alt_key().then(|| {
                        new_hotkey.alt_key = true;
                        "Alt"
                    }))
                    .chain(event.shift_key().then(|| {
                        new_hotkey.shift_key = true;
                        "Shift"
                    }))
                    .chain(iter::once(value.as_str()))
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(" + ");

                event.prevent_default();
                event.stop_propagation();
                input_elem.blur().unwrap_js();
                input_elem.set_value(&display_value);
                new_hotkey.value = value;
                self.track_hotkey.value.set(new_hotkey);
            });
        let track_hotkey_section = ContentSection::new()
            .class_list("d[flex] f-d[row] j-c[space-between] a-i[center]")
            .checkbox(track_checkbox)
            .input(track_input);

        ContentSection::new()
            .class_list("d[flex] f-d[column] g[6]")
            .section(attack_toggle_section)
            .section(track_hotkey_section)
    }
}

pub(super) fn init(
    settings: &'static Settings,
    active_settings: &'static ActiveSettings,
) -> JsResult<()> {
    let _addon_window_handle = WINDOWS_ROOT
        .try_append_dom(active_settings.render(settings)?)
        .ok_or_else(|| err_code!())?;

    let _settings_window_handle = WINDOWS_ROOT
        .try_append_dom(settings.render(active_settings)?)
        .ok_or_else(|| err_code!())?;
    Ok(())
}
