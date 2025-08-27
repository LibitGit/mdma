use std::{fmt, sync::OnceLock};

use common::{debug_log, err_code, map_err, messaging::prelude::*, sleep, web_extension_sys::browser};
use futures::{SinkExt, StreamExt, channel::mpsc, stream::SplitStream};
use gloo_net::websocket::futures::WebSocket;
use port::{PORT_DISPATCHER_TX, PortDispatcher};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{
    connection::Connection,
    console_error,
    types::{MessageExt, StorageRefreshToken},
};

pub mod port;

// TODO: rename to sth like `RUNTIME_TX` since that's what it is.
static RUNTIME_TX: OnceLock<mpsc::UnboundedSender<Message>> = OnceLock::new();
pub static SOCKET_TX: OnceLock<mpsc::UnboundedSender<Message>> = OnceLock::new();

pub struct RuntimeDispatcher {
    rx: mpsc::UnboundedReceiver<Message>,
}

impl RuntimeDispatcher {
    pub fn recv(&mut self) -> futures::stream::Next<'_, mpsc::UnboundedReceiver<Message>> {
        self.rx.next()
    }

    // TODO: This is not blocking in wasm.
    /// Like [`RuntimeDispatcher::recv`], but will not block longer than
    /// `timeout`. Returns:
    ///  * `Ok(message)` if there was a message in the channel before the
    ///    timeout was reached.
    ///  * `Err(Timeout)` if no message arrived on the channel before the
    ///    timeout was reached.
    ///  * `Err(Disconnected)` when channel is closed and no messages left in
    ///    the queue.
    pub async fn recv_timeout(&mut self, timeout: u32) -> Result<Message, RecvTimeoutError> {
        futures::select! {
            res = self.rx.next() => res.ok_or(RecvTimeoutError::Disconnected),
            _ = sleep(timeout) => Err(RecvTimeoutError::Timeout),
        }
    }
}

/// An error returned when failing to receive a message in a method that
/// block/wait for a message for a while, but has a timeout after which it gives
/// up.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum RecvTimeoutError {
    /// No message arrived on the channel before the timeout was reached. The
    /// channel is still open.
    Timeout,

    /// The channel is closed. Either the sender was dropped before sending any
    /// message, or the message has already been extracted from the
    /// receiver.
    Disconnected,
}

impl fmt::Display for RecvTimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            RecvTimeoutError::Timeout => "timed out waiting on channel",
            RecvTimeoutError::Disconnected => "channel is closed and no messages left in the queue",
        };

        msg.fmt(f)
    }
}

impl RuntimeDispatcher {
    fn new(rx: mpsc::UnboundedReceiver<Message>) -> Self {
        Self { rx }
    }

    async fn run_event_loop(&mut self, state: &'static Connection) -> Result<(), JsValue> {
        let msg = self.recv().await.ok_or_else(|| err_code!())?;

        if msg.target != Target::Background || msg.kind != MessageKind::Request {
            return Err(err_code!());
        }

        if msg.sender == Target::Foreground {
            todo!()
        }

        Self::dispatch_popup_message(msg, state).await
    }

