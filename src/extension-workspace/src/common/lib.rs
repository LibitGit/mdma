#![feature(closure_track_caller)]

cfg_if::cfg_if! {
    if #[cfg(feature = "extension")] {
        pub mod error;
        pub mod file_names;
        pub mod js_imports;
        pub mod macros;
        pub mod web_extension_sys;

        pub use skibidi::*;
    }
}

#[cfg(feature = "extension")]
#[macro_use]
pub mod log;
#[cfg(any(feature = "backend", feature = "background"))]
pub mod connection;
#[cfg(feature = "task")]
pub mod messaging;

#[cfg(feature = "extension")]
pub mod sleep {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use futures::future::FusedFuture;
    use pin_project::pin_project;
    use wasm_bindgen_futures::JsFuture;

    #[pin_project]
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Sleep {
        #[pin]
        fut: Option<JsFuture>,
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.as_mut().project().fut.as_pin_mut() {
                Some(fut) => fut.poll(cx).map(|_| {
                    self.project().fut.set(None);
                    ()
                }),
                None => Poll::Pending,
            }
        }
    }

    impl FusedFuture for Sleep {
        fn is_terminated(&self) -> bool {
            self.fut.is_none()
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "background")] {
            fn global() -> web_sys::ServiceWorkerGlobalScope {
                use wasm_bindgen::JsCast;

                js_sys::global().unchecked_into()
            }

        } else if #[cfg(any(feature = "popup", feature = "foreground"))] {
            fn global() -> web_sys::Window {
                use crate::UnwrapJsExt;

                web_sys::window().unwrap_js()
            }
        }
    }

    pub fn sleep(ms: u32) -> Sleep {
        use js_sys::Promise;

        let promise = Promise::new(&mut |resolve, _| {
            let _ =
                global().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
        });

        Sleep {
            fut: Some(wasm_bindgen_futures::JsFuture::from(promise)),
        }
    }
}

#[cfg(feature = "extension")]
pub use sleep::sleep;

#[cfg(feature = "extension")]
pub mod skibidi {
    #[cfg(debug_assertions)]
    use ::std::panic::Location;

    use wasm_bindgen::JsValue;

    pub type Result<T> = std::result::Result<T, JsValue>;

    #[macro_export]
    macro_rules! abort {
        ($process:expr, $err:expr $(,)?) => {{
            $crate::js_imports::js_error(::std::boxed::Box::from([
                ::wasm_bindgen::prelude::JsValue::from(format!(
                    "{}{}{} {}{}{}{}",
                    ::obfstr::obfstr!("[MDMA, "),
                    ::obfstr::obfstr!($process),
                    ::obfstr::obfstr!("] Abort in"),
                    ::obfstr::obfstr!(file!()),
                    ::obfstr::obfstr!(":"),
                    line!(),
                    ::obfstr::obfstr!(":"),
                )),
                ::wasm_bindgen::prelude::JsValue::from(format!("{}", $err)),
            ]));
            std::process::abort();
        }};
    }

    pub trait UnwrapJsExt<T> {
        fn unwrap_js(self) -> T;
    }

    #[cfg(debug_assertions)]
    impl<T, E: ::std::fmt::Debug> UnwrapJsExt<T> for ::std::result::Result<T, E> {
        #[inline]
        #[track_caller]
        fn unwrap_js(self) -> T {
            match self {
                Ok(value) => value,
                Err(err) => {
                    let caller = Location::caller();
                    panic!("[{}:{}] {err:?}", caller.file(), caller.line());
                }
            }
        }
    }

    #[cfg(not(debug_assertions))]
    impl<T, E> UnwrapJsExt<T> for ::std::result::Result<T, E> {
        #[inline]
        #[track_caller]
        fn unwrap_js(self) -> T {
            self.unwrap_or_else(
                #[track_caller]
                |_| wasm_bindgen::throw_val(err_code!(track_caller)),
            )
        }
    }

    #[cfg(debug_assertions)]
    impl<T> UnwrapJsExt<T> for Option<T> {
        #[inline]
        #[track_caller]
        fn unwrap_js(self) -> T {
            match self {
                Some(value) => value,
                None => {
                    let caller = Location::caller();
                    panic!(
                        "[{}:{}] Called unwrap on None value",
                        caller.file(),
                        caller.line()
                    );
                }
            }
        }
    }

    #[cfg(not(debug_assertions))]
    impl<T> UnwrapJsExt<T> for Option<T> {
        #[inline]
        #[track_caller]
        fn unwrap_js(self) -> T {
            self.unwrap_or_else(
                #[track_caller]
                || wasm_bindgen::throw_val(err_code!(track_caller)),
            )
        }
    }
}
