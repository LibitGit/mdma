pub(crate) mod dominator_helpers;
pub(crate) mod logging;
pub(crate) mod window_events;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};

use common::{debug_log, throw_err_code, messaging::prelude::*, err_code};
use futures::{Stream, StreamExt};
use futures_signals::signal::{Mutable, Signal, SignalExt, from_stream};
use futures_signals::signal_map::{self, MapDiff, SignalMap};
use js_sys::{Math, Promise};
use pin_project::pin_project;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use wasm_bindgen::JsValue;
use web_sys::{Document, Window};

use crate::bindings::engine::peer::Peer;
use crate::globals::peers::PeerId;

pub type DefaultResult = std::result::Result<JsValue, JsValue>;
pub type JsResult<T> = std::result::Result<T, JsValue>;

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const BUFFER_SIZE: usize = 32;

thread_local! {
    static BUFFER: RefCell<[u8; BUFFER_SIZE]> = const { RefCell::new([0; BUFFER_SIZE]) };
}

/// Generates a random string with the provided length.
/// Uses js_sys::Math for random number generation.
///
/// # Arguments:
/// * `length` - The desired length of the random string.
///
/// # Returns:
/// A static &'static str slice containing the random string.
pub(crate) fn generate_random_str(length: usize) -> String {
    if length > BUFFER_SIZE {
        throw_err_code!("buffer size smaller than length");
    }

    let char_len = CHARSET.len();
    BUFFER.with_borrow_mut(|buffer| {
        buffer.iter_mut().take(length).for_each(|byte| {
            // Generate a random index and map to CHARSET
            let random_index = (Math::random() * char_len as f64) as usize % char_len;
            *byte = CHARSET[random_index]
        });

        unsafe {
            // Convert buffer to a &'static str
            String::from_utf8_unchecked(buffer[..length].to_vec())
        }
    })
}

pub(crate) fn window() -> Window {
    web_sys::window().unwrap_js()
}

pub(crate) fn document() -> Document {
    window().document().unwrap_js()
}

pub(crate) fn is_from_polish_alphabet(char: char) -> bool {
    char.is_ascii_alphabetic()
        || matches!(
            char,
            'ą' | 'ć'
                | 'ę'
                | 'ł'
                | 'ń'
                | 'ó'
                | 'ś'
                | 'ź'
                | 'ż'
                | 'Ą'
                | 'Ć'
                | 'Ę'
                | 'Ł'
                | 'Ń'
                | 'Ó'
                | 'Ś'
                | 'Ź'
                | 'Ż'
        )
}

pub(crate) fn polish_to_ascii(c: char) -> char {
    match c {
        'ą' | 'Ą' => 'a',
        'ć' | 'Ć' => 'c',
        'ę' | 'Ę' => 'e',
        'ł' | 'Ł' => 'l',
        'ń' | 'Ń' => 'n',
        'ó' | 'Ó' => 'o',
        'ś' | 'Ś' => 's',
        'ź' | 'Ź' => 'z',
        'ż' | 'Ż' => 'z',
        other => other, // Return unchanged if not a Polish character
    }
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
                let caller = std::panic::Location::caller();
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
            |_| {
                let err = err_code!(track_caller);
                crate::console_error!(err.clone());
                wasm_bindgen::throw_val(err)
            },
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
                let caller = std::panic::Location::caller();
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
            || {
                let err = err_code!(track_caller);
                crate::console_error!(err.clone());
                wasm_bindgen::throw_val(err)
            },
        )
    }
}

#[pin_project]
#[derive(Debug)]
#[must_use = "Streams do nothing unless polled"]
struct SignalMapStream<S> {
    #[pin]
    signal_map: S,
}

impl<S: SignalMap> Stream for SignalMapStream<S> {
    type Item = MapDiff<S::Key, S::Value>;

    //Attempt to pull out the next value of this stream, registering the current task for wakeup
    //if the value is not yet available, and returning None if the stream is exhausted
    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.project().signal_map.poll_map_change(cx)
    }
}

#[inline]
pub fn to_stream<S>(signal_map: S) -> impl Stream<Item = MapDiff<S::Key, S::Value>>
where
    S: SignalMap,
{
    SignalMapStream { signal_map }
}

pub trait Setting {
    fn as_setting_signal(&self, f: fn(Value) -> Value) -> impl Stream<Item = Value>;
}

impl<A> Setting for A
where
    A: SettingOption,
{
    fn as_setting_signal(&self, f: fn(Value) -> Value) -> impl Stream<Item = Value> {
        self.as_option_signal(f).to_stream().skip(1)
    }
}

