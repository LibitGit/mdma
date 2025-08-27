use std::sync::OnceLock;

use common::{err_code, messaging::prelude::*};
use futures::{SinkExt, StreamExt, channel::mpsc};
use wasm_bindgen::prelude::*;

use crate::{DisplayMessage, LoadingReason, Popup, UserData, console_error};

// TODO: rename to sth like `RUNTIME_TX` since that's what it is.
static DISPATCHER: OnceLock<mpsc::UnboundedSender<Message>> = OnceLock::new();

pub struct Dispatcher {
    rx: mpsc::UnboundedReceiver<Message>,
}

impl Dispatcher {
    fn _new(rx: mpsc::UnboundedReceiver<Message>) -> Self {
        Self { rx }
    }

    pub(super) fn init() -> Result<Self, JsValue> {
        let (tx, rx) = mpsc::unbounded();

        DISPATCHER.set(tx).map_err(|_| err_code!())?;

        Ok(Self::_new(rx))
    }

    pub(super) fn spawn_event_loop(self, state: &'static Popup) {
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(err_code) = Self::_spawn_event_loop(self.rx, state).await {
                console_error!(err_code)
            }
        });
    }

    async fn _spawn_event_loop(
        mut rx: mpsc::UnboundedReceiver<Message>,
        state: &'static Popup,
    ) -> Result<(), JsValue> {
        let validator = MessageValidator::builder(Target::Background)
            .maybe_kind(None)
            .build();

        while let Some(msg) = rx.next().await {
            validator.validate(&msg)?;

            match msg.kind {
                MessageKind::Event => match msg.task {
                    Task::OpenPopup => common::debug_log!("RECEIVED OPEN POPUP IN POPUP"),
                    _ => unreachable!(),
                },
                MessageKind::Response => match msg.task {
                    Task::OAuth2 => {
                        let update = msg.popup.ok_or_else(|| err_code!())?;
                        let update_state = update.state.ok_or_else(|| err_code!())?;

                        if matches!(
                            update_state,
                            PopupState::LoggedOut | PopupState::LoggedIn | PopupState::JoinDiscord
                        ) {
                            state
                                .loading
                                .remove_reason(LoadingReason::LoggingIn)
                                .ok_or_else(|| err_code!())?;
                        }
                        let display_msg = update.msg.map(|msg| match msg {
                            PopupMessage::LoginFailed { .. }
                                if update_state == PopupState::JoinDiscord =>
                            {
                                DisplayMessage::join_discord()
                            }
                            PopupMessage::LoginFailed { reason } => {
                                let txt = match reason {
                                    Some(reason) => {
                                        format!("Wystąpił błąd podczas logowania - {reason}")
                                    }
                                    None => "Wystąpił błąd podczas logowania!".to_owned(),
                                };
                                DisplayMessage::error(txt)
                            }
                            PopupMessage::RefreshAfterLogin => DisplayMessage::success(
                                "Odśwież kartę z Margonem, aby wczytać zestaw!".to_owned(),
                            ),
                        });

                        state.message.set_neq(display_msg);

                        state
                            .user
                            .set_neq(msg.username.map(|nick| UserData::new(nick, msg.premium)));
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }

        Err(err_code!())
    }

    pub async fn dispatch(item: Message) -> Result<(), mpsc::SendError> {
        DISPATCHER.wait().send(item).await
    }

    pub async fn recv(&mut self) -> Option<Message> {
        self.rx.next().await
    }
}
