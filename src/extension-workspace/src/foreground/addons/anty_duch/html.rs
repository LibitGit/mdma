use dominator::{Dom, apply_methods, html};
use futures_signals::signal::Mutable;

use crate::addon_window::prelude::*;
use crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT, tips_parser::tip};
use crate::prelude::*;

use super::{ADDON_NAME, ActiveSettings};

impl ActiveSettings {
    fn render(&'static self) -> JsResult<Dom> {
        let window_size =
            Addons::get_window_size(ADDON_NAME, Self::WINDOW_TYPE).ok_or_else(|| err_code!())?;
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_left(self.build_attack_state_bubble())
            .push_right(decors::CloseButton::new())
            .push_right(decors::SizeToggle::new(1))
            .push_right(decors::CollapseButton::new())
            .build();
        let window_header = WindowHeader::new(decor);

        let window_content = WindowContent::builder()
            .class_list("f-d[column]")
            .section(self.anti_afk_settings(window_size))
            .heading(Heading::builder().class_list("m[0]").text("Kastrat"))
            .section(self.kastrat_settings(window_size))
            .heading(Heading::builder().class_list("m[0]").text("Berserker"))
            .section(self.berserker_settings(window_size));

        AddonWindow::builder(ADDON_NAME)
            .header(window_header)
            .content(window_content)
            .build()
    }

    fn build_attack_state_bubble(&'static self) -> decors::StateBubble {
        decors::StateBubble::builder()
            .active_signal(self.kastrat.attack_toggle.signal())
            .on_click(|_| {
                common::debug_log!("new attack toggle:", !self.kastrat.attack_toggle.get());
                self.kastrat
                    .attack_toggle
                    .set(!self.kastrat.attack_toggle.get())
            })
            .mixin(|b| {
                apply_methods!(b, {
                    .tip!({
                        .text_signal(self.kastrat.attack_toggle.signal_ref(|active| match active {
                            true => "Wyłącz atakowanie graczy",
                            false => "Włącz atakowanie graczy",
                        }))
                    })
                })
            })
    }

    fn anti_afk_settings(&'static self, window_size: &'static Mutable<u8>) -> ContentSection {
        ContentSection::new()
            .input_pair(
                InputPair::builder(
                    Input::builder()
                        .value_signal(self.anti_afk.return_pos.x.signal_ref(|x| x.to_string()))
                        .maxlength(s!("3"))
                        .text_signal(|| {
                            window_size.signal_ref(|size| match size {
                                0 => "",
                                _ => "X",
                            })
                        })
                        .input_type(InputType::number(0.0, 500.0))
                        .on_input(move |event, input| {
                            let value = input.value_as_number();

                            self.anti_afk.return_pos.x.set_neq(value as u16);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                    Input::builder()
                        .value_signal(self.anti_afk.return_pos.y.signal_ref(|y| y.to_string()))
                        .maxlength("3")
                        .text_signal(|| {
                            window_size.signal_ref(|size| match size {
                                0 => "",
                                _ => "Y",
                            })
                        })
                        .input_type(InputType::number(0.0, 500.0))
                        .on_input(move |event, input| {
                            let value = input.value_as_number();

                            self.anti_afk.return_pos.y.set_neq(value as u16);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                )
                .class_list("j-c[space-around]"),
            )
            .checkbox(
                Checkbox::builder(self.anti_afk.return_active.clone())
                    .class_list("p-top[6]")
                    .text("Wracanie na koordy"),
            )
    }

    fn kastrat_settings(&'static self, window_size: &'static Mutable<u8>) -> ContentSection {
        ContentSection::new()
            .input_pair(
                InputPair::builder(
                    Input::builder()
                        .value_signal(self.kastrat.lvl.min.signal_ref(|min| min.to_string()))
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

                            self.kastrat.lvl.min.set_neq(value as u16);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                    Input::builder()
                        .value_signal(self.kastrat.lvl.max.signal_ref(|max| max.to_string()))
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

                            self.kastrat.lvl.max.set_neq(value as u16);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                )
                .class_list("j-c[space-around] p-top[6]"),
            )
            .input_pair(
                InputPair::builder(
                    Input::builder()
                        .value_signal(self.kastrat.return_pos.x.signal_ref(|x| x.to_string()))
                        .maxlength(s!("3"))
                        .text_signal(|| {
                            window_size.signal_ref(|size| match size {
                                0 => "",
                                _ => "X",
                            })
                        })
                        .input_type(InputType::number(0.0, 500.0))
                        .on_input(move |event, input| {
                            let value = input.value_as_number();

                            self.kastrat.return_pos.x.set_neq(value as u16);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                    Input::builder()
                        .value_signal(self.kastrat.return_pos.y.signal_ref(|y| y.to_string()))
                        .maxlength("3")
                        .text_signal(|| {
                            window_size.signal_ref(|size| match size {
                                0 => "",
                                _ => "Y",
                            })
                        })
                        .input_type(InputType::number(0.0, 500.0))
                        .on_input(move |event, input| {
                            let value = input.value_as_number();

                            self.kastrat.return_pos.y.set_neq(value as u16);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                )
                .class_list("j-c[space-around] p-top[6]"),
            )
            .checkbox(
                Checkbox::builder(self.kastrat.return_active.clone())
                    .class_list("p-top[6]")
                    .text("Wracanie na koordy"),
            )
    }

    fn berserker_settings(&'static self, window_size: &'static Mutable<u8>) -> ContentSection {
        ContentSection::new()
            .input_pair(
                InputPair::builder(
                    Input::builder()
                        .value_signal(self.berserker.delay.min.signal_ref(|min| min.to_string()))
                        .maxlength(s!("5"))
                        .text_signal(|| {
                            window_size.signal_ref(|size| match size {
                                0 => "",
                                _ => "Min",
                            })
                        })
                        .input_type(InputType::number(1.0, 99999.0))
                        .on_input(move |event, input| {
                            let value = input.value_as_number();

                            self.berserker.delay.min.set_neq(value as usize);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                    Input::builder()
                        .value_signal(self.berserker.delay.max.signal_ref(|max| max.to_string()))
                        .maxlength("5")
                        .text_signal(|| {
                            window_size.signal_ref(|size| match size {
                                0 => "",
                                _ => "Max",
                            })
                        })
                        .input_type(InputType::number(2.0, 99999.0))
                        .on_input(move |event, input| {
                            let value = input.value_as_number();

                            self.berserker.delay.max.set_neq(value as usize);
                            event.stop_propagation();
                            event.stop_immediate_propagation();
                        }),
                )
                .class_list("j-c[space-around] p-top[6]"),
            )
            .input(
                Input::builder()
                    .class_list("m-top[6]")
                    .value_signal(self.berserker.npc_name.signal_cloned())
                    .confirm_button(InputButton::builder().class_list("m-top[6]").on_click(
                        move |_, input| {
                            let value = input.value();

                            self.berserker.npc_name.set(value);
                        },
                    ))
                    .size(InputSize::Custom("w[108]")),
            )
            .checkbox(
                Checkbox::builder(self.berserker.fast_fight.clone())
                    .class_list("p-top[6]")
                    .text("Szybka walka"),
            )
    }
}

pub(super) fn init(active_settings: &'static ActiveSettings) -> JsResult<()> {
    let _active_settings_window_handle = WINDOWS_ROOT
        .try_append_dom(active_settings.render()?)
        .ok_or_else(|| err_code!())?;

    Ok(())
}
