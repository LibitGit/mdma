use std::cell::RefCell;
use std::collections::VecDeque;

use common::messaging::prelude::*;
use common::web_extension_sys::cookies::{Cookie, CookieDetails};
use common::{UnwrapJsExt, err_code, map_err, throw_err_code};
use common::{debug_log, web_extension_sys::browser};
use futures::channel::mpsc::UnboundedSender;
use futures::channel::oneshot;
use js_sys::{Object, Reflect};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_wasm_bindgen::Serializer;
use serde_with::skip_serializing_none;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::{ PENDING_REQUESTS, USER_DATA, global};

const TOKEN_CLAIMS_BUFFER_SIZE: usize = 128;

thread_local! {
    pub(crate) static TASK_QUEUE: RefCell<VecDeque<Message>> = const { RefCell::new(VecDeque::new()) };
    pub(crate) static DISPATCHER: RefCell<UnboundedSender<Message>> = throw_err_code!("accessing dispatcher before set");
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Tasks {
    Tokens,
    UserData,
    AddonData,
    Uuid,
    Login,
    CancelLogin,
    Cookie,
    OpenPopup,
    SuccessfulConnection,
    AttachDebugger,
    DetachDebugger,
    KeyDown,
    KeyUp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum Targets {
    Backend,
    Background,
    Middleground,
    Foreground,
    Popup,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Task {
    task: Tasks,
    target: Targets,
    pub(crate) access_token: Option<String>,
    pub(crate) refresh_token: Option<String>,
    pub(crate) extension_id: Option<String>,
    pub(crate) uuid: Option<String>,
    pub(crate) cookie_details: Option<CookieDetails>,
    pub(crate) cookie_data: Option<Cookie>,
    pub(crate) username: Option<String>,
    pub(crate) access_level: Option<u8>,
    pub(crate) settings: Option<Value>,
    pub(crate) key: Option<char>,
}

impl Task {
    #[inline]
    pub(crate) fn builder() -> TaskBuilder {
        TaskBuilder::default()
    }

    pub(crate) fn type_(&self) -> &Tasks {
        &self.task
    }

    #[inline]
    pub(crate) fn empty_response(task: Tasks) -> Self {
        Self {
            task,
            target: Targets::Background,
            access_token: None,
            refresh_token: None,
            extension_id: None,
            uuid: None,
            username: None,
            access_level: None,
            settings: None,
            cookie_data: None,
            cookie_details: None,
            key: None,
        }
    }

    #[inline]
    pub(crate) fn to_string(&self) -> Result<String, JsValue> {
        serde_json::to_string(self).map_err(map_err!(from))
    }

    pub(crate) fn redirect(self, new_target: Targets) -> Self {
        Self {
            target: new_target,
            ..self
        }
    }

    #[inline]
    pub(crate) fn to_value(&self) -> Result<Object, JsError> {
        match self.serialize(&Serializer::json_compatible()) {
            Ok(task) => Ok(task.unchecked_into()),
            Err(err) => Err(JsError::from(err)),
        }
    }

    pub(crate) async fn wait_for_condition(
        condition: impl Fn() -> bool,
        interval: u32,
        timeout: u32,
    ) -> bool {
        let mut i = 0;
        while i < timeout {
            Self::delay(interval).await;
            i += interval;

            if condition() {
                return true;
            }
        }

        false
    }

    pub(crate) async fn delay(ms: u32) -> bool {
        use js_sys::Promise;

        let promise = Promise::new(&mut |resolve, _| {
            let _ =
                global().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
        });
        let _ = JsFuture::from(promise).await;
        true
    }

    // pub(crate) async fn delay_range(min: usize, max: usize) {
    //     let promise = js_sys::Promise::new(&mut |resolve, _| {
    //         let timeout = min as f64 + js_sys::Math::random() * ((max - min) as
    // f64);         let _ = global()
    //             .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve,
    // timeout as i32);     });
    //     let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    // }

    pub(crate) fn merge_json(a: &mut Value, b: Value) {
        if let Value::Object(a) = a {
            if let Value::Object(b) = b {
                for (k, v) in b {
                    if v.is_null() {
                        a.remove(&k);
                    } else {
                        Self::merge_json(a.entry(k).or_insert(Value::Null), v);
                    }
                }

                return;
            }
        }

        *a = b;
    }

    pub(crate) fn enqueue(self) -> Result<(), JsValue> {
        DISPATCHER
            .try_with(|cell| {
                cell.try_borrow_mut()
                    .map_err(map_err!(from))?
                    .unbounded_send(self)
                    .map_err(map_err!(from))
            })
            .map_err(map_err!(from))?
    }

    pub(crate) async fn execute(&self) -> Result<(), JsValue> {
        match self.target {
            Targets::Backend => {
                if SOCKET.with(|ws| ws.ready_state()) == 0 {
                    Self::wait_for_condition(|| SOCKET.with(|ws| ws.ready_state()) == 1, 100, 10_000)
                        .await;
                }

                let data = self.to_string()?;
                debug_log!("Sending task to backend: ", self.to_value().unwrap_js());
                SOCKET.with(|ws| ws.send_with_str(&data)).map_err(map_err!())
            }
            Targets::Popup | Targets::Middleground => {
                let message = self.to_value()?;
                debug_log!(&message);
                browser()
                    .runtime()
                    .send_message(&message)
                    .await
                    .map_err(map_err!())?;
                Ok(())
            }
            Targets::Foreground => FOREGROUND_PORT.with_borrow(|port| {
                let port = port.as_ref().ok_or_else(|| err_code!())?;
                let message = self.to_value().map_err(map_err!())?;
                port.post_message(&message);
                Ok(())
            }),
            Targets::Background => todo!(),
        }
    }

    fn validate_token_exp(token: &str) -> Result<bool, JsValue> {
        use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};

        let encoded_token_claims = token.split('.').nth(1).ok_or_else(|| err_code!())?;

        debug_log!(
            "decoded len estimate:",
            base64::decoded_len_estimate(encoded_token_claims.len())
        );

        let mut claims = [0u8; TOKEN_CLAIMS_BUFFER_SIZE];
        let decoded_len = STANDARD_NO_PAD
            .decode_slice_unchecked(encoded_token_claims, &mut claims)
            .map_err(map_err!(from))?;
        let claims = unsafe { std::str::from_utf8_unchecked(&claims[..decoded_len]) };
        let claims: Value = serde_json::from_str(claims).map_err(map_err!(from))?;
        let exp = claims
            .get("exp")
            .and_then(|exp| exp.as_u64())
            .ok_or_else(|| err_code!())?;

        Ok((js_sys::Date::new_0().get_time() as u64 / 1000) < exp)
    }

    pub(crate) async fn fetch_new_tokens(refresh_token: String) -> Result<Self, JsValue> {
        if !Self::validate_token_exp(&refresh_token)? {
            return Ok(Self::empty_response(Tasks::Tokens));
        }

        let (sender, receiver) = oneshot::channel::<Self>();

        PENDING_REQUESTS.with_borrow_mut(|pending| pending.insert(Tasks::Tokens, sender));

        Self::builder()
            .task(Tasks::Tokens)
            .target(Targets::Backend)
            .refresh_token(refresh_token)
            .build()?
            .execute()
            .await?;

        receiver.await.map_err(map_err!(from))
    }

    //TODO: Rename since it does not always fetch.
    //TODO: After checking in user data check in chrome.storage.local if nothing is
    //there get the data with addon data.
    //TODO: Other or separate task/method for getting user data without addon_data
    // ?
    pub(crate) async fn fetch_user_data() -> Result<Self, JsValue> {
        let from_user_data = USER_DATA.with_borrow(|data| match data.access_token.as_ref() {
            Some(access_token) if Self::validate_token_exp(access_token).unwrap_or(false) => (
                data.username.clone(),
                data.access_level,
                data.settings.clone(),
            ),
            _ => (None, None, None),
        });

        if let (Some(username), Some(access_level), settings) = from_user_data {
            return Task::builder()
                .task(Tasks::UserData)
                .target(Targets::Background)
                .username(username)
                .access_level(access_level)
                .maybe_settings(settings)
                .build();
        }

        let access_token = Self::try_get_access_token().await?;

        debug_log!(&format!("access_token: {access_token:?}"));

        let (sender, receiver) = oneshot::channel::<Self>();

        PENDING_REQUESTS.with_borrow_mut(|pending| pending.insert(Tasks::UserData, sender));

        Self::builder()
            .task(Tasks::UserData)
            .target(Targets::Backend)
            .access_token_opt(access_token)
            .build()?
            .execute()
            .await?;

        receiver.await.map_err(map_err!(from))
    }

    pub(crate) async fn try_get_access_token() -> Result<Option<String>, JsValue> {
        match USER_DATA.with_borrow_mut(|data| data.access_token.clone()) {
            Some(token) if Self::validate_token_exp(&token)? => Ok(Some(token)),
            _ => {
                let Some(refresh_token) = Self::get_refresh_token().await? else {
                    return Ok(None);
                };

                Task::fetch_new_tokens(refresh_token)
                    .await
                    .map(|task| task.access_token)
            }
        }
    }

    async fn get_refresh_token() -> Result<Option<String>, JsError> {
        if let Some(refresh_token) = USER_DATA.with_borrow(|data| data.refresh_token.clone()) {
            return Ok(Some(refresh_token));
        }

        let item = browser()
            .storage()
            .sync()
            .get(&JsValue::from_str("refresh_token"))
            .await;
        let refresh_token =
            Reflect::get(&item, &JsValue::from_str("refresh_token")).map_err(|_err| {
                debug_log!(_err);
                JsError::new("Could not get \"refresh_token\"")
            })?;

        Ok(refresh_token.as_string())
    }

    pub(crate) async fn fetch_user_data_after_login(_task: Task) -> Result<Self, JsValue> {
        let (sender, receiver) = oneshot::channel::<Self>();

        PENDING_REQUESTS.with_borrow_mut(|pending| pending.insert(Task::Tokens, sender));
        //TODO: Is this correct ?
        //Await login finish, can be canceled when window gets closed before oauth
        // completes
        receiver.await.map_err(map_err!(from))?;

        Self::fetch_user_data().await
    }

    pub(crate) fn cancel_login() -> Result<(), JsValue> {
        match PENDING_REQUESTS.with_borrow_mut(|pending| pending.remove(&Task::Tokens)) {
            Some(_) => {
                debug_log!("Canceled login gracefully");
                Ok(())
            }
            None => Err(err_code!()),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct TaskBuilder {
    task: Option<Tasks>,
    target: Option<Targets>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    extension_id: Option<String>,
    uuid: Option<String>,
    cookie_details: Option<CookieDetails>,
    cookie_data: Option<Cookie>,
    username: Option<String>,
    access_level: Option<u8>,
    settings: Option<Value>,
    key: Option<char>,
}

impl TaskBuilder {
    pub(crate) fn task(mut self, task: Tasks) -> Self {
        self.task = Some(task);
        self
    }

    pub(crate) fn target(self, target: Targets) -> Self {
        Self {
            target: Some(target),
            ..self
        }
    }

    pub(crate) fn access_token(mut self, access_token: String) -> Self {
        self.access_token = Some(access_token);
        self
    }

    pub(crate) fn access_token_opt(self, access_token: Option<String>) -> Self {
        Self {
            access_token,
            ..self
        }
    }

    pub(crate) fn refresh_token(mut self, refresh_token: String) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }

    pub(crate) fn extension_id(self, extension_id: String) -> Self {
        Self {
            extension_id: Some(extension_id),
            ..self
        }
    }

    //pub(crate) fn uuid(mut self, uuid: String) -> Self {
    //    self.uuid = Some(uuid);
    //    self
    //}

    pub(crate) fn cookie_data(mut self, cookie_data: Cookie) -> Self {
        self.cookie_data = Some(cookie_data);
        self
    }

    pub(crate) fn username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    pub(crate) fn access_level(mut self, level: u8) -> Self {
        self.access_level = Some(level);
        self
    }

    pub(crate) fn settings(mut self, settings: Value) -> Self {
        self.settings = Some(settings);
        self
    }

    pub(crate) fn maybe_settings(mut self, settings: Option<Value>) -> Self {
        self.settings = settings;
        self
    }

    #[inline]
    pub(crate) fn to_value(&self) -> Result<Object, JsValue> {
        let value = self
            .serialize(&Serializer::json_compatible())
            .map_err(map_err!(from))?;

        Ok(value.unchecked_into())
    }

    pub(crate) fn build(self) -> Result<Task, JsValue> {
        let TaskBuilder {
            task,
            target,
            access_token,
            refresh_token,
            extension_id,
            uuid,
            username,
            access_level,
            settings,
            cookie_data,
            cookie_details,
            key,
        } = self;

        Ok(Task {
            task: task.ok_or_else(|| err_code!())?,
            target: target.ok_or_else(|| err_code!())?,
            access_token,
            refresh_token,
            extension_id,
            uuid,
            username,
            access_level,
            settings,
            cookie_details,
            cookie_data,
            key,
        })
    }
}
