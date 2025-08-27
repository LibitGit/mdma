use std::cell::RefCell;

use common::{
    err_code,
    messaging::prelude::{self as messaging, *},
};
use futures::StreamExt;

use crate::utils::JsResult;

use super::port::Port;

const UNAUTHORIZED_ACCESS_LEVEL: u8 = 0;

thread_local! {
    // TODO: Store encrypted.
    static PREMIUM: RefCell<Option<messaging::Premium>> = const { RefCell::new(None) };
}

pub struct Premium;

impl Premium {
    // TODO: Check exp.
    pub fn active() -> bool {
        PREMIUM.with_borrow(|premium| premium.is_some())
    }

    #[cfg(feature = "antyduch")]
    pub fn anty_duch() -> bool {
        PREMIUM.with_borrow(|premium| premium.as_ref().is_some_and(|premium| premium.antyduch))
    }
    
    /// # SAFETY
    /// Has to be called after [`Port`] connection is initialized.
    pub(super) async fn init() -> JsResult<()> {
        Port::send(&Message::new(
            Task::UserData,
            Target::Background,
            MessageKind::Request,
        ))
        .await?;

        let validator = MessageValidator::builder(Target::Background)
            .kind(MessageKind::Response)
            .build();
        let mut rx_lock = Port::get().rx.borrow_mut();

        loop {
            let msg = rx_lock.next().await.ok_or_else(|| err_code!())?;

            validator.validate(&msg)?;

            match msg.task {
                Task::OpenPopup => {
                    if let Some(err) = msg.error {
                        crate::prelude::message(&err)?;
                    }
                }
                Task::UserData => PREMIUM.with_borrow_mut(|premium| *premium = msg.premium),
                _ => unreachable!(),
            };
        }
    }
}
