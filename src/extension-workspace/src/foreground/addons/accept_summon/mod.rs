mod html;

use std::str::Chars;

use proc_macros::Settings;

use crate::addon_window::ui_components::NickInput;
use crate::bindings::engine::communication;
use crate::prelude::*;

const ADDON_NAME: AddonName = AddonName::AcceptGroup;

#[derive(Settings, Default)]
struct Settings {
    excluded_nicks: NickInput,
}

pub(crate) fn init() -> JsResult<()> {
    let settings_window = Settings::new(ADDON_NAME);
    html::init(settings_window)?;

    Emitter::intercept_on(EmitterEvent::Ask, move |socket_response: &mut Response| {
        Box::pin(async move {
            if !Addons::is_active(ADDON_NAME) {
                return Ok(());
            }

            let Some(ask) = socket_response.ask.as_ref() else {
                return Ok(());
            };

            if ask
                .re
                .as_deref()
                .is_none_or(|reply| reply != communication::party::ACCEPT_SUMMON)
            {
                return Ok(());
            }

            let summon_handled =
                handle_summon(ask.q.as_ref().ok_or_else(|| err_code!())?, settings_window);

            if summon_handled {
                socket_response.ask = None;
            };

            Ok(())
        })
    })?;

    Ok(())
}

fn handle_summon(question: &str, settings_window: &Settings) -> bool {
    let nick = get_nick_from_question(question.chars());

    match settings_window
        .excluded_nicks
        .lock_ref()
        .iter()
        .any(|excluded_nick| {
            nick.to_lowercase()
                .starts_with(&excluded_nick.to_lowercase())
        }) {
        true => {
            message(
                "Nie akceptuje przywołania, ponieważ nick przywołującego jest na liście wykluczeń.",
            )
            .unwrap_js();
            false
        }
        false => {
            communication::send_task(&communication::party::accept_summon(true)).unwrap_js();
            true
        }
    }
}

// FIXME: Doesn't work when someone has nick "* przyzywa do*".
fn get_nick_from_question(question: Chars<'_>) -> String {
    let mut nick = String::new();

    for (index, char) in question.enumerate() {
        if index < 6 {
            continue;
        }
        if index > 26 {
            break;
        }

        nick.push(char);
    }

    nick
}
