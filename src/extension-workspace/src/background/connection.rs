use std::{
    cell::RefCell,
    pin::Pin,
    task::{Context, Poll},
};

use common::{
    debug_log, err_code, map_err, messaging::prelude::*, sleep, web_extension_sys::browser,
};
use futures::{channel::oneshot, future::FusedFuture};
use pin_project::pin_project;
use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::{
    FOREGROUND_PORT, console_error,
    dispatcher::Dispatcher,
    types::{AuthResponse, ExecutionError, Jwt, MessageExt, StorageRefreshToken},
};

const POPUP_OPEN_DEADLINE: u32 = 300;

#[derive(Debug, Serialize)]
struct WebAuthFlowDetails<'a> {
    interactive: Option<bool>,
    url: &'a str,
}

impl<'a> WebAuthFlowDetails<'a> {
    fn new(url: &'a str) -> Self {
        Self {
            interactive: Some(true),
            url,
        }
    }
}

#[derive(Debug)]
pub struct Connection {
    /// User details available after connection authorization.
    pub user: RefCell<Option<User>>,
}

impl Connection {
    fn new(user: User) -> Self {
        Self {
            user: RefCell::new(Some(user)),
        }
    }

    pub(super) async fn establish_authorized(dispatcher: &mut Dispatcher) -> Result<Self, JsValue> {
        let (user, from_authorized) = Self::_establish_authorized(dispatcher).await?;

        Self::store_refresh_token(user.refresh_token.clone()).await?;

        if !from_authorized {
            Self::notify_login_success(&user).await?;
        }

        Ok(Self::new(user))
    }

    async fn _establish_authorized(dispatcher: &mut Dispatcher) -> Result<(User, bool), JsValue> {
        let storage_obj = browser()
            .storage()
            .sync()
            .get(&JsValue::from_str("refresh_token"))
            .await
            .map_err(map_err!())?
            .into();
        let Some(refresh_token) =
            serde_wasm_bindgen::from_value::<StorageRefreshToken>(storage_obj)
                .map_err(map_err!(from))?
                .token
                .and_then(|token| token.validate().then(|| token.into_inner()))
        else {
            return Self::from_unauthorized(dispatcher).await;
        };

        dispatcher
            .socket
            .send(
                Message::builder(Task::Tokens, Target::Backend, MessageKind::Request)
                    .refresh_token(refresh_token)
                    .build(),
            )
            .await
            .map_err(map_err!(from))?;

        let response = dispatcher
            .socket
            .recv()
            .await
            .ok_or_else(|| err_code!())??;
        let validator = MessageValidator::builder(Target::Backend)
            .kind(MessageKind::Response)
            .task(Task::Tokens)
            .build();

        validator.validate(&response)?;

        if response.access_token.is_none() {
            return Self::from_unauthorized(dispatcher).await;
        }

        Ok((response.try_into().map_err(|_| err_code!())?, true))
    }

    async fn from_unauthorized(dispatcher: &mut Dispatcher) -> Result<(User, bool), JsValue> {
        let auth_response = Self::web_auth_workflow(dispatcher).await?;
        let msg = Message::builder(Task::Tokens, Target::Backend, MessageKind::Request)
            .code(auth_response.code)
            .build();

        dispatcher.socket.send(msg).await.map_err(map_err!(from))?;

        let response = dispatcher
            .socket
            .recv()
            .await
            .ok_or_else(|| err_code!())??;

        debug_log!(@f "from_unauthorized response: {response:#?}");

        let validator = MessageValidator::builder(Target::Backend)
            .kind(MessageKind::Response)
            .task(Task::Tokens)
            .build();

        validator.validate(&response)?;

        if let Some(err) = response.error {
            let popup_update = match err.is_empty() {
                true => PopupUpdate::join_discord(), // no member data on empty string.
                false => PopupUpdate {
                    state: Some(PopupState::LoggedOut),
                    msg: Some(PopupMessage::LoginFailed { reason: Some(err) }),
                },
            };
            Message::builder(Task::OAuth2, Target::Popup, MessageKind::Response)
                .popup(popup_update)
                .build()
                .execute()
                .await?;

            return Box::pin(Self::from_unauthorized(dispatcher)).await;
        }

        Ok((response.try_into().map_err(|_| err_code!())?, false))
    }

