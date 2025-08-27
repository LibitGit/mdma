// pub(crate) mod task;

use std::{
    cell::{Cell, LazyCell, RefCell},
    collections::VecDeque,
    sync::OnceLock,
};

use common::{
    closure, debug_log, err_code, map_err,
    messaging::prelude::*,
    sleep,
    web_extension_sys::{
        browser,
        cookies::{self, CookieDetails},
    },
};
use futures::{
    SinkExt, StreamExt,
    channel::{mpsc, oneshot},
    stream::Next,
};
use serde::Serialize;
use serde_json::{Value, json};
use wasm_bindgen::{intern, prelude::*};

use crate::{
    s,
    utils::{JsResult, UnwrapJsExt},
};

use super::{GlobalsError, addons::AddonName, hero::Hero};

static PORT: OnceLock<Port> = OnceLock::new();

const EXTENSION_ID: &str = match option_env!("EXTENSION_ID") {
    Some(id) => id,
    None => "edjblpkppjpgkhbacpjfghacgjlppomc",
};

thread_local! {
    static CONNECT_INFO: LazyCell<js_sys::Object> = const {
        LazyCell::new(|| {
            serde_wasm_bindgen::to_value(&ConnectInfo::new(intern(s!("foreground"))))
                .unwrap_js()
                .unchecked_into()
        })
    };
}

#[derive(Debug)]
pub struct Port {
    port: RefCell<common::web_extension_sys::runtime::port::Port>,
    tx: mpsc::UnboundedSender<Message>,
    pub rx: RefCell<mpsc::UnboundedReceiver<Message>>,
}

// SAFETY: no threads on wasm32.
unsafe impl Send for Port {}
unsafe impl Sync for Port {}

impl Port {
    pub fn get() -> &'static Self {
        PORT.wait()
    }

    pub async fn send(msg: &Message) -> JsResult<()> {
        // if !Self::get().active.get() {
        //     Self::reconnect().await;
        // }

        Self::post_message(msg.to_value()?.as_ref());

        Ok(())
    }
}

impl Port {
    /// Connect to the background runtime via a message port.
    ///
    /// # Errors
    /// If the user isn't authorized after the connection is made return an
    /// [`Unauthorized`][GlobalsError::Unauthorized] error.
    pub(super) async fn init_authorized() -> Result<(), GlobalsError> {
        let port = CONNECT_INFO
            .with(|connect_info| browser().runtime().connect(EXTENSION_ID, &*connect_info));
        let (tx, mut rx) = mpsc::unbounded();

        port.on_disconnect().add_listener(&closure!(
            @once
            { let tx = tx.clone() },
            move || {
                if let Some(err) = browser().runtime().last_error() {
                    debug_log!(err);
                    console_error!();
                }

                debug_log!("Disconnected foreground port.");
                tx.close_channel();
            },
        ));
        port.on_message().add_listener(&closure!(
            { let tx = tx.clone() },
            move |message: JsValue| {
                let item = serde_wasm_bindgen::from_value(message).unwrap_js();
                common::debug_log!(@f "{:#?}", &item);

                tx.unbounded_send(item).unwrap_js();
            },
        ));

        port.post_message(
            Message::builder(Task::Handshake, Target::Background, MessageKind::Request)
                .build()
                .to_value()?
                .as_ref(),
        );

        let msg = rx.next().await.ok_or_else(|| err_code!())?;
        let validator = MessageValidator::builder(Target::Background)
            .kind(MessageKind::Response)
            .task(Task::Handshake)
            .build();

        validator.validate(&msg)?;

        if let Some(error) = msg.error {
            crate::prelude::message(&error)?;

            wasm_bindgen_futures::spawn_local(async move {
                if let Err(err_code) = Self::display_refresh_message(rx).await {
                    console_error!(err_code);
                }
            });
            return Err(GlobalsError::Unauthorized);
        }

        #[cfg(feature = "antyduch")]
        {
            Port::send(
                Task::builder()
                    .task(Tasks::AttachDebugger)
                    .target(Targets::Background)
                    .build()
                    .unwrap_js()
                    .to_value()
                    .unwrap_js(),
            )
            .await;
        }

        PORT.set(Self {
            port: RefCell::new(port),
            rx: RefCell::new(rx),
            tx,
        })
        .map_err(|_| GlobalsError::unrecoverable())
    }

