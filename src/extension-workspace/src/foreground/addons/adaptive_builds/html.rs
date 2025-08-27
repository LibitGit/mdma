use dominator::{html, Dom};
use futures_signals::signal::SignalExt;

use crate::addon_window::prelude::*;
use crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT};
use crate::prelude::*;

use super::{Settings, ADDON_NAME};

impl Settings {
    fn render(&'static self) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_right(decors::CloseButton::new())
            .build();
        let window_header = WindowHeader::new(decor);

        let window_content = WindowContent::builder().section(
            ContentSection::new()
                .class_list("d[flex] g[5] f-d[row]")
                .checkbox(
                    Checkbox::builder(self.colossus.active.clone())
                        .class_list("w-s[pre-line] l-h[16]")
                        .text("Wybieraj zestaw automatycznie\nna mapie z kolosami"),
                )
                .button(
                    Button::builder()
                        .class_list("w[98] t-a[left] h[28]")
                        .no_hover()
                        .text_signal(
                            self.colossus
                                .build
                                .signal()
                                .map(|curr_build| format!("Zestaw {curr_build}")),
                        )
                        .mixin(|builder| {
                            builder.child(html!("div", {
                                .class!(pos[absolute] r[8] align-center menu-arrow)
                            }))
                        })
                        .on_mousedown(|event| event.stop_propagation())
                        .on_click(|_| self.scroll_active.set_neq(true))
                        .scroll_wrapper(
                            ScrollWrapper::builder(|| || self.scroll_active.set(false))
                                .visible_signal(self.scroll_active.signal())
                                .class_list("w[90] l[1]")
                                .option(self.build_scroll_wrapper_option("Zestaw 1", 1))
                                .option(self.build_scroll_wrapper_option("Zestaw 2", 2))
                                .option(self.build_scroll_wrapper_option("Zestaw 3", 3))
                                .option(self.build_scroll_wrapper_option("Zestaw 4", 4))
                                .option(self.build_scroll_wrapper_option("Zestaw 5", 5))
                                .option(self.build_scroll_wrapper_option("Zestaw 6", 6))
                                .option(self.build_scroll_wrapper_option("Zestaw 7", 7))
                                .option(self.build_scroll_wrapper_option("Zestaw 8", 8))
                                .option(self.build_scroll_wrapper_option("Zestaw 9", 9))
                                .build(),
                        ),
                ),
        );

        SettingsWindow::builder(ADDON_NAME)
            .header(window_header)
            .content(window_content)
            .build()
    }

    fn build_scroll_wrapper_option(
        &'static self,
        txt: &'static str,
        build_no: u8,
    ) -> ScrollWrapperOption {
        ScrollWrapperOption::builder()
            .text(txt)
            .on_click(move |_| self.colossus.build.set_neq(build_no))
            .build()
    }
}

pub(super) fn init(settings: &'static Settings) -> JsResult<()> {
    let _settings_window_handle = WINDOWS_ROOT
        .try_append_dom(settings.render()?)
        .ok_or_else(|| err_code!())?;

    Ok(())
}
