use common::{
    err_code, map_err,
    messaging::prelude::*,
    web_extension_sys::{browser, cookies},
};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

use futures::{StreamExt, channel::mpsc};

use crate::{connection::Connection, types::MessageExt};

pub(super) static PORT_DISPATCHER_TX: OnceLock<mpsc::UnboundedSender<Message>> = OnceLock::new();

pub struct PortDispatcher {
    rx: mpsc::UnboundedReceiver<Message>,
}

impl PortDispatcher {
    pub(super) fn new(rx: mpsc::UnboundedReceiver<Message>) -> Self {
        Self { rx }
    }

    pub fn recv(&mut self) -> futures::stream::Next<'_, mpsc::UnboundedReceiver<Message>> {
        self.rx.next()
    }
}

impl PortDispatcher {
    pub(super) async fn run_event_loop(
        &mut self,
        state: &'static Connection,
        validator: &MessageValidator,
    ) -> Result<(), JsValue> {
        let msg = self.recv().await.ok_or_else(|| err_code!())?;

        validator.validate(&msg)?;

        Self::dispatch_foreground_message(msg, state).await
    }

    async fn dispatch_foreground_message(
        msg: Message,
        state: &'static Connection,
    ) -> Result<(), JsValue> {
        if msg.kind == MessageKind::Event {
            todo!("USER SETTINGS UPDATES");
        }

        match msg.task {
            Task::Handshake => {
                Message::new(Task::Handshake, Target::Foreground, MessageKind::Response)
                    .execute()
                    .await
            }
            Task::Cookie => {
                let cookie_details = &msg
                    .cookie
                    .ok_or_else(|| err_code!())?
                    .details
                    .ok_or_else(|| err_code!())?;
                let cookie_details = serde_wasm_bindgen::to_value(cookie_details)
                    .map_err(map_err!(from))?
                    .unchecked_into();
                let cookie = serde_wasm_bindgen::from_value::<cookies::Cookie>(
                    browser().cookies().get(&cookie_details).await,
                )
                .map_err(map_err!(from))?;
                let cookie = Cookie::response(cookie.value);

                Message::builder(Task::Cookie, Target::Foreground, MessageKind::Response)
                    .cookie(cookie)
                    .build()
                    .execute()
                    .await
            }
            Task::InitSession => {
                Ok(())
                // Message::builder(Task::InitSession, Target::Backend,
                // MessageKind::Request) .
            }
            _ => unreachable!(),
        }
    }
}