    // TODO: Better name.
    async fn display_refresh_message(mut rx: mpsc::UnboundedReceiver<Message>) -> JsResult<()> {
        let validator = MessageValidator::builder(Target::Background)
            .maybe_kind(None)
            .build();

        loop {
            let Some(msg) = rx.next().await else {
                return Ok(());
            };

            validator.validate(&msg)?;

            match msg.kind {
                MessageKind::Event => match msg.task {
                    Task::Handshake => {
                        crate::prelude::message("[MDMA::RS] Odśwież grę, aby wczytać zestaw!")?;
                    }
                    _ => unreachable!(),
                },
                MessageKind::Response => match msg.task {
                    Task::OpenPopup => {
                        if let Some(err) = msg.error {
                            crate::prelude::message(&err)?;
                        }
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }
    }

    pub(super) async fn init_session() -> JsResult<Value> {
        Self::send(&Message::new(
            Task::InitSession,
            Target::Background,
            MessageKind::Request,
        ))
        .await?;

        let validator = MessageValidator::builder(Target::Background)
            .kind(MessageKind::Response)
            .build();
        let mut rx_lock = Self::get().rx.borrow_mut();

        loop {
            let msg = rx_lock.next().await.ok_or_else(|| err_code!())?;

            validator.validate(&msg)?;

            match msg.task {
                Task::OpenPopup => {
                    if let Some(err) = msg.error {
                        crate::prelude::message(&err)?;
                    }
                }
                Task::InitSession => return Ok(msg.settings.unwrap_or_default()),
                _ => unreachable!(),
            };
        }
    }

    // fn dispatch_message(task: Task) {
    //     match task.type_() {
    //         Tasks::UserData => Self::on_user_data(task),
    //         Tasks::Cookie => Self::on_cookie(task),
    //         Tasks::SuccessfulConnection => Self::on_successful_connection(task),
    //         _ => unreachable!(),
    //     }
    // }

    // fn on_successful_connection(task: Task) {
    //     PENDING_REQUESTS.with_borrow_mut(|pending| {
    //         pending
    //             .get_mut(&Tasks::SuccessfulConnection)
    //             .unwrap_js()
    //             .pop_front()
    //             .unwrap_js()
    //             .send(task)
    //             .unwrap_js()
    //     })
    // }

    // fn on_user_data(task: Task) {
    //     PENDING_REQUESTS.with_borrow_mut(|pending| {
    //         pending
    //             .get_mut(&Tasks::UserData)
    //             .unwrap_js()
    //             .pop_front()
    //             .unwrap_js()
    //             .send(task)
    //             .unwrap_js()
    //     })
    // }

    // fn on_cookie(task: Task) {
    //     PENDING_REQUESTS.with_borrow_mut(|pending| {
    //         pending
    //             .get_mut(&Tasks::Cookie)
    //             .unwrap_js()
    //             .pop_front()
    //             .unwrap_js()
    //             .send(task)
    //             .unwrap_js()
    //     })
    // }

    // async fn reconnect() {
    //     let (sender, receiver) = oneshot::channel::<Task>();

    //     let should_replace = PENDING_REQUESTS.with_borrow_mut(|pending| {
    //         let queue =
    // pending.get_mut(&Tasks::SuccessfulConnection).unwrap_js();         queue.
    // push_back(sender);         queue.len() == 1
    //     });

    //     if !should_replace {
    //         //debug_log!("Waiting...");
    //         receiver.await.unwrap_js();
    //         return;
    //     }

    //     //debug_log!("REPLACING PORT...");
    //     let connection_success_task = Self::__internal_init(receiver).await;

    //     PENDING_REQUESTS.with_borrow_mut(|pending| {
    //         let waiting =
    // pending.get_mut(&Tasks::SuccessfulConnection).unwrap_js();

    //         //TODO: Is there a better way to do this ?
    //         while let Some(sender) = waiting.pop_front() {
    //             sender.send(connection_success_task.clone()).unwrap_js()
    //         }
    //     })
    // }

    fn post_message(msg: &JsValue) {
        Self::get().port.borrow().post_message(msg);
    }

    // pub(crate) async fn __internal_send_settings_change(addon_name: &AddonName,
    // setting: Value) {     let hero = Hero::get();
    //     let settings = json!({
    //         intern(&hero.account_string): {
    //             intern(&hero.char_id_string): {
    //                 intern(addon_name.key_str()): {
    //                     intern(s!("settings")): setting,
    //                 }
    //             }
    //         }
    //     });
    //     let msg = Task::builder()
    //         .target(Targets::Background)
    //         .task(Tasks::AddonData)
    //         .settings(settings)
    //         .build()
    //         .unwrap_js()
    //         .to_value()
    //         .unwrap_js();

    //     Self::send(msg).await;
    // }

    // pub(crate) async fn __internal_send_active_settings_change(
    //     addon_name: &AddonName,
    //     setting: Value,
    // ) {
    //     let hero = Hero::get();
    //     let settings = json!({
    //         intern(&hero.account_string): {
    //             intern(&hero.char_id_string): {
    //                 addon_name.key_str(): {
    //                     intern(s!("active_settings")): setting,
    //                 }
    //             }
    //         }
    //     });
    //     let msg = Task::builder()
    //         .target(Targets::Background)
    //         .task(Tasks::AddonData)
    //         .settings(settings)
    //         .build()
    //         .unwrap_js()
    //         .to_value()
    //         .unwrap_js();

    //     Self::send(msg).await;
    // }

    pub(crate) async fn fetch_cookie(cookie_details: CookieDetails) -> JsResult<cookies::Cookie> {
        Self::send(
            &Message::builder(Task::Cookie, Target::Background, MessageKind::Request)
                .cookie(Cookie::request(cookie_details))
                .build(),
        )
        .await?;

        let validator = MessageValidator::builder(Target::Background)
            .kind(MessageKind::Response)
            .build();
        let mut rx_lock = Self::get().rx.borrow_mut();
        let cookie = loop {
            let msg = rx_lock.next().await.ok_or_else(|| err_code!())?;

            validator.validate(&msg)?;

            match msg.task {
                Task::OpenPopup => {
                    if let Some(err) = msg.error {
                        crate::prelude::message(&err)?;
                    }
                }
                Task::Cookie => break msg.cookie.ok_or_else(|| err_code!())?,
                _ => unreachable!(),
            };
        };

        cookie.try_into().map_err(|_| err_code!())
    }

    // /// SAFETY: Can be called only after Hero is initialized.
    // pub(crate) async fn fetch_user_data_with_settings() -> JsResult<Task> {
    //     let (sender, receiver) = oneshot::channel::<Task>();

    //     PENDING_REQUESTS.with_borrow_mut(|pending| {
    //         pending
    //             .entry(Tasks::UserData)
    //             .or_insert(VecDeque::with_capacity(5))
    //             .push_back(sender)
    //     });

    //     let hero = Hero::get();
    //     Self::send(
    //         Task::builder()
    //             .task(Tasks::UserData)
    //             .target(Targets::Background)
    //             .settings(json!([&hero.account_string, &hero.char_id_string]))
    //             .build()
    //             .map_err(map_err!())?
    //             .to_value()
    //             .map_err(map_err!())?,
    //     )
    //     .await;

    //     receiver.await.map_err(map_err!(from))
    // }
}

#[derive(Serialize)]
struct ConnectInfo<'a> {
    name: &'a str,
    //TODO: Read up on that.
    #[serde(rename = "includeTlsChannelId")]
    include_tls_channel_id: bool,
}

impl<'a> ConnectInfo<'a> {
    fn new(name: &'a str) -> Self {
        Self {
            name,
            include_tls_channel_id: true,
        }
    }
}
