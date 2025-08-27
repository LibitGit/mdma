#[macro_export]
macro_rules! map_err {
    () => {
        |_err| {
            ::web_sys::console::debug_5(
                &::wasm_bindgen::JsValue::from_str(::obfstr::obfstr!("%c MDMA %c %c Rust ")),
                &::wasm_bindgen::JsValue::from_str(::obfstr::obfstr!(
                    "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
                )),
                &::wasm_bindgen::JsValue::from_str(::obfstr::obfstr!("")),
                &::wasm_bindgen::JsValue::from_str(::obfstr::obfstr!(
                    "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
                )),
                &_err.into(),
            );
            $crate::err_code!()
        }
    };
    (from) => {
        |_err| {
            ::web_sys::console::debug_5(
                &::wasm_bindgen::JsValue::from_str(::obfstr::obfstr!("%c MDMA %c %c Rust ")),
                &::wasm_bindgen::JsValue::from_str(::obfstr::obfstr!(
                    "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
                )),
                &::wasm_bindgen::JsValue::from_str(::obfstr::obfstr!("")),
                &::wasm_bindgen::JsValue::from_str(::obfstr::obfstr!(
                    "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
                )),
                &wasm_bindgen::JsError::from(_err).into(),
            );
            $crate::err_code!()
        }
    };
}

#[macro_export]
macro_rules! err_code {
    () => {
        ::wasm_bindgen::JsValue::from_f64($crate::error::encode_location(
            Some(env!("CARGO_PKG_NAME")),
            file!(),
            line!() as u16,
        ) as f64)
    };
    (as_num) => {
        $crate::error::encode_location(Some(env!("CARGO_PKG_NAME")), file!(), line!() as u16) as f64
    };
    (track_caller) => {{
        let caller = ::std::panic::Location::caller();
        ::wasm_bindgen::JsValue::from_f64($crate::error::encode_location(
            None,
            caller.file(),
            caller.line() as u16,
        ) as f64)
    }};
}

#[macro_export]
macro_rules! trap {
    ($input:expr) => {{
        #[cfg(debug_assertions)]
        panic!("{:?}", $input);

        #[cfg(not(debug_assertions))]
        ::wasm_bindgen::throw_val($input);
    }};
}

#[macro_export]
macro_rules! throw_err_code {
    ($($input:tt)*) => {{
        #[cfg(debug_assertions)]
        panic!($($input)*);

        #[cfg(not(debug_assertions))]
        ::wasm_bindgen::throw_val($crate::err_code!());
    }};
}

#[macro_export]
macro_rules! error {
    ($err:expr) => {{
        $crate::js_imports::js_error(::std::boxed::Box::from([
            ::wasm_bindgen::prelude::JsValue::from(format!(
                "{} {}{}{}{}",
                ::obfstr::obfstr!("MDMA error in"),
                ::obfstr::obfstr!(file!()),
                ::obfstr::obfstr!(":"),
                line!(),
                ::obfstr::obfstr!(":"),
            )),
            ::wasm_bindgen::prelude::JsValue::from(format!("{}", $err)),
        ]));
    }};
}

#[macro_export]
macro_rules! log {
    ($($data:expr),+) => {{
        $crate::js_imports::js_log(::std::boxed::Box::from([
            ::wasm_bindgen::JsValue::from_str("%c MDMA %c %c Rust "),
            ::wasm_bindgen::JsValue::from_str("background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"),
            ::wasm_bindgen::JsValue::from_str(""),
            ::wasm_bindgen::JsValue::from_str("background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"),
            ::wasm_bindgen::JsValue::from_str(&format!(
                "{}{}{}{}",
                ::obfstr::obfstr!(file!()),
                ::obfstr::obfstr!(":"),
                line!(),
                ::obfstr::obfstr!(":"),
            )),
            $(wasm_bindgen::JsValue::from($data),)+
        ]))
    }};
    // (debug => $($data:expr),+) => {{
    //     $crate::js_imports::js_debug(::std::boxed::Box::from([
    //         ::wasm_bindgen::prelude::JsValue::from_str(::wasm_bindgen::intern(::obfstr::obfstr!("[MDMA::RS]:"))),
    //         $(wasm_bindgen::prelude::JsValue::from($data),)+
    //     ]))
    // }};
}

#[macro_export]
macro_rules! debug_log {
    ($($data:expr),+ $(,)?) => {{
        #[cfg(debug_assertions)]
        $crate::log!($($data),+);
    }};

    (@f $format_str:expr $(, $input:expr)*) => {{
        #[cfg(debug_assertions)]
        $crate::log!(&format!($format_str $(, $input)*));
    }};

}
