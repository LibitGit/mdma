use dominator::{Dom, events::Click, html};
use wasm_bindgen::{JsValue, intern};
use web_sys::HtmlElement;

use crate::{addon_window::MdmaAddonWindow, prelude::*, utils::logging::console_log};

impl AddonData {
    /// Renders an addon box in the UI.
    pub(crate) fn render(&self, addon_name: AddonName) -> Dom {
        html!("div", {
            .class("mdma-addon")
            .class_signal("active", self.active.signal())
            .child(html!("div", {
                .class!(addon-title)
                .text(addon_name.as_str())
            }))
            .child(html!("div", {
                .class("addons-positioner")
                .apply_if(
                    addon_name.default_header_description(WindowType::AddonWindow).is_some(),
                    |builder| builder.add_window_toggle(WindowType::AddonWindow, addon_name)
                )
                .apply_if(
                    addon_name.default_header_description(WindowType::SettingsWindow).is_some(),
                    |builder| builder.add_window_toggle(WindowType::SettingsWindow, addon_name)
                )
            }))
            .event(move |event: Click| {
                let target_class_list = match event.dyn_target::<HtmlElement>() {
                    Some(target) => target.class_list(),
                    None => return console_error!(),
                };

                if target_class_list.contains("mdma-button") {
                     return;
                }

                Self::toggle_addon(addon_name)
            })
        })
    }

    /// Toggles the current addon active state
    fn toggle_addon(addon_name: AddonName) {
        let addon_data = Addons::get_addon(addon_name).unwrap_js();
        let after_toggle = !addon_data.active.get();

        addon_data.active.set(after_toggle);

        let msg = match after_toggle {
            true => {
                JsValue::from_str(intern("turned on "))
                    + JsValue::from_str(intern(addon_name.key_str()))
            }
            false => {
                JsValue::from_str(intern("turned off "))
                    + JsValue::from_str(intern(addon_name.key_str()))
            }
        };

        console_log(msg);
    }
}