    async fn web_auth_workflow(dispatcher: &mut Dispatcher) -> Result<AuthResponse, JsValue> {
        let mut rx = AuthFlow::launch(dispatcher).await?;
        let redirect_url = loop {
            futures::select! {
                auth_flow_response = rx => {
                    match auth_flow_response.map_err(map_err!(from))? {
                        Ok(url) => break url.as_string().ok_or_else(|| err_code!())?,
                        Err(error) => {
                            Message::builder(Task::OAuth2, Target::Popup, MessageKind::Response)
                                .popup(PopupUpdate {
                                    state: Some(PopupState::LoggedOut),
                                    msg: Some(PopupMessage::LoginFailed{
                                        reason: error
                                            .dyn_ref::<js_sys::Error>()
                                            .and_then(|err| err.message().as_string()),
                                    })
                                })
                                .build()
                                .execute()
                                .await?;

                            rx = AuthFlow::launch(dispatcher).await?;
                        }
                    };
                }
                runtime_msg = dispatcher.runtime.recv() => {
                    let msg = runtime_msg.ok_or_else(|| err_code!())?;
                    let validator = MessageValidator::builder(Target::Popup).task(Task::UserData).build();

                    validator.validate(&msg)?;

                    common::debug_log!("DURING LOGGING IN");
                    Message::builder(Task::UserData, msg.sender, MessageKind::Response)
                        .popup(PopupState::LoggingIn)
                        .build()
                        .execute()
                        .await?
                }
                _ = sleep(10_000) => {
                    dispatcher
                        .socket
                        .send(Message::builder(Task::KeepAlive, Target::Backend, MessageKind::Request).build())
                        .await
                        .map_err(map_err!(from))?;

                    debug_log!("sent heartbeat")
                }
                port_msg = dispatcher.port.recv() => {
                    let msg = port_msg.ok_or_else(|| err_code!())?;

                    MessageValidator::new(Target::Foreground).validate(&msg)?;

                    match msg.task {
                        Task::Handshake => Message::builder(Task::Handshake, msg.sender, MessageKind::Response)
                            .error("[MDMA::RS] Aby korzystać z zestawu dokończ logowanie wewnątrz okna Discord!")
                            .build()
                            .execute()
                            .await?,
                        Task::OpenPopup => AuthFlow::on_open_popup(dispatcher, PopupState::LoggingIn).await?,
                        _ => unreachable!(),
                    }
                }
            }
        };

        AuthResponse::from_url(redirect_url).ok_or_else(|| err_code!())
    }

    async fn store_refresh_token(token: Jwt) -> Result<(), JsValue> {
        browser()
            .storage()
            .sync()
            .set(
                serde_wasm_bindgen::to_value(&StorageRefreshToken::new(token))
                    .map_err(map_err!(from))?
                    .unchecked_ref(),
            )
            .await
            .map_err(map_err!())
    }

    async fn notify_login_success(user: &User) -> Result<(), JsValue> {
        if FOREGROUND_PORT.with_borrow(Option::is_some) {
            Message::new(Task::Handshake, Target::Foreground, MessageKind::Event)
                .execute()
                .await?;
        }

        Message::builder(Task::OAuth2, Target::Popup, MessageKind::Response)
            .username(user.nick.clone())
            .maybe_premium(user.premium)
            .popup(PopupUpdate {
                state: Some(PopupState::LoggedIn),
                msg: FOREGROUND_PORT
                    .with_borrow(Option::is_some)
                    .then_some(PopupMessage::RefreshAfterLogin),
            })
            .build()
            .execute()
            .await?;

        Ok(())
    }
}

#[pin_project]
struct AuthFlow {
    #[pin]
    rx: oneshot::Receiver<Result<JsValue, JsValue>>,
}

impl Future for AuthFlow {
    type Output = Result<Result<JsValue, JsValue>, oneshot::Canceled>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.as_mut().project().rx.poll(cx)
    }
}

impl FusedFuture for AuthFlow {
    fn is_terminated(&self) -> bool {
        self.rx.is_terminated()
    }
}

impl AuthFlow {
    /// Wait for user prompt and launch the Discord oauth2 procedure returning a
    /// cancellation safe [`Receiver`][oneshot::Receiver].
    async fn launch(dispatcher: &mut Dispatcher) -> Result<Self, JsValue> {
        loop {
            futures::select! {
                runtime_msg = dispatcher.runtime.recv() => {
                    let msg = runtime_msg.ok_or_else(|| err_code!())?;

                    MessageValidator::new(Target::Popup).validate(&msg)?;

                    match msg.task {
                        Task::OAuth2 => break,
                        Task::UserData => {
                            common::debug_log!("BEFORE LOGGING IN");
                            Message::builder(Task::UserData, msg.sender, MessageKind::Response)
                                .popup(PopupState::LoggedOut)
                                .build()
                                .execute()
                                .await?
                        }
                        _ => unreachable!(),
                    }
                }
                port_msg = dispatcher.port.recv() => {
                    let msg = port_msg.ok_or_else(|| err_code!())?;

                    MessageValidator::new(Target::Foreground).validate(&msg)?;

                    match msg.task {
                        Task::Handshake => Message::builder(Task::Handshake, msg.sender, MessageKind::Response)
                            .error("[MDMA::RS] Aby korzystać z zestawu zaloguj się klikając w ikonę rozszerzenia!")
                            .build()
                            .execute()
                            .await?,
                        Task::OpenPopup => Self::on_open_popup(dispatcher, PopupState::LoggedOut).await?,
                        _ => unreachable!(),
                    }

                }
            };
        }

        let auth_url = match option_env!("LOGIN_URL") {
            Some(url) => url,
            None => "http://localhost:3000/login",
        };
        let auth_flow_details = serde_wasm_bindgen::to_value(&WebAuthFlowDetails::new(auth_url))
            .map_err(map_err!(from))?;

        let (tx, rx) = oneshot::channel::<Result<JsValue, JsValue>>();

        wasm_bindgen_futures::spawn_local(async move {
            if tx
                .send(
                    browser()
                        .identity()
                        .launch_web_auth_flow(auth_flow_details.unchecked_ref())
                        .await,
                )
                .is_err()
            {
                console_error!()
            }
        });

        Ok(Self { rx })
    }

