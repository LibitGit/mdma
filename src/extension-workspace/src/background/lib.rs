#![feature(closure_track_caller)]
use std::cell::RefCell;
use std::collections::HashMap;

pub use common;
use common::messaging::prelude::*;
use common::web_extension_sys::runtime::port::Port;
use futures::channel::oneshot;
pub use obfstr::obfstr as s;
use wasm_bindgen::prelude::*;

pub mod connection;
mod dispatcher;
mod exports;
pub mod types;

thread_local! {
    static PENDING_REQUESTS: RefCell<HashMap<Task, oneshot::Sender<Message>>> = RefCell::new(HashMap::new());
    #[doc = "Needed for `console_err!` macro to display a message to the user."]
    static FOREGROUND_PORT: RefCell<Option<Port>> = const { RefCell::new(None) };
    static TASK_QUEUE: RefCell<types::TaskQueue> = RefCell::new(types::TaskQueue::new());
}

#[wasm_bindgen(start)]
async fn main() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    let mut dispatcher = dispatcher::Dispatcher::init()?;

    types::TaskQueue::init();

    let connection: &'static _ = Box::leak(Box::new(
        connection::Connection::establish_authorized(&mut dispatcher).await?,
    ));
    common::debug_log!("AUTHORIZED CONNECTION ESTABLISHED!");

    dispatcher.spawn_event_loop(connection);

    Ok(())
}

// fn onmessage_factory() -> Function {
//     closure!(|event: MessageEvent| async move {
//         let Some(data) = event.data().as_string() else {
//             return debug_log!("Ws response is not a string!");
//         };
//         let task = match serde_json::from_str(&data) {
//             Err(_err) => {
//                 return debug_log!("Ws response is not a task", event.data(),
// &_err.to_string());             }
//             Ok(t) => t,
//         };

//         debug_log!(&format!("ws task: {task:?}"));

//         dispatch_task(task).await.unwrap_js();
//     })
// }

// async fn dispatch_task(msg: Message) -> Result<(), JsValue> {
//     match msg.task {
//         // Task::Uuid => set_uuid(msg),
//         Task::Tokens => set_tokens(msg).await,
//         Task::UserData => set_user_data(msg),
//         _ => Err(err_code!()),
//     }
// }

// TODO: Send error message via foreground port to display in-game.
#[macro_export]
macro_rules! console_error {
    () => {{
        use ::wasm_bindgen::{JsValue, intern};

        let error_code = ::common::err_code!();
        ::web_sys::console::error_5(
            &JsValue::from_str(intern($crate::s!("%c MDMA %c %c Rust "))),
            &JsValue::from_str(intern($crate::s!(
                "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
            ))),
            &JsValue::from_str(intern($crate::s!(""))),
            &JsValue::from_str(intern($crate::s!(
                "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
            ))),
            &error_code,
        );
        // $crate::utils::logging::__internal_console_error(error_code);
    }};
    ($error_code:expr) => {{
        use ::wasm_bindgen::{JsValue, intern};

        // let _ = $crate::bindings::message(intern($crate::s!("[MDMA::RS] Wystąpił
        // błąd!")));
        ::web_sys::console::error_5(
            &JsValue::from_str(intern($crate::s!("%c MDMA %c %c Rust "))),
            &JsValue::from_str(intern($crate::s!(
                "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
            ))),
            &JsValue::from_str(intern($crate::s!(""))),
            &JsValue::from_str(intern($crate::s!(
                "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
            ))),
            &$error_code,
        );
        // $crate::utils::logging::__internal_console_error($error_code);
    }};
}
