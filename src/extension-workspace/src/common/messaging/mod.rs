use std::fmt;

#[cfg(feature = "backend")]
use axum::extract::ws::Message as WsMessage;
#[cfg(feature = "background")]
use gloo_net::websocket::Message as WsMessage;
#[cfg(feature = "extension")]
use js_sys::Object;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, Visitor},
};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::skip_serializing_none;
use uuid::{Uuid, fmt::Simple};
#[cfg(feature = "extension")]
use wasm_bindgen::prelude::*;

#[cfg(any(feature = "backend", feature = "background"))]
use crate::connection::SessionScope;
#[cfg(feature = "extension")]
use crate::map_err;
#[cfg(feature = "popup")]
use crate::web_extension_sys::browser;
#[cfg(any(feature = "foreground", feature = "background"))]
use crate::web_extension_sys::cookies::{self, CookieDetails};

pub mod validator;
pub mod prelude {
    pub use super::{Message, MessageKind, Premium, Target, Task, validator::MessageValidator};

    #[cfg(any(feature = "backend", feature = "background"))]
    pub use crate::connection::SessionScope;

    #[cfg(any(feature = "popup", feature = "background"))]
    pub use super::{PopupMessage, PopupState, PopupUpdate};

    #[cfg(any(feature = "foreground", feature = "background"))]
    pub use super::Cookie;
}

// Whenever adding a new task make sure backend is in sync with extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u32)]
pub enum Task {
    Handshake,
    Tokens,
    KeepAlive,
    OAuth2,
    UserData,
    LogOut,
    OpenPopup,
    Cookie,
    InitSession,
    TerminateSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Target {
    Backend,
    Background,
    Foreground,
    Popup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum MessageKind {
    Request,
    Response,
    Event,
}

/// Serde deserialization decorator to map Uuid to Simple formatter.
pub fn uuid_as_simple<'de, D>(de: D) -> Result<Simple, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Uuid::deserialize(de)?.simple())
}

pub fn opt_uuid_as_simple<'de, D>(de: D) -> Result<Option<Simple>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalUuidVisitor;