    async fn on_open_popup<B: Into<PopupUpdate>>(
        dispatcher: &mut Dispatcher,
        popup_update: B,
    ) -> Result<(), JsValue> {
        let popup_open = Self::try_open_popup(dispatcher, popup_update).await?;
        if !popup_open {
            debug_log!("POPUP FAILED TO OPEN!");
        }
        let maybe_error =
            (!popup_open).then_some("[MDMA::RS] Nie udało się otworzyć okna rozszerzenia!");

        Message::builder(Task::OpenPopup, Target::Foreground, MessageKind::Response)
            .maybe_error(maybe_error)
            .build()
            .execute()
            .await?;

        Ok(())
    }

    /// Returns:
    ///  * `Ok(true)` if the popup was opened successfuly.
    ///  * `Ok(false)` if the popup failed to open in time.
    ///  * `Err(err)` if messaging fails.
    async fn try_open_popup<B: Into<PopupUpdate>>(
        dispatcher: &mut Dispatcher,
        popup_update: B,
    ) -> Result<bool, JsValue> {
        let message =
            Message::new(Task::OpenPopup, Target::Popup, MessageKind::Event).to_value()?;

        debug_log!("Sending test message to popup");

        match browser()
            .runtime()
            .send_message(&message)
            .await
            .map_err(Into::into)
        {
            Ok(_) => return Ok(true), // popup already open
            Err(ExecutionError::JsError(val)) => return Err(val).map_err(map_err!()),
            Err(ExecutionError::NoReceiver) => {} // try opening
        }

        if Dispatcher::open_popup().await.is_err() {
            return Ok(false);
        }

        let Ok(msg) = dispatcher.runtime.recv_timeout(POPUP_OPEN_DEADLINE).await else {
            return Ok(false);
        };
        let validator = MessageValidator::builder(Target::Popup)
            .task(Task::UserData)
            .build();

        validator.validate(&msg)?;

        Message::builder(Task::UserData, Target::Popup, MessageKind::Response)
            .popup(popup_update)
            .build()
            .execute()
            .await?;

        Ok(true)
    }
}

/// User details available after connection authorization.
#[derive(Debug)]
pub struct User {
    /// Determines session persistance.
    pub scope: SessionScope,
    /// Current game session data.
    pub session: Option<Session>,
    access_token: Jwt,
    refresh_token: Jwt,
    pub nick: String,
    pub premium: Option<Premium>,
}

impl User {
    fn new(
        scope: SessionScope,
        access_token: Jwt,
        refresh_token: Jwt,
        username: String,
        premium: Option<Premium>,
    ) -> Self {
        Self {
            scope,
            session: None,
            access_token,
            refresh_token,
            nick: username,
            premium,
        }
    }
}

impl TryFrom<Message> for User {
    type Error = Message;

    fn try_from(mut value: Message) -> Result<Self, Self::Error> {
        let Some(scope) = value.session_scope else {
            return Err(value);
        };
        let Some(access_token) = value.access_token.take().map(Jwt::new) else {
            return Err(value);
        };
        let Some(refresh_token) = value.refresh_token.take().map(Jwt::new) else {
            return Err(value);
        };
        let Some(username) = value.username.take() else {
            return Err(value);
        };
        let maybe_premium = value.premium;

        Ok(Self::new(
            scope,
            access_token,
            refresh_token,
            username,
            maybe_premium,
        ))
    }
}

#[derive(Debug)]
pub struct Session {
    /// Id of the account the user is currently logged in to.
    pub account_id: u64,
    /// Id of the character the user is currently playing as.
    pub char_id: u64,
    /// Addon settings of the session.
    pub addon_settings: Value,
}
