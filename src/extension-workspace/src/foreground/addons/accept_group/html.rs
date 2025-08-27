use dominator::Dom;

use crate::addon_window::prelude::*;
use crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT};

use super::*;

impl Settings {
    pub(super) fn render(&'static self) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_right(decors::CloseButton::new())
            .push_left(decors::OpacityToggle::new())
            .build();
        let settings_window_header = WindowHeader::new(decor);
        let settings_window_content = WindowContent::builder()
            .heading(
                Heading::builder()
                    .class_list("m-top[0]")
                    .text("Akceptuj zaproszenia od"),
            )
            .checkbox_pair(
                Checkbox::builder(self.none.clone()).text("Nieznajomych"),
                Checkbox::builder(self.friend.clone()).text("Przyjaciół"),
            )
            .checkbox_pair(
                Checkbox::builder(self.clan.clone()).text("Członków klanu"),
                Checkbox::builder(self.clan_ally.clone()).text("Sojuszników klanu"),
            )
            .apply_if(WorldConfig::has_fractions(), |builder| {
                builder.checkbox(
                    Checkbox::builder(self.fraction_ally.clone()).text("Sojuszników frakcji"),
                )
            })
            .section(ContentSection::new().class_list("label j-c[left] w[100%] a-i[center]"))
            .heading(
                Heading::builder()
                    .text("Nie akceptuj automatycznie zaproszeń od")
                    .info_bubble(
                        InfoBubble::builder()
                            .text("Wielkość liter nie ma znaczenia.")
                            .build(),
                    ),
            )
            .excluded_nicks_setting(&self.excluded_nicks);

        SettingsWindow::builder(AddonName::AcceptGroup)
            .header(settings_window_header)
            .content(settings_window_content)
            .build()
    }
}

pub(super) fn init(settings_window: &'static Settings) -> JsResult<()> {
    let _handle = WINDOWS_ROOT
        .try_append_dom(settings_window.render()?)
        .ok_or_else(|| err_code!())?;

    Ok(())
}
