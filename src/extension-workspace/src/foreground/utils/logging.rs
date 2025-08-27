use wasm_bindgen::JsValue;

use crate::bindings::get_engine;
use crate::interface::{CONSOLE_MESSAGES, ConsoleLog, ConsoleLogTypes};

pub(crate) fn __internal_console_error(error_message: JsValue) {
    use crate::interface::CONSOLE_LOGS;

    let console_error = ConsoleLog::new(
        ConsoleLogTypes::Error,
        error_message,
        get_engine().get_ev().ok(),
    );

    CONSOLE_LOGS.with_borrow_mut(|logs| {
        if logs.len() == logs.capacity() {
            logs.pop_back();
        }
        logs.push_front(console_error.clone());
    });
    CONSOLE_MESSAGES.with(|logs| logs.lock_mut().push_cloned(console_error))
}

pub(crate) fn console_log(msg: JsValue) {
    use crate::interface::CONSOLE_LOGS;

    let console_log = ConsoleLog::new(ConsoleLogTypes::Message, msg, get_engine().get_ev().ok());

    CONSOLE_LOGS.with_borrow_mut(|logs| {
        if logs.len() == logs.capacity() {
            logs.pop_back();
        }
        logs.push_front(console_log.clone());
    });
    CONSOLE_MESSAGES.with(|logs| logs.lock_mut().push_cloned(console_log))
}