    impl<'de> Visitor<'de> for OptionalUuidVisitor {
        type Value = Option<Simple>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an optional UUID string")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            Ok(Some(Uuid::deserialize(deserializer)?.simple()))
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    de.deserialize_option(OptionalUuidVisitor)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LogOutDetails {
    /// Whether to log out from all devices (true) or only from the current
    /// account (false).
    pub all_devices: bool,
}

impl LogOutDetails {
    pub const fn new(all_devices: bool) -> Self {
        Self { all_devices }
    }
}

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub task: Task,
    pub target: Target,
    pub sender: Target,
    pub kind: MessageKind,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    // #[serde(deserialize_with = "opt_uuid_as_simple", default)]
    // pub cid: Option<Simple>,
    pub username: Option<String>,
    pub premium: Option<Premium>,
    pub settings: Option<Value>,
    #[cfg(any(feature = "foreground", feature = "background"))]
    pub cookie: Option<Cookie>,
    #[cfg(any(feature = "backend", feature = "background"))]
    pub session_scope: Option<SessionScope>,
    pub log_out: Option<LogOutDetails>,
    /// Code provided after logging in via oauth2, used for establishing an
    /// authorized session.
    pub code: Option<String>,
    pub error: Option<String>,
    #[cfg(any(feature = "popup", feature = "background"))]
    pub popup: Option<PopupUpdate>,
}

impl Message {
    cfg_if::cfg_if! {
        if #[cfg(feature = "background")] {
            const CURRENT_TARGET: Target = Target::Background;
        } else if #[cfg(feature = "foreground")] {
            const CURRENT_TARGET: Target = Target::Foreground;
        } else if #[cfg(feature = "popup")] {
            const CURRENT_TARGET: Target = Target::Popup;
        } else if #[cfg(feature = "backend")] {
            const CURRENT_TARGET: Target = Target::Backend;
        } else {
            const CURRENT_TARGET: Target = compile_error!("In order to use messaging activate either of the necessary features.");
        }
    }

    pub fn new(task: Task, target: Target, message_kind: MessageKind) -> Self {
        Self::builder(task, target, message_kind).build()
    }

    pub fn builder(task: Task, target: Target, message_kind: MessageKind) -> MessageBuilder {
        MessageBuilder::new(task, target, Self::CURRENT_TARGET, message_kind)
    }

    pub fn to_string(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

#[cfg(feature = "extension")]
impl Message {
    pub fn to_value(&self) -> Result<Object, JsValue> {
        Ok(serde_wasm_bindgen::to_value(self)
            .map_err(map_err!())?
            .unchecked_into())
    }
}

#[cfg(feature = "background")]
impl Message {
    /// Merges two json objects together by assigning values from b into a.
    pub fn merge_json_objects(a: &mut Value, b: Value) {
        if let Value::Object(a) = a {
            if let Value::Object(b) = b {
                for (k, v) in b {
                    if v.is_null() {
                        a.remove(&k);
                    } else {
                        Self::merge_json_objects(a.entry(k).or_insert(Value::Null), v);
                    }
                }

                return;
            }
        }

        *a = b;
    }
}

#[cfg(feature = "popup")]
impl Message {
    pub async fn execute(&self) -> Result<(), JsValue> {
        match self.target {
            Target::Background => {
                let message = self.to_value()?;

                browser()
                    .runtime()
                    .send_message(&message)
                    .await
                    .map_err(map_err!())?;
            }
            _ => todo!(),
        }

        Ok(())
    }
}

#[cfg(any(feature = "backend", feature = "background"))]
impl TryFrom<WsMessage> for Message {
    type Error = WsMessage;

    fn try_from(value: WsMessage) -> Result<Self, Self::Error> {
        let WsMessage::Text(txt) = &value else {
            return Err(value);
        };

        serde_json::from_str(txt).map_err(|_| value)
    }
}

#[cfg(any(feature = "backend", feature = "background"))]
impl Message {
    pub fn into_ws_message(&self) -> serde_json::Result<WsMessage> {
        self.to_string().map(|msg| WsMessage::Text(msg.into()))
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Message");

        let Message {
            task,
            target,
            sender,
            kind,
            access_token,
            refresh_token,
            // cid,
            username,
            premium,
            settings,
            #[cfg(any(feature = "foreground", feature = "background"))]
            cookie,
            #[cfg(any(feature = "backend", feature = "background"))]
            session_scope,
            log_out,
            code,
            error,
            #[cfg(any(feature = "popup", feature = "background"))]
            popup,
        } = &self;

        debug_struct.field("task", &task);
        debug_struct.field("target", &target);
        debug_struct.field("sender", &sender);
        debug_struct.field("kind", &kind);

        if let Some(access_token) = access_token.as_ref() {
            debug_struct.field("access_token", access_token);
        }
        if let Some(refresh_token) = refresh_token.as_ref() {
            debug_struct.field("refresh_token", refresh_token);
        }
        // if let Some(cid) = cid.as_ref() {
        //     debug_struct.field("cid", cid);
        // }
        if let Some(username) = username.as_ref() {
            debug_struct.field("username", username);
        }
        if let Some(premium) = premium.as_ref() {
            debug_struct.field("premium", premium);
        }
        if let Some(settings) = settings.as_ref() {
            debug_struct.field("settings", settings);
        }
        #[cfg(any(feature = "backend", feature = "background"))]
        if let Some(session_scope) = session_scope.as_ref() {
            debug_struct.field("session_scope", session_scope);
        }
        if let Some(log_out) = log_out.as_ref() {
            debug_struct.field("log_out", log_out);
        }
        if let Some(code) = code.as_ref() {
            debug_struct.field("code", code);
        }
        if let Some(error) = error.as_ref() {
            debug_struct.field("error", error);
        }
        #[cfg(any(feature = "popup", feature = "background"))]
        if let Some(popup) = popup.as_ref() {
            debug_struct.field("popup", popup);
        }
        #[cfg(any(feature = "foreground", feature = "background"))]
        if let Some(cookie) = cookie.as_ref() {
            debug_struct.field("cookie", cookie);
        }

        debug_struct.finish()
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Premium {
    pub exp: u64,
    pub neon: bool,
    pub animation: bool,
    pub antyduch: bool,
}

#[cfg(feature = "backend")]
impl Premium {
    pub fn new(exp: u64, neon: bool, animation: bool, antyduch: bool) -> Self {
        Self {
            exp,
            neon,
            animation,
            antyduch,
        }
    }
}

// TODO: Better name ?
#[cfg(any(feature = "popup", feature = "background"))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PopupUpdate {
    pub state: Option<PopupState>,
    pub msg: Option<PopupMessage>,
}

#[cfg(any(feature = "popup", feature = "background"))]
impl PopupUpdate {
    pub fn join_discord() -> Self {
        Self {
            state: Some(PopupState::JoinDiscord),
            msg: Some(PopupMessage::LoginFailed { reason: None }),
        }
    }
}

#[cfg(any(feature = "popup", feature = "background"))]
impl From<PopupState> for PopupUpdate {
    fn from(value: PopupState) -> Self {
        Self {
            state: Some(value),
            msg: None,
        }
    }
}

#[cfg(any(feature = "popup", feature = "background"))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PopupState {
    LoggingIn = 0,
    LoggingOut,
    LoggedIn,
    LoggedOut,
    JoinDiscord,
}

#[cfg(any(feature = "popup", feature = "background"))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PopupMessage {
    LoginFailed { reason: Option<String> },
    RefreshAfterLogin,
}

#[cfg(any(feature = "popup", feature = "background"))]
impl PopupMessage {
    pub fn login_failed(reason: String) -> Self {
        Self::LoginFailed {
            reason: Some(reason),
        }
    }
}

