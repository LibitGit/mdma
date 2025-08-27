use super::*;
use crate::addon_window::prelude::*;
use crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT};
use dominator::Dom;
impl Settings {
    fn render(&'static self) -> JsResult<Dom> {
        //let excluded_nicks = state.excluded_nicks.clone();
        //let excluded_nicks_input = Input::builder()
        //    .placeholder("Nick Gracza")
        //    .maxlength("21")
        //    .confirm_button(InputButton::builder().on_click(move |_, input_elem| {
        //        let nick = input_elem.value();
        //
        //        if nick.len() < 3 {
        //            return message("Nick jest zbyt krótki.");
        //        }
        //
        //        let mut is_all_whitespace = true;
        //        for (index, char) in nick.chars().enumerate() {
        //            match index == 0 && char.is_whitespace() {
        //                true => return message("Nick nie może zaczynać się od spacji."),
        //                false => is_all_whitespace = false,
        //            }
        //            if !char.is_alphabetic() && !char.is_whitespace() {
        //                return message("Nick zawiera niedozwolone znaki.");
        //            }
        //        }
        //        if is_all_whitespace {
        //            return message("Nick nie może składać się tylko ze spacji.");
        //        }
        //        if matches!(
        //            nick.to_lowercase().as_str(),
        //            "libit" | "limit" | "lihit" | "liwit" | "litib" | "lipit"
        //        ) {
        //            input_elem.set_value("");
        //            return message("Ode mnie nie przyjmiesz?");
        //        }
        //        if excluded_nicks.lock_ref().contains(&nick) {
        //            return message("Ten nick znajduje się już na liście wykluczeń.");
        //        }
        //
        //        excluded_nicks.lock_mut().push_cloned(nick);
        //        input_elem.set_value("");
        //    }));

        let decor = HeaderDecor::builder()
            .push_right(decors::CloseButton::new())
            .push_left(decors::OpacityToggle::new())
            .build();
        let settings_window_header = WindowHeader::new(decor);
        let settings_window_content = WindowContent::builder()
            .heading(
                Heading::builder()
                    .class_list("m-top[0]")
                    .text("Nie akceptuj automatycznie przywołań od")
                    .info_bubble(
                        InfoBubble::builder()
                            .text("Wielkość liter nie ma znaczenia.")
                            .build(),
                    ),
            )
            .excluded_nicks_setting(&self.excluded_nicks);

        SettingsWindow::builder(AddonName::AcceptSummon)
            .header(settings_window_header)
            .content(settings_window_content)
            .build()
    }
}

pub(crate) fn init(settings_window: &'static Settings) -> JsResult<()> {
    let _handle = WINDOWS_ROOT
        .try_append_dom(settings_window.render()?)
        .ok_or_else(|| err_code!())?;

    Ok(())
}
