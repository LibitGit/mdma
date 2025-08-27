mod html;

use std::str::Chars;

use common::err_code;
use futures_signals::signal::Mutable;

use crate::addon_window::ui_components::NickInput;
use crate::bindings::engine::communication;
use crate::globals::others::OtherBTreeMap;
use crate::prelude::*;

use proc_macros::Settings;

const ADDON_NAME: AddonName = AddonName::AcceptGroup;

#[derive(Settings)]
struct Settings {
    none: Mutable<bool>,
    friend: Mutable<bool>,
    clan: Mutable<bool>,
    clan_ally: Mutable<bool>,
    fraction_ally: Mutable<bool>,
    excluded_nicks: NickInput,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            none: Mutable::new(false),
            friend: Mutable::new(true),
            clan: Mutable::new(true),
            clan_ally: Mutable::new(true),
            fraction_ally: Mutable::new(true),
            excluded_nicks: NickInput::default(),
        }
    }
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
                .is_none_or(|reply| reply != communication::party::ACCEPT)
            {
                return Ok(());
            }

            let invite_handled =
                try_handle_invite(ask.q.as_ref().ok_or_else(|| err_code!())?, settings_window)?;

            if invite_handled {
                socket_response.ask = None;
            };

            Ok(())
        })
    })?;

    Ok(())
}

fn try_handle_invite(question: &str, settings_window: &Settings) -> JsResult<bool> {
    let can_handle_from_peers = settings_window.friend.get() && settings_window.clan.get();
    let nick = get_nick_from_question(question.chars());
    let handled_from_peers = can_handle_from_peers && handle_invite_from_peers(&nick);

    if handled_from_peers {
        return Ok(true);
    }

    try_handle_invite_from_others(nick.get_cloned(), settings_window)
}

fn get_nick_from_question(question: Chars<'_>) -> Mutable<String> {
    let mut nick = String::new();
    let mut write = false;

    for char in question {
        match char {
            '[' if write => break,
            ']' => write = true,
            '[' => write = false,
            _ => {
                if write {
                    nick.push(char)
                }
            }
        }
    }

    Mutable::new(nick)
}

fn handle_invite_from_peers(nick: &Mutable<String>) -> bool {
    let nick = nick.get_cloned();

    Peers::get()
        .lock_ref()
        .iter()
        .any(|(_, peer_data)| *peer_data.nick.lock_ref() == nick)
        && {
            communication::send_task(&communication::party::accept(true)).unwrap_js();
            true
        }
}

fn try_handle_invite_from_others(nick: String, settings_window: &Settings) -> JsResult<bool> {
    let Some((_, other_data)) = OtherBTreeMap::get()
        .lock_ref()
        .iter()
        .find(|(_, other_data)| *other_data.nick.lock_ref() == nick)
        .map(|(id, other_data)| (*id, other_data.clone()))
    else {
        return Err(err_code!());
    };
    let relation = other_data.relation.get();
    let should_accept = match relation {
        Relation::None => &settings_window.none,
        Relation::Friend => &settings_window.friend,
        Relation::Clan => &settings_window.clan,
        Relation::ClanAlly => &settings_window.clan_ally,
        Relation::FractionAlly => &settings_window.fraction_ally,
        _ => return Err(err_code!()),
    };

    if should_accept.get() {
        communication::send_task(&communication::party::accept(true))?;
        return Ok(true);
    }

    Ok(false)
}