pub trait SettingOption {
    fn as_option_signal(&self, f: fn(Value) -> Value) -> impl Signal<Item = Value>;
}

impl<A> SettingOption for Mutable<A>
where
    A: serde::Serialize,
{
    fn as_option_signal(&self, f: fn(Value) -> Value) -> impl Signal<Item = Value> {
        self.signal_ref(move |data| f(json!(data))).dedupe_cloned()
    }
}

//TODO: Iterate recursivly to determine the keys changed on MapDiff.
//And signal only those properties.
impl<K, V> SettingOption for signal_map::MutableBTreeMap<K, V>
where
    K: serde::Serialize + Ord + Clone + ToString + Debug + 'static,
    V: serde::Serialize + Clone + Debug + 'static,
{
    fn as_option_signal(&self, f: fn(Value) -> Value) -> impl Signal<Item = Value> {
        from_stream(to_stream(self.signal_map_cloned()))
            //.inspect(|jd| debug_log!(@f "diff detected: {jd:?}"))
            .map(move |diff_opt| match diff_opt {
                Some(diff) => match diff {
                    MapDiff::Clear {} => f(Value::Null),
                    MapDiff::Insert { key, value } => f(json!({ key.to_string(): value })),
                    MapDiff::Update { key, value } => f(json!({ key.to_string(): value })),
                    MapDiff::Remove { key } => f(json!({ key.to_string(): Value::Null })),
                    MapDiff::Replace { entries } => {
                        let entries = BTreeMap::from_iter(
                            entries.iter().map(|(key, value)| (key.to_string(), value)),
                        );
                        f(json!(entries))
                    }
                },
                //None => throw_err_code!("Stream did not return a value"),
                None => f(Value::Null),
            })
            .dedupe_cloned()
    }
}

pub trait SettingFromValue {
    fn update(&self, value: Value);
}

impl<T> SettingFromValue for Mutable<T>
where
    T: DeserializeOwned,
{
    fn update(&self, value: Value) {
        match serde_json::from_value(value) {
            Ok(value) => {
                let _ = self.replace(value);
            }
            Err(_err) => debug_log!(_err.to_string()),
        }
    }
}

impl<K, V> SettingFromValue for signal_map::MutableBTreeMap<K, V>
where
    K: DeserializeOwned + Ord + Clone,
    V: DeserializeOwned + Clone,
{
    fn update(&self, value: Value) {
        match serde_json::from_value::<BTreeMap<K, V>>(value) {
            Ok(values) => {
                let mut map_lock = self.lock_mut();
                for (key, value) in values {
                    let _ = map_lock.insert_cloned(key, value);
                }
            }
            Err(_err) => debug_log!(_err.to_string()),
        }
    }
}

//pub(crate) async fn wait_for_val<A>(
//    condition: impl Fn() -> Option<A>,
//    interval: u32,
//    timeout: u32,
//) -> JsResult<A> {
//    if interval > timeout {
//        return Err(err_code!());
//    }
//
//    let mut i = 0;
//
//    while i < timeout {
//        if let Some(ret) = condition() {
//            return Ok(ret);
//        }
//
//        i += interval;
//
//        delay(interval).await;
//    }
//
//    Err(err_code!())
//}

pub(crate) async fn wait_for_without_timeout(condition: impl Fn() -> bool, interval: u32) {
    loop {
        if condition() {
            return;
        }

        delay(interval).await;
    }
}

// pub(crate) async fn wait_for(
//    condition: impl Fn() -> bool,
//    interval: u32,
//    timeout: u32,
// ) -> JsResult<()> {
//    let mut i = 0;

//    while i < timeout {
//        if condition() {
//            return Ok(());
//        }

//        i += interval;

//        delay(interval).await;
//    }

//    Err(err_code!())
// }

pub(crate) async fn delay(ms: u32) {
    let promise = Promise::new(&mut |resolve, _| {
        let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

pub(crate) async fn delay_range(min: usize, max: usize) {
    let promise = Promise::new(&mut |resolve, _| {
        let timeout = min as f64 + js_sys::Math::random() * ((max - min) as f64);
        let _ = window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, timeout as i32);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

pub(crate) trait GetOnlinePeer {
    fn get_online(&self, key: &PeerId) -> Option<&Peer>;
}

impl GetOnlinePeer for signal_map::MutableBTreeMapLockRef<'_, PeerId, Peer> {
    fn get_online(&self, key: &PeerId) -> Option<&Peer> {
        self.get(key)
            .and_then(|peer_data| peer_data.online.get().then_some(peer_data))
    }
}
