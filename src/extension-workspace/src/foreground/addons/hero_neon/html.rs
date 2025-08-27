use std::ops::Deref;

use dominator::events::{ContextMenu, MouseButton};
use dominator::{Dom, apply_methods, html, with_node};
use futures_signals::signal::{self, SignalExt};

use crate::addon_window::prelude::*;
use crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT, tips_parser::tip};
use crate::prelude::*;

use super::{ADDON_NAME, ActiveSettings, DEFAULT_OFFSET, DEFAULT_RADIUS, DEFAULT_ROTATION_STEP};

trait WindowContentExt {
    fn size_setting(self, active_settings: &'static ActiveSettings) -> Self;
    fn offset_setting(self, active_settings: &'static ActiveSettings) -> Self;
    fn color_setting(self, active_settings: &'static ActiveSettings) -> Self;
    fn mode_setting(self, active_settings: &'static ActiveSettings) -> Self;
    fn interpolation_speed_setting(self, active_settings: &'static ActiveSettings) -> Self;
}

impl WindowContentExt for WindowContent {
    fn size_setting(self, active_settings: &'static ActiveSettings) -> Self {
        let size_input = Input::builder()
            .class_list("w[150]")
            .input_type(InputType::slider(0.0, 100.0))
            .mixin(|b, _| {
                apply_methods!(b, {
                    .style_signal(
                        "background",
                        active_settings.radius
                            .signal()
                            .map(|percentage| {
                                format!("linear-gradient(to right, rgb(57, 107, 41) {percentage:.2}%, rgb(12, 13, 13) {percentage:.2}%)")
                            })
                    )
                    .tip!({
                        .text_signal(active_settings.radius.signal().map(|size| format!("Rozmiar neonu: {size}px\n\nPPM aby przywrócić do wartości domyślnej")))
                    })
                    .with_node!(input_elem => {
                        .event(move |event: ContextMenu| {
                            debug_log!(@f "{event:?}");
                            if event.button() == MouseButton::Right {
                                input_elem.set_value(&DEFAULT_RADIUS.to_string());
                                active_settings.radius.set(DEFAULT_RADIUS);
                            }
                        })
                    })
                })
            })
            .value(active_settings.radius.get().to_string())
            .on_input(|_event, input| {
                let value = input.value_as_number();
                active_settings.radius.set(value.clamp(0.0, 100.0));
            });
        let size_slider_setting = ContentSection::new()
            .class_list("d[flex] f-d[row] a-i[center] j-c[space-between]")
            .section(ContentSection::new().text("Rozmiar"))
            .input(size_input);

        self.section(
            ContentSection::new()
                .class_list("d[flex] f-d[column]")
                .section(size_slider_setting),
        )
    }

    fn offset_setting(self, active_settings: &'static ActiveSettings) -> Self {
        let offset_input = Input::builder()
            .class_list("w[150]")
            .input_type(InputType::slider(0.0, 100.0))
            .mixin(|b, _| {
                apply_methods!(b, {
                    .style_signal(
                        "background",
                        active_settings.offset
                            .signal()
                            .map(|percentage| {
                                let percentage = percentage * 100.0;
                                format!("linear-gradient(to right, rgb(57, 107, 41) {percentage:.2}%, rgb(12, 13, 13) {percentage:.2}%)")
                            })
                    )
                    .tip!({
                        .text_signal(active_settings.offset.signal().map(|offset| format!("Zanikanie neonu od: {:.0}%\n\nPPM aby przywrócić do wartości domyślnej", offset * 100.0)))
                    })
                    .with_node!(input_elem => {
                        .event(move |event: ContextMenu| {
                            debug_log!(@f "{event:?}");
                            if event.button() == MouseButton::Right {
                                input_elem.set_value(&(DEFAULT_OFFSET * 100.0).to_string());
                                active_settings.offset.set(DEFAULT_OFFSET);
                            }
                        })
                    })
                })
            })
            .value((active_settings.offset.get() * 100.0).to_string())
            .on_input(|_event, input| {
                let value = input.value_as_number();
                active_settings.offset.set((value / 100.0).clamp(0.0, 1.0) as f32);
            });
        let offset_slider_setting = ContentSection::new()
            .class_list("d[flex] f-d[row] a-i[center] j-c[space-between] p-top[6] g[5]")
            .section(ContentSection::new().text("Zanikanie od"))
            .input(offset_input);

        self.section(
            ContentSection::new()
                .class_list("d[flex] f-d[column]")
                .section(offset_slider_setting),
        )
    }

    fn color_setting(self, active_settings: &'static ActiveSettings) -> Self {
        let start_color_input = Input::builder()
            .input_type(InputType::color())
            .size(InputSize::Color)
            .value(active_settings.start_color.lock_ref().deref())
            .on_input(move |_, input_elem| {
                active_settings.start_color.set_neq(input_elem.value());
            });
        let start_color_section = ContentSection::new()
            .class_list("d[flex] f-d[row] a-i[center] j-c[space-between] p-top[6]")
            .section(ContentSection::new().text_signal(
                active_settings.interpolate.signal().dedupe().map(
                    |interpolate| match interpolate {
                        true => "Kolor 1",
                        false => "Kolor",
                    },
                ),
            ))
            .input(start_color_input);

        let end_color_section_signal =
            active_settings
                .interpolate
                .signal()
                .dedupe()
                .map(move |interpolate| {
                    interpolate.then(|| {
                        let end_color_input = Input::builder()
                            .input_type(InputType::color())
                            .size(InputSize::Color)
                            .value(active_settings.end_color.lock_ref().deref())
                            .on_input(move |_, input_elem| {
                                active_settings.end_color.set_neq(input_elem.value());
                            });

                        ContentSection::new()
                            .class_list("d[flex] f-d[row] a-i[center] j-c[space-between] p-top[6]")
                            .section(ContentSection::new().text("Kolor 2"))
                            .input(end_color_input)
                    })
                });

        self.section(start_color_section)
            .section_signal(end_color_section_signal)
    }

    fn mode_setting(self, active_settings: &'static ActiveSettings) -> Self {
        let mono_button = Button::builder()
            .no_hover()
            .text("Mono")
            .selected_signal(signal::not(active_settings.interpolate.signal()))
            .on_click(move |_| {
                active_settings.interpolation_progress.set(0);
                active_settings.interpolate.set_neq(false);
            });
        let duo_button = Button::builder()
            .no_hover()
            .text("Duo")
            .selected_signal(active_settings.interpolate.signal())
            .on_click(move |_| active_settings.interpolate.set_neq(true));

        // TODO: Happy pride month
        // let special_button = Button::builder()
        //     .no_hover()
        //     .text("Homo")
        //     .selected_signal(active_settings.interpolate.signal())
        //     .on_click(move |_| active_settings.interpolate.set_neq(true));

        self.heading(Heading::builder().text("Tryb wyświetlania"))
            .section(
                ContentSection::new()
                    .class_list("d[flex] f-d[row] j-c[space-around] a-i[center]")
                    .button(mono_button)
                    .button(duo_button), // .button(special_button),
            )
    }

    fn interpolation_speed_setting(self, active_settings: &'static ActiveSettings) -> Self {
        let speed_slider_setting = active_settings.interpolate.signal().dedupe().map(move |interpolate| {
            interpolate.then(|| {
                let step_input = Input::builder()
                    .class_list("w[150]")
                    .input_type(InputType::slider(1.0, 50.0))
                    .mixin(|b, _| {
                        apply_methods!(b, {
                            .style_signal(
                                "background",
                                active_settings.step
                                    .signal()
                                    .map(|percentage| {
                                        let percentage = percentage * 2;
                                        format!("linear-gradient(to right, rgb(57, 107, 41) {percentage:.2}%, rgb(12, 13, 13) {percentage:.2}%)")
                                    })
                            )
                            .tip!({
                                .text_signal(active_settings.step.signal().map(|step| format!("Prędkość zmiany koloru: {:.0}%\n\nPPM aby przywrócić do wartości domyślnej", step as f64 / DEFAULT_ROTATION_STEP as f64 * 100.0)))
                            })
                            .with_node!(input_elem => {
                                .event(move |event: ContextMenu| {
                                    debug_log!(@f "{event:?}");
                                    if event.button() == MouseButton::Right {
                                        input_elem.set_value(&DEFAULT_ROTATION_STEP.to_string());
                                        active_settings.step.set(DEFAULT_ROTATION_STEP);
                                    }
                                })
                            })
                        })
                    })
                    .value(active_settings.step.get().to_string())
                    .on_input(|_event, input| {
                        let value = input.value_as_number();
                        active_settings.step.set(value.clamp(0.0, 50.0) as u16);
                    });

                ContentSection::new()
                    .class_list("d[flex] f-d[row] a-i[center] j-c[space-between]")
                    .section(ContentSection::new().text("Prędkość"))
                    .input(step_input)
            })
        });

        self.section(
            ContentSection::new()
                .class_list("d[flex] f-d[column] p-top[6]")
                .section_signal(speed_slider_setting),
        )
    }
}

impl ActiveSettings {
    fn render(&'static self) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_right(decors::CloseButton::new())
            // TODO: add state bubble for toggling neon
            .build();
        let window_header = WindowHeader::new(decor);

        let window_content = WindowContent::builder()
            .class_list("d[flex] f-d[column]")
            .heading(
                Heading::builder()
                    .text("Ustawienia ogólne")
                    .class_list("first-heading"),
            )
            .section(ContentSection::new())
            .size_setting(self)
            .offset_setting(self)
            .mode_setting(self)
            .color_setting(self)
            .interpolation_speed_setting(self);

        AddonWindow::builder(ADDON_NAME)
            .header(window_header)
            .content(window_content)
            .build()
    }

    #[cfg(not(feature = "ni"))]
    fn init_neon_div(&'static self) -> JsResult<()> {
        use std::ops::Not;

        use dominator::DomBuilder;

        let neon = DomBuilder::<web_sys::HtmlDivElement>::new_html("div");
        let shadow = neon
            .__internal_shadow_root(web_sys::ShadowRootMode::Closed)
            .child(
                DomBuilder::<web_sys::HtmlDivElement>::new_html("div")
                    .style("position", "absolute")
                    .style_signal("display", Addons::active_signal(ADDON_NAME).ok_or_else(|| err_code!())?.map(|active| active.not().then_some("none")))
                    .style("border-radius", "50%")
                    .style("z-index", "2")
                    .style_signal("left", self.left.signal_ref(|left| format!("{:.0}px", left)))
                    .style_signal("top", self.top.signal_ref(|top| format!("{:.0}px", top)))
                    .style_signal("width", self.radius.signal_ref(|radius| format!("{:.0}px", radius * 2.0)))
                    .style_signal("height", self.radius.signal_ref(|radius| format!("{:.0}px", radius * 2.0)))
                    .style_signal("background", self.offset.signal().switch(|offset| self.color_signal().map(move |color| {
                        format!("radial-gradient(circle, rgba({}, {}, {}, 1) {:.0}%, transparent 70%)", color.red, color.green, color.blue, offset * 100.0)
                    })))
                    .into_dom(),
            );
        let neon = neon.__internal_transfer_callbacks(shadow).into_dom();
        let base_div = document()
            .get_element_by_id("base")
            .ok_or_else(|| err_code!())?;
        let _neon_handle = dominator::append_dom(&base_div, neon);

        Ok(())
    }
}

pub(super) fn init(active_settings: &'static ActiveSettings) -> JsResult<()> {
    let _settings_window_handle = WINDOWS_ROOT
        .try_append_dom(active_settings.render()?)
        .ok_or_else(|| err_code!())?;

    #[cfg(not(feature = "ni"))]
    active_settings.init_neon_div()?;

    Ok(())
}