#[cfg(any(feature = "foreground", feature = "background"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub details: Option<CookieDetails>,
    pub value: Option<String>,
}

#[cfg(any(feature = "foreground", feature = "background"))]
impl Cookie {
    #[cfg(feature = "foreground")]
    pub fn request(details: CookieDetails) -> Self {
        Self {
            details: Some(details),
            value: None,
        }
    }

    #[cfg(feature = "background")]
    pub fn response(value: String) -> Self {
        Self {
            details: None,
            value: Some(value),
        }
    }
}

#[cfg(any(feature = "foreground", feature = "background"))]
impl TryInto<cookies::Cookie> for Cookie {
    type Error = Self;

    fn try_into(mut self) -> Result<cookies::Cookie, Self::Error> {
        Ok(cookies::Cookie {
            value: self.value.take().ok_or_else(|| self)?,
        })
    }
}

#[skip_serializing_none]
#[derive(Serialize)]
pub struct MessageBuilder {
    task: Task,
    target: Target,
    sender: Target,
    kind: MessageKind,
    access_token: Option<String>,
    refresh_token: Option<String>,
    // cid: Option<Simple>,
    username: Option<String>,
    premium: Option<Premium>,
    settings: Option<Value>,
    #[cfg(any(feature = "foreground", feature = "background"))]
    pub cookie: Option<Cookie>,
    #[cfg(any(feature = "backend", feature = "background"))]
    session_scope: Option<SessionScope>,
    log_out: Option<LogOutDetails>,
    code: Option<String>,
    error: Option<String>,
    #[cfg(any(feature = "popup", feature = "background"))]
    popup: Option<PopupUpdate>,
}

impl MessageBuilder {
    pub const fn new(
        task: Task,
        target: Target,
        sender: Target,
        message_kind: MessageKind,
    ) -> Self {
        Self {
            task,
            target,
            sender,
            kind: message_kind,
            access_token: None,
            refresh_token: None,
            // cid: None,
            username: None,
            premium: None,
            settings: None,
            #[cfg(any(feature = "foreground", feature = "background"))]
            cookie: None,
            #[cfg(any(feature = "backend", feature = "background"))]
            session_scope: None,
            log_out: None,
            code: None,
            error: None,
            #[cfg(any(feature = "popup", feature = "background"))]
            popup: None,
        }
    }

    pub fn access_token(mut self, access_token: String) -> Self {
        self.access_token = Some(access_token);
        self
    }

    pub fn refresh_token(mut self, refresh_token: String) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }

    pub fn maybe_refresh_token(mut self, refresh_token: Option<String>) -> Self {
        self.refresh_token = refresh_token;
        self
    }

    // pub const fn cid(mut self, cid: Simple) -> Self {
    //     self.cid = Some(cid);
    //     self
    // }

    pub const fn premium(mut self, premium: Premium) -> Self {
        self.premium = Some(premium);
        self
    }

    #[cfg(any(feature = "foreground", feature = "background"))]
    pub fn cookie(mut self, cookie: Cookie) -> Self {
        self.cookie = Some(cookie);
        self
    }

    pub const fn log_out(mut self, log_out: LogOutDetails) -> Self {
        self.log_out = Some(log_out);
        self
    }

    pub const fn maybe_premium(mut self, premium: Option<Premium>) -> Self {
        self.premium = premium;
        self
    }

    #[cfg(any(feature = "popup", feature = "background"))]
    pub fn popup<B: Into<PopupUpdate>>(mut self, popup: B) -> Self {
        self.popup = Some(popup.into());
        self
    }

    pub fn code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    pub fn maybe_error<A: ToString>(mut self, error: Option<A>) -> Self {
        self.error = error.as_ref().map(ToString::to_string);
        self
    }

    pub fn error<A: ToString>(mut self, error: A) -> Self {
        self.error = Some(error.to_string());
        self
    }

    #[cfg(any(feature = "backend", feature = "background"))]
    pub fn session_scope(mut self, session_scope: SessionScope) -> Self {
        self.session_scope = Some(session_scope);
        self
    }

    pub fn username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    pub fn maybe_username(mut self, username: Option<String>) -> Self {
        self.username = username;
        self
    }

    // pub(crate) fn maybe_settings(mut self, settings: Option<Value>) -> Self {
    //     self.settings = settings;
    //     self
    // }

    pub fn build(self) -> Message {
        Message {
            task: self.task,
            target: self.target,
            sender: self.sender,
            kind: self.kind,
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            // cid: self.cid,
            username: self.username,
            premium: self.premium,
            settings: self.settings,
            #[cfg(any(feature = "foreground", feature = "background"))]
            cookie: self.cookie,
            #[cfg(any(feature = "backend", feature = "background"))]
            session_scope: self.session_scope,
            log_out: self.log_out,
            code: self.code,
            error: self.error,
            #[cfg(any(feature = "popup", feature = "background"))]
            popup: self.popup,
        }
    }
}
