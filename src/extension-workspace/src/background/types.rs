use std::collections::VecDeque;

use common::{
    UnwrapJsExt, debug_log, err_code, map_err, messaging::prelude::*, sleep,
    web_extension_sys::browser,
};
use futures::{SinkExt, channel::mpsc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use wasm_bindgen::{intern, prelude::*};

use crate::{FOREGROUND_PORT, TASK_QUEUE, console_error, dispatcher::SOCKET_TX};

// TODO: Better name.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct StorageRefreshToken {
    #[serde(rename = "refresh_token")]
    pub token: Option<Jwt>,
}

impl StorageRefreshToken {
    pub const KEY: &str = "refresh_token";

    pub fn new(token: Jwt) -> Self {
        Self { token: Some(token) }
    }
}

#[derive(Debug)]
pub struct AuthResponse {
    pub code: String,
}

impl AuthResponse {
    pub fn new(code: String) -> Self {
        Self { code }
    }

    pub fn from_url(url: String) -> Option<Self> {
        let query_start = url.find('?')?;
        let query_string = &url[query_start + 1..];

        let mut code = None;

        if let Some((key, value)) = query_string.split_once('=') {
            match key {
                "code" => code = Some(value.to_string()),
                _ => {}
            }
        }

        Some(Self::new(code?))
    }
}

pub struct TaskQueue {
    queue: VecDeque<Message>,
    rx: mpsc::UnboundedReceiver<Message>,
    tx: mpsc::UnboundedSender<Message>,
}

impl TaskQueue {
    pub(super) fn new() -> Self {
        let (tx, rx) = mpsc::unbounded::<Message>();

        Self {
            queue: VecDeque::with_capacity(10),
            rx,
            tx,
        }
    }

    pub(super) fn init() {
        wasm_bindgen_futures::spawn_local(async {
            loop {
                let to_send_opt = TASK_QUEUE.with_borrow_mut(|task_queue| {
                    while let Ok(Some(msg)) = task_queue.rx.try_next() {
                        debug_log!("Task received!", msg.to_value().unwrap_js());
                        task_queue.update(msg);
                    }

                    task_queue.queue.pop_front()
                });

                if let Some(task) = to_send_opt {
                    if let Err(err_code) = task.execute().await {
                        console_error!(err_code)
                    }
                }

                sleep(150).await;
            }
        });
    }

    //TODO: Remove duplicate tasks ?
    fn update(&mut self, msg: Message) {
        // queue: &mut VecDeque<Message>, msg: Message
        if msg.task != Task::UserData {
            self.queue.push_back(msg);
            return;
        }

        //If task is sending addon data settings try merge them into one task.
        let Some(old_settings_task) = self
            .queue
            .iter_mut()
            .find(|queued_msg| queued_msg.task == Task::UserData)
        else {
            self.queue.push_back(msg);
            return;
        };
        let old_settings = old_settings_task.settings.as_mut().unwrap_js();
        let new_settings = msg.settings.unwrap_js();

        Message::merge_json_objects(old_settings, new_settings);
        debug_log!("Settings task updated!");
    }
}

#[derive(Debug, PartialEq)]
pub enum ExecutionError {
    NoReceiver,
    JsError(JsValue),
}

impl From<JsValue> for ExecutionError {
    fn from(value: JsValue) -> Self {
        match value.dyn_ref::<js_sys::Error>().is_some_and(|err| {
            err.message() == intern("Could not establish connection. Receiving end does not exist.")
        }) {
            true => Self::NoReceiver,
            false => Self::JsError(value),
        }
    }
}

impl From<ExecutionError> for JsValue {
    fn from(value: ExecutionError) -> Self {
        match value {
            ExecutionError::JsError(val) => val,
            ExecutionError::NoReceiver => {
                js_sys::Error::new("Could not establish connection. Receiving end does not exist.")
                    .into()
            }
        }
    }
}

pub(crate) trait MessageExt {
    async fn execute(self) -> Result<(), JsValue>;
    fn enqueue(self) -> Result<(), JsValue>;
}

impl MessageExt for Message {
    async fn execute(self) -> Result<(), JsValue> {
        match self.target {
            Target::Backend => {
                // if SOCKET.with(|ws| ws.ready_state()) == 0 {
                //     Self::wait_for_condition(
                //         || SOCKET.with(|ws| ws.ready_state()) == 1,
                //         100,
                //         10_000,
                //     )
                //     .await;
                // }

                debug_log!("Sending task to backend: ", self.to_value().unwrap_js());

                SOCKET_TX.wait().send(self).await.map_err(map_err!(from))?;
            }
            Target::Popup => {
                let message = self.to_value()?;

                debug_log!(@f "Sending message to popup: {self:?}");

                // Doesn't error when NoReceiver was returned.
                if let Err(ExecutionError::JsError(val)) = browser()
                    .runtime()
                    .send_message(&message)
                    .await
                    .map_err(Into::into)
                {
                    return Err(val).map_err(map_err!());
                }
            }
            Target::Foreground => {
                let port = FOREGROUND_PORT
                    .with_borrow(Option::clone)
                    .ok_or_else(|| err_code!())?;
                let message = self.to_value()?;
                port.post_message(&message);
            }
            Target::Background => unreachable!(),
        }

        Ok(())
    }

    fn enqueue(self) -> Result<(), JsValue> {
        TASK_QUEUE.with_borrow_mut(|task_queue| {
            task_queue.tx.unbounded_send(self).map_err(map_err!(from))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Jwt(String);

impl Jwt {
    const CLAIMS_BUFFER_SIZE: usize = 128;

    pub const fn new(token: String) -> Self {
        Self(token)
    }

    /// Returns true if the String is a valid JWT and isn't expired.
    pub fn validate(&self) -> bool {
        use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};

        let Some(encoded_token_claims) = self.0.split('.').nth(1) else {
            return false;
        };

        debug_log!(
            "decoded len estimate:",
            base64::decoded_len_estimate(encoded_token_claims.len())
        );

        let mut claims = [0u8; Self::CLAIMS_BUFFER_SIZE];
        let Ok(decoded_len) =
            STANDARD_NO_PAD.decode_slice_unchecked(encoded_token_claims, &mut claims)
        else {
            return false;
        };
        let claims = unsafe { std::str::from_utf8_unchecked(&claims[..decoded_len]) };

        #[derive(Debug, Deserialize)]
        struct Claims {
            exp: u64,
        }

        let Ok(claims) = serde_json::from_str::<Claims>(claims) else {
            return false;
        };

        js_sys::Date::new_0().get_time() as u64 / 1000 < claims.exp
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}
