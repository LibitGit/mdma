use dominator::Dom;

use crate::addon_window::prelude::*;
use crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT};
use crate::prelude::*;

use super::{ADDON_NAME, Settings};

impl Settings {
    fn render(&'static self) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_right(decors::CloseButton::new())
            .build();
        let window_header = WindowHeader::new(decor);
        let window_content = WindowContent::builder()
            .class_list("f-d[column]")
            .auto_remove_setting(self);

        SettingsWindow::builder(ADDON_NAME)
            .header(window_header)
            .content(window_content)
            .build()
    }
}

trait WindowContentExt {
    fn auto_remove_setting(self, settings: &'static Settings) -> Self;
}

impl WindowContentExt for WindowContent {
    fn auto_remove_setting(self, settings: &'static Settings) -> Self {
        let auto_remove_checkbox =
            Checkbox::builder(settings.auto_remove.clone())
            .text("Usuwaj po [s]")
            .info_bubble(
                InfoBubble::builder()
                    .text("Wpis zostanie usunięty po upływie podanego okresu czasu liczonego od maksymalnego respawnu.")
                    .build(),
            );
        let auto_remove_input = Input::builder()
            .value(settings.auto_remove_sec.get().to_string())
            .input_type(InputType::number(u8::MIN as f64, u8::MAX as f64))
            .maxlength("3")
            .on_input(move |event, elem| {
                event.prevent_default();
                event.stop_immediate_propagation();

                let value = elem.value_as_number() as u8;
                settings.auto_remove_sec.set_neq(value);
            });
        let font_size_section = ContentSection::new()
            .class_list("d[flex] j-c[space-between]")
            .checkbox(auto_remove_checkbox)
            .input(auto_remove_input);

        self.section(font_size_section)
    }
}

    // #[cfg(not(feature = "ni"))]
    // fn init_timers(&'static self) -> JsResult<()> {
    //     use std::ops::Not;

    //     use dominator::DomBuilder;

    //     let neon = DomBuilder::<web_sys::HtmlDivElement>::new_html("div");
    //     let shadow = neon
    //         .__internal_shadow_root(web_sys::ShadowRootMode::Closed)
    //         .child(
    //             DomBuilder::<web_sys::HtmlDivElement>::new_html("div")
    //                 .style("position", "absolute")
    //                 .style_signal("display", Addons::active_signal(ADDON_NAME).ok_or_else(|| err_code!())?.map(|active| active.not().then_some("none")))
    //                 .style("z-index", "2")
    //                 .style_signal("left", self.left.signal_ref(|left| format!("{:.0}px", left)))
    //                 .style_signal("top", self.top.signal_ref(|top| format!("{:.0}px", top)))
    //                 .style_signal("width", self.radius.signal_ref(|radius| format!("{:.0}px", radius * 2.0)))
    //                 .style_signal("height", self.radius.signal_ref(|radius| format!("{:.0}px", radius * 2.0)))
    //                 .style_signal("background", self.offset.signal().switch(|offset| self.color_signal().map(move |color| {
    //                     format!("radial-gradient(circle, rgba({}, {}, {}, 1) {:.0}%, transparent 70%)", color.red, color.green, color.blue, offset * 100.0)
    //                 })))
    //                 .into_dom(),
    //         );
    //     let neon = neon.__internal_transfer_callbacks(shadow).into_dom();
    //     let base_div = document()
    //         .get_element_by_id("base")
    //         .ok_or_else(|| err_code!())?;
    //     let _neon_handle = dominator::append_dom(&base_div, neon);

    //     Ok(())
    // }

pub(super) fn init(settings: &'static Settings) -> JsResult<()> {
    let _settings_window_handle = WINDOWS_ROOT
        .try_append_dom(settings.render()?)
        .ok_or_else(|| err_code!())?;

    Ok(())
}