    async fn dispatch_popup_message(
        msg: Message,
        state: &'static Connection,
    ) -> Result<(), JsValue> {
        match msg.task {
            Task::UserData => {
                let user_opt = state.user.borrow();
                let username = user_opt.as_ref().map(|user| user.nick.clone());
                let premium = user_opt.as_ref().and_then(|user| user.premium.clone());
                debug_log!("SENDING USER DATA");
                // TODO: Is this correct ?
                let state = match username.is_some() {
                    true => PopupState::LoggedIn,
                    false => PopupState::LoggedOut,
                };

                drop(user_opt);

                Message::builder(Task::UserData, Target::Popup, MessageKind::Response)
                    .maybe_username(username)
                    .maybe_premium(premium)
                    .popup(state)
                    .build()
                    .execute()
                    .await?
            }
            Task::LogOut => {
                // TODO: If a session is active more work needs to be done.
                state.user.borrow_mut().take();
                browser()
                    .storage()
                    .sync()
                    .remove(&JsValue::from_str(StorageRefreshToken::KEY))
                    .await
                    .map_err(map_err!())?;

                Message::builder(Task::LogOut, Target::Backend, MessageKind::Request)
                    .log_out(msg.log_out.ok_or_else(|| err_code!())?)
                    .build()
                    .execute()
                    .await?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}

pub struct SocketDispatcher {
    tx: mpsc::UnboundedSender<Message>,
    rx: SplitStream<WebSocket>,
}

impl SocketDispatcher {
    pub async fn send(&mut self, item: Message) -> Result<(), mpsc::SendError> {
        self.tx.send(item).await
    }

    pub async fn recv(&mut self) -> Option<Result<Message, JsValue>> {
        self.rx.next().await.map(|res| {
            res.map_err(map_err!(from))
                .and_then(|msg| Message::try_from(msg).map_err(|_| err_code!()))
        })
    }
}

impl SocketDispatcher {
    fn new(tx: mpsc::UnboundedSender<Message>, rx: SplitStream<WebSocket>) -> Self {
        Self { tx, rx }
    }

    async fn run_event_loop(&mut self, _state: &'static Connection) -> Result<(), JsValue> {
        let msg = self.recv().await.ok_or_else(|| err_code!())??;

        if msg.target != Target::Background || msg.kind != MessageKind::Request {
            return Err(err_code!());
        }

        Ok(())
    }
}

pub struct Dispatcher {
    pub runtime: RuntimeDispatcher,
    pub port: PortDispatcher,
    pub socket: SocketDispatcher,
}

impl Dispatcher {
    // TODO: Enum describing the sender and a single method for this?
    pub async fn dispatch_from_runtime(item: Message) -> Result<(), mpsc::SendError> {
        RUNTIME_TX.wait().send(item).await
    }

    // TODO: Enum describing the sender and a single method for this?
    pub async fn dispatch_from_port(item: Message) -> Result<(), mpsc::SendError> {
        PORT_DISPATCHER_TX.wait().send(item).await
    }

    /// Opens the popup window.
    pub async fn open_popup() -> Result<(), JsValue> {
        let window_id = crate::FOREGROUND_PORT
            .with_borrow(|port| {
                port.as_ref()
                    .ok_or_else(|| err_code!())?
                    .sender()
                    .ok_or_else(|| err_code!())
            })?
            .tab()
            .ok_or_else(|| err_code!())?
            .window_id();
        let window = browser()
            .windows()
            .get(window_id)
            .await
            .map_err(map_err!())?;

        if !window.focused() {
            let info = serde_wasm_bindgen::to_value(&WindowUpdateInfo::new(true))
                .map_err(map_err!(from))?
                .unchecked_into();

            browser()
                .windows()
                .update(window_id, &info)
                .await
                .map_err(map_err!())?;
        }

        let open_popup_options = serde_wasm_bindgen::to_value(&OpenPopupOptions::new(window_id))
            .map_err(map_err!(from))?
            .unchecked_into();

        browser()
            .action()
            .open_popup_with_options(&open_popup_options)
            .await
            .map_err(map_err!())?;

        Ok(())
    }
}

impl Dispatcher {
    const SOCKET_URL: &str = match option_env!("SOCKET_URL") {
        Some(url) => url,
        None => "ws://localhost:3000/ws",
    };

    fn _new(runtime: RuntimeDispatcher, port: PortDispatcher, socket: SocketDispatcher) -> Self {
        Self {
            runtime,
            port,
            socket,
        }
    }

    pub(super) fn init() -> Result<Self, JsValue> {
        let (mut socket_tx, socket_rx) = WebSocket::open(Self::SOCKET_URL)
            .map_err(map_err!(from))?
            .split();

        // State machine for sending messages via socket tx.
        let (new_socket_tx, mut rx) = mpsc::unbounded::<Message>();

        SOCKET_TX
            .set(new_socket_tx.clone())
            .map_err(|_| err_code!())?;

        wasm_bindgen_futures::spawn_local(async move {
            while let Some(msg) = rx.next().await {
                let msg = match msg.into_ws_message().map_err(map_err!(from)) {
                    Ok(msg) => msg,
                    Err(err_code) => return console_error!(err_code),
                };

                if let Err(err_code) = socket_tx.send(msg).await.map_err(map_err!(from)) {
                    console_error!(err_code)
                }
            }
        });

        let (tx, rx) = mpsc::unbounded();

        RUNTIME_TX.set(tx).map_err(|_| err_code!())?;

        let runtime = RuntimeDispatcher::new(rx);
        let (tx, rx) = mpsc::unbounded();

        PORT_DISPATCHER_TX.set(tx).map_err(|_| err_code!())?;

        let port = PortDispatcher::new(rx);
        let socket = SocketDispatcher::new(new_socket_tx, socket_rx);
        Ok(Self::_new(runtime, port, socket))
    }

    // TODO: loop { select! between rx and some var stopping execution }.
    pub(super) fn spawn_event_loop(self, state: &'static Connection) {
        let mut runtime = self.runtime;
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                if let Err(err_code) = runtime.run_event_loop(state).await {
                    return console_error!(err_code);
                }
            }
        });

        let mut socket = self.socket;
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                if let Err(err_code) = socket.run_event_loop(state).await {
                    return console_error!(err_code);
                }
            }
        });

        let mut port = self.port;
        let validator = MessageValidator::builder(Target::Foreground)
            .maybe_kind(None)
            .build();
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                if let Err(err_code) = port.run_event_loop(state, &validator).await {
                    return console_error!(err_code);
                }
            }
        });
    }
}

#[derive(Serialize)]
struct WindowUpdateInfo {
    focused: bool,
}

impl WindowUpdateInfo {
    fn new(focused: bool) -> Self {
        Self { focused }
    }
}

#[derive(Serialize)]
struct OpenPopupOptions {
    #[serde(rename = "windowId")]
    window_id: i32,
}

impl OpenPopupOptions {
    fn new(window_id: i32) -> Self {
        Self { window_id }
    }
}
