use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
};

use common::{
    map_err,
    web_extension_sys::cookies::{Cookie, CookieDetails},
};
use futures::channel::oneshot;
use js_sys::Object;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_wasm_bindgen::Serializer;
use serde_with::skip_serializing_none;
use wasm_bindgen::prelude::*;

use crate::{
    interface::{init_windows_layer, render_widget},
    prelude::message,
    s,
    utils::{JsResult, UnwrapJsExt, document},
};

thread_local! {
    pub(crate) static PENDING_REQUESTS: RefCell<HashMap<Tasks, VecDeque<oneshot::Sender<Task>>>> = RefCell::new(HashMap::new())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
// TODO: Rename to TaskType
pub(crate) enum Tasks {
    UserData,
    AddonData,
    Cookie,
    SuccessfulConnection,
    OpenPopup,
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
    pub(crate) cookie_details: Option<CookieDetails>,
    pub(crate) cookie_data: Option<Cookie>,
    pub(crate) uuid: Option<String>,
    pub(crate) username: Option<String>,
    pub(crate) access_level: Option<u8>,
    pub(crate) settings: Option<Value>,
    #[cfg(feature = "antyduch")]
    pub(crate) key: Option<char>,
}

impl Task {
    pub const UNAUTHORIZED_ACCESS_LEVEL: u8 = 0;

    pub(crate) const fn builder() -> TaskBuilder {
        TaskBuilder::new()
    }

    pub(crate) const fn type_(&self) -> &Tasks {
        &self.task
    }

    #[inline]
    pub(crate) fn to_value(&self) -> JsResult<Object> {
        Ok(self
            .serialize(&Serializer::json_compatible())
            .map_err(map_err!())?
            .unchecked_into())
    }
}

#[derive(Debug, Default)]
pub(crate) struct TaskBuilder {
    task: Option<Tasks>,
    target: Option<Targets>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    extension_id: Option<String>,
    cookie_details: Option<CookieDetails>,
    uuid: Option<String>,
    username: Option<String>,
    access_level: Option<u8>,
    settings: Option<Value>,
    #[cfg(feature = "antyduch")]
    pub(crate) key: Option<char>,
}

impl TaskBuilder {
    const fn new() -> Self {
        Self {
            task: None,
            target: None,
            access_token: None,
            refresh_token: None,
            extension_id: None,
            cookie_details: None,
            uuid: None,
            username: None,
            access_level: None,
            settings: None,
            #[cfg(feature = "antyduch")]
            key: None,
        }
    }

    pub(crate) const fn task(mut self, task: Tasks) -> Self {
        self.task = Some(task);
        self
    }

    pub(crate) const fn target(mut self, target: Targets) -> Self {
        self.target = Some(target);
        self
    }

    pub(crate) fn cookie_details(mut self, cookie_details: CookieDetails) -> Self {
        self.cookie_details = Some(cookie_details);
        self
    }

    pub(crate) fn settings(mut self, settings: Value) -> Self {
        self.settings = Some(settings);
        self
    }

    #[cfg(feature = "antyduch")]
    pub(crate) const fn key(mut self, key: char) -> Self {
        self.key = Some(key);
        self
    }

    pub(crate) fn build(self) -> Result<Task, JsError> {
        let TaskBuilder {
            task,
            target,
            access_token,
            refresh_token,
            extension_id,
            cookie_details,
            uuid,
            username,
            access_level,
            settings,
            #[cfg(feature = "antyduch")]
            key,
        } = self;

        Ok(Task {
            task: task.ok_or(JsError::new("Missing field: task"))?,
            target: target.ok_or(JsError::new("Missing field: task"))?,
            access_token,
            refresh_token,
            extension_id,
            cookie_details,
            cookie_data: None,
            uuid,
            username,
            access_level,
            settings,
            #[cfg(feature = "antyduch")]
            key,
        })
    }
}
