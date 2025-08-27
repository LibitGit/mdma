// TODO: Update documentation.
// TODO: Make sure adding to the emitter happens after dispatch and before emit
//       Write a wait_for that doesn't take an interval, but instead works on game ticks, e.g. resolve_after: 5 game ticks if cond == true.
//       (what if not true? resolve on the next tick?)
use std::{
    cell::RefCell, collections::BTreeMap, fmt, future::Future, hint::unreachable_unchecked,
    pin::Pin,
};

use common::map_err;
use enum_iterator::{Sequence, all};
use futures::channel::oneshot;
use futures::{StreamExt, stream::FuturesUnordered};

use crate::bindings::prelude::*;
use crate::utils::JsResult;

// const EMITTER_MAP_CAPACITY: usize = cardinality::<EmitterEvent>();
const MESSAGE_HANDLERS_CAPACITY: usize = 10;
const MESSAGE_WAITING_LIST_CAPACITY: usize = 10;
const MESSAGE_INTERCEPTORS_CAPACITY: usize = 1;

// TODO: Better name ?
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Sequence, PartialOrd, Ord)]
pub enum EmitterEvent {
    Artisanship,
    Ask,
    Emo,
    Enhancement,
    Friends,
    Hero,
    Item,
    Loot,
    Members,
    Other,
    Task,
    Settings,
    Warn,
}

type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
type CallbackFnOnce = Box<dyn for<'a> FnOnce(&'a Response) -> BoxedFuture<'a, JsResult<()>>>;
type CallbackFnMut = Box<dyn for<'a> FnMut(&'a Response) -> BoxedFuture<'a, JsResult<()>>>;

thread_local! {
    static CALLBACK_ID: RefCell<usize> = const { RefCell::new(0) };
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct CallbackId(usize);

/// Enum to differentiate between handler async closure types.
enum Callback {
    Once(CallbackFnOnce),
    Mut(CallbackFnMut),
}

/// A callback that gets executed after a specified event gets emitted.
///
/// Handlers encapsulate both the callback function and its execution limits,
/// providing a way to control how many times and under what conditions
/// a callback should be invoked in response to events.
pub struct Handler {
    callback: Callback,
    limit: CallbackLimit,
    id: CallbackId,
}

impl fmt::Debug for Handler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handler")
            .field(
                "callback",
                match &self.callback {
                    Callback::Mut(_) => &"Callback::Mut(_)",
                    Callback::Once(_) => &"Callback::Once(_)",
                },
            )
            .field("limit", &self.limit)
            .field("id", &self.id)
            .finish()
    }
}

type InterceptorFnOnce = Box<dyn for<'a> FnOnce(&'a mut Response) -> BoxedFuture<'a, JsResult<()>>>;
type InterceptorFnMut = Box<dyn for<'a> FnMut(&'a mut Response) -> BoxedFuture<'a, JsResult<()>>>;

/// Enum to differentiate between interceptor async closure types.
enum InterceptorCallback {
    Once(InterceptorFnOnce),
    Mut(InterceptorFnMut),
}

/// A [`Handler`] which can mutate the websocket's reponse.
///
/// The interceptor is mainly used for removing parts of a [`Response`] whenever the game's dispatch would result in an [`Err`],
/// e.g. after sending `_g("clan&a=members")` without opening the appropraite clan widget first.
///
/// ### Note
///
/// Always make sure to not partialy remove a response and run other handlers on it expecting the removed part.
pub struct Interceptor {
    callback: InterceptorCallback,
    limit: CallbackLimit,
    id: CallbackId,
}

impl fmt::Debug for Interceptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interceptor")
            .field(
                "callback",
                match &self.callback {
                    InterceptorCallback::Mut(_) => &"InterceptorCallback::Mut(_)",
                    InterceptorCallback::Once(_) => &"InterceptorCallback::Once(_)",
                },
            )
            .field("limit", &self.limit)
            .field("id", &self.id)
            .finish()
    }
}

#[derive(Debug)]
enum CallbackLimit {
    Unbounded,
    Bounded(usize),
    Once,
}

#[derive(Debug)]
struct Waiting {
    callback_id: CallbackId,
    sender: oneshot::Sender<()>,
}

impl Waiting {
    fn new(callback_id: CallbackId, sender: oneshot::Sender<()>) -> Self {
        Self {
            callback_id,
            sender,
        }
    }
}

thread_local! {
    pub static EMITTER: RefCell<Emitter> = const { RefCell::new(Emitter::new()) };
}

#[derive(Debug)]
pub struct Emitter {
    pub(crate) handlers: BTreeMap<EmitterEvent, Vec<Handler>>,
    handlers_after: BTreeMap<EmitterEvent, Vec<Handler>>,
    pub(crate) interceptors: BTreeMap<EmitterEvent, Vec<Interceptor>>,
    waiting_list: BTreeMap<EmitterEvent, Vec<Waiting>>,
}

impl Emitter {
    const fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
            handlers_after: BTreeMap::new(),
            interceptors: BTreeMap::new(),
            waiting_list: BTreeMap::new(),
        }
    }

    pub(super) fn init() -> JsResult<()> {
        #[cfg(feature = "antyduch")]
        {
            let (sender, receiver) = futures::channel::mpsc::unbounded::<bool>();

            Self::register_on(EmitterEvent::Hero, move |res: &Response| {
                let sender = sender.clone();
                Box::pin(async move {
                    common::debug_log!(@f "{:?}", res.h);
                    let back = res
                        .h
                        .as_ref()
                        .and_then(|h_data| h_data.back)
                        .unwrap_or_default();

                    sender.unbounded_send(back).map_err(map_err!(from))
                })
            })?;

            crate::pathfinder::NOTIFY.set(Some(receiver));
        }

        Self::intercept_on(EmitterEvent::Warn, |res| {
            Box::pin(async {
                if res
                    .w
                    .as_ref()
                    .is_some_and(|msg| msg.starts_with("Pakiet odrzucony"))
                {
                    res.w.take();
                }

                Ok(())
            })
        })?;

        Ok(())
    }

    pub(crate) async fn emit_events(socket_response: &mut Response) -> bool {
        let mut event_data_changed = false;

        for event in all::<EmitterEvent>() {
            if !Self::should_emit(&event, socket_response) {
                continue;
            }
            if Self::emit(event, socket_response).await {
                event_data_changed = true;
            }
        }

        event_data_changed
    }

    pub(crate) async fn emit_after_events(socket_response: &Response) {
        for event in all::<EmitterEvent>() {
            if Self::should_emit(&event, socket_response) {
                Self::emit_after(event, socket_response).await
            }
        }
    }

    fn should_emit(event: &EmitterEvent, socket_response: &Response) -> bool {
        use EmitterEvent::*;

        EMITTER.with_borrow(|emitter| {
            if !emitter.callback_registered_for(&event) {
                return false;
            }

            match event {
                Artisanship => socket_response.artisanship.is_some(),
                Ask => socket_response.ask.is_some(),
                Emo => socket_response.emo.is_some(),
                Enhancement => socket_response.enhancement.is_some(),
                Friends => socket_response.friends.is_some(),
                Hero => socket_response.h.is_some(),
                Item => socket_response.item.is_some(),
                Loot => socket_response.loot.is_some(),
                Members => socket_response.members.is_some(),
                Other => socket_response.other.is_some(),
                Task => socket_response.t.is_some(),
                Settings => socket_response.character_settings.is_some(),
                Warn => socket_response.w.is_some(),
            }
        })
    }

    fn callback_registered_for(&self, event: &EmitterEvent) -> bool {
        self.handlers.contains_key(event) || self.interceptors.contains_key(event)
    }

    // TODO: Update docs.
    /// Adds an unbounded event listener that will get executed each time the event gets emitted.
    /// For bounded alternatives see [`register_limited`], [`register_once`].
    ///
    /// [`register_limited`]: #method.register_limited.
    /// [`register_once`]: #method.register_once.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `handlers` map is currently borrowed.
    pub fn register_after_on<C>(event: EmitterEvent, callback: C) -> JsResult<CallbackId>
    where
        C: for<'a> FnMut(&'a Response) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>>
            + 'static,
    {
        Self::register_handler_after(
            event,
            CallbackLimit::Unbounded,
            Callback::Mut(Box::new(callback)),
        )
    }

    // TODO: Update docs.
    /// Internal method for registering a handler on the emitter.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `handlers` map is currently borrowed.
    fn register_handler_after(
        event: EmitterEvent,
        limit: CallbackLimit,
        callback: Callback,
    ) -> JsResult<CallbackId> {
        let id = CallbackId(CALLBACK_ID.with_borrow_mut(|id| {
            let old = *id;
            *id = (*id).wrapping_add(1);
            old
        }));
        let listener = Handler {
            limit,
            callback,
            id,
        };

        EMITTER.with_borrow_mut(|emitter| {
            emitter
                .handlers_after
                .entry(event)
                .or_insert_with(|| Vec::with_capacity(MESSAGE_HANDLERS_CAPACITY))
                .push(listener);
        });

        Ok(id)
    }

    /// Adds an unbounded event listener that will get executed each time the event gets emitted.
    /// For bounded alternatives see [`register_limited`], [`register_once`].
    ///
    /// [`register_limited`]: #method.register_limited.
    /// [`register_once`]: #method.register_once.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `handlers` map is currently borrowed.
    pub fn register_on<C>(event: EmitterEvent, callback: C) -> JsResult<CallbackId>
    where
        C: for<'a> FnMut(&'a Response) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>>
            + 'static,
    {
        Self::register_handler(
            event,
            CallbackLimit::Unbounded,
            Callback::Mut(Box::new(callback)),
        )
    }

    /// Adds an event listener that will get executed a given number of times.
    /// For an unbounded alternative see [`register_on`].
    ///
    /// [`register_on`]: #method.register_on.
    ///
    /// # Panics
    ///
    /// Panics in non optimized builds if `limit == 0`.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `handlers` map is currently borrowed.
    pub fn register_limited<C>(
        event: EmitterEvent,
        limit: usize,
        callback: C,
    ) -> JsResult<CallbackId>
    where
        C: for<'a> FnMut(&'a Response) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>>
            + 'static,
    {
        debug_assert!(
            limit != 0,
            "The call limit was set to 0. If you don't want to call the callback why even add it?"
        );

        Self::register_handler(
            event,
            CallbackLimit::Bounded(limit),
            Callback::Mut(Box::new(callback)),
        )
    }

    /// Adds an event listener that will get executed once.
    /// For an unbounded alternative see [`register_on`].
    ///
    /// [`register_on`]: #method.register_on.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `handlers` map is currently borrowed.
    pub fn register_once<C>(&self, event: EmitterEvent, callback: C) -> JsResult<CallbackId>
    where
        C: for<'a> FnOnce(&'a Response) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>>
            + 'static,
    {
        Self::register_handler(
            event,
            CallbackLimit::Once,
            Callback::Once(Box::new(callback)),
        )
    }

    /// Internal method for registering a handler on the emitter.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `handlers` map is currently borrowed.
    fn register_handler(
        event: EmitterEvent,
        limit: CallbackLimit,
        callback: Callback,
    ) -> JsResult<CallbackId> {
        let id = CallbackId(CALLBACK_ID.with_borrow_mut(|id| {
            let old = *id;
            *id = (*id).wrapping_add(1);
            old
        }));
        let listener = Handler {
            limit,
            callback,
            id,
        };

        EMITTER.with_borrow_mut(|emitter| {
            emitter
                .handlers
                .entry(event)
                .or_insert_with(|| Vec::with_capacity(MESSAGE_HANDLERS_CAPACITY))
                .push(listener);
        });

        Ok(id)
    }

    /// Adds an unbounded event interceptor that will get executed each time the event gets emitted.
    /// For bounded alternatives see [`intercept_limited`], [`intercept_once`].
    ///
    /// [`intercept_limited`]: #method.intercept_limited.
    /// [`intercept_once`]: #method.intercept_once.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `interceptors` map is currently borrowed.
    pub fn intercept_on<B>(event: EmitterEvent, callback: B) -> JsResult<CallbackId>
    where
        B: for<'a> FnMut(&'a mut Response) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>>
            + 'static,
    {
        Self::register_intercept(
            event,
            CallbackLimit::Unbounded,
            InterceptorCallback::Mut(Box::new(callback)),
        )
    }

    /// Adds an event interceptor that will get executed a given number of times.
    /// For an unbounded alternative see [`intercept_on`].
    ///
    /// [`intercept_on`]: #method.intercept_on.
    ///
    /// # Panics
    ///
    /// Panics in non optimized builds if `limit == 0`.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `interceptors` map is currently borrowed.
    pub fn intercept_limited<D>(
        event: EmitterEvent,
        limit: usize,
        callback: D,
    ) -> JsResult<CallbackId>
    where
        D: for<'a> FnMut(&'a mut Response) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>>
            + 'static,
    {
        debug_assert!(
            limit != 0,
            "The call limit was set to 0. If you don't want to call the callback why even add it?"
        );

        Self::register_intercept(
            event,
            CallbackLimit::Bounded(limit),
            InterceptorCallback::Mut(Box::new(callback)),
        )
    }

    /// Adds an event interceptor that will get executed once.
    /// For an unbounded alternative see [`intercept_on`].
    ///
    /// [`intercept_on`]: #method.intercept_on.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `interceptors` map is currently borrowed.
    pub fn intercept_once<D>(event: EmitterEvent, callback: D) -> JsResult<CallbackId>
    where
        D: for<'a> FnOnce(&'a mut Response) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>>
            + 'static,
    {
        Self::register_intercept(
            event,
            CallbackLimit::Once,
            InterceptorCallback::Once(Box::new(callback)),
        )
    }

    /// Internal method for registering an interceptor on the emitter.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `interceptors` map is currently borrowed.
    fn register_intercept(
        event: EmitterEvent,
        limit: CallbackLimit,
        callback: InterceptorCallback,
    ) -> JsResult<CallbackId> {
        let id = CallbackId(CALLBACK_ID.with_borrow_mut(|id| {
            let old = *id;
            *id = (*id).wrapping_add(1);
            old
        }));
        let interceptor = Interceptor {
            limit,
            callback,
            id,
        };

        EMITTER.with_borrow_mut(|emitter| {
            emitter
                .interceptors
                .entry(event)
                .or_insert_with(|| Vec::with_capacity(MESSAGE_INTERCEPTORS_CAPACITY))
                .push(interceptor);
        });

        Ok(id)
    }

    /// Creates a [`Future`] that get's resolved after an [`Interceptor`] for the given event is executed.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `waiting_list` map is currently borrowed or if no interceptor with the specified id gets executed during emit.
    pub async fn wait_for_intercept(event: EmitterEvent, callback_id: CallbackId) -> JsResult<()> {
        let (tx, rx) = futures::channel::oneshot::channel();
        let waiting = Waiting::new(callback_id, tx);

        EMITTER.with_borrow_mut(|emitter| {
            emitter
                .waiting_list
                .entry(event)
                .or_insert_with(|| Vec::with_capacity(MESSAGE_WAITING_LIST_CAPACITY))
                .push(waiting);
        });

        rx.await.map_err(map_err!(from))
    }

    /// Remove a [`Handler`] from the task queue.
    ///
    /// Returns [`None`] if there is no handler with the provided id, [`Some`] otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `handlers` map is currently borrowed.
    pub fn unregister_handler(event: &EmitterEvent, callback_id: CallbackId) -> Option<Handler> {
        EMITTER.with_borrow_mut(|emitter| {
            let handlers = emitter.handlers.get_mut(event)?;

            handlers
                .iter()
                .position(|handler| handler.id == callback_id)
                .map(|index| handlers.remove(index))
        })
    }

    /// Remove an [`Interceptor`] from the task queue.
    ///
    /// Returns [`None`] if there is no interceptor with the provided id, [`Some`] otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if  the `interceptors` map is currently borrowed.
    pub fn unregister_interceptor(
        event: &EmitterEvent,
        callback_id: CallbackId,
    ) -> Option<Interceptor> {
        EMITTER.with_borrow_mut(|emitter| {
            let interceptors = emitter.interceptors.get_mut(event)?;

            interceptors
                .iter()
                .position(|interceptor| interceptor.id == callback_id)
                .map(|index| interceptors.remove(index))
        })
    }

    /// Emits callbacks for the given [`EmitterEvent`].
    ///
    /// This method calls all the available [`Interceptor`]s first, notifying every [`Waiting`] from the `waiting_list` after that.
    /// Then it executes each [`Handler`] registered for the event.
    ///
    /// Returns `true` if an interceptor was executed, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if [`emit_interceptors`] fails, if any of the `Receiver`s from the `wait_list` were dropped or if the `handlers` map is currently borrowed.
    ///
    /// [`emit_interceptors`]: #method.emit_interceptors.
    async fn emit(event: EmitterEvent, socket_response: &mut Response) -> bool {
        let intercepted_ids = unsafe { Self::emit_interceptors(event, socket_response).await };
        let intercepted_any = !intercepted_ids.is_empty();
        let mut emitter_task_set = futures::stream::FuturesUnordered::new();

        EMITTER.with_borrow_mut(|emitter| {
            emitter.empty_waiting_list(event, intercepted_ids);

            let handlers = emitter
                .handlers
                .entry(event)
                .or_insert_with(|| Vec::with_capacity(MESSAGE_HANDLERS_CAPACITY));

            unsafe {
                let mut index = 0;
                while index < handlers.len() {
                    Self::enqueue_handler(&emitter_task_set, &mut index, handlers, socket_response);
                }
            }
        });

        while let Some(handler_result) = emitter_task_set.next().await {
            if let Err(err_code) = handler_result {
                console_error!(err_code);
            }
        }

        intercepted_any
    }

    fn empty_waiting_list(&mut self, event: EmitterEvent, intercepted: Vec<CallbackId>) {
        self.waiting_list
            .entry(event)
            .or_insert_with(|| Vec::with_capacity(MESSAGE_WAITING_LIST_CAPACITY))
            .drain(..)
            .filter_map(|waiting| {
                intercepted
                    .contains(&waiting.callback_id)
                    .then_some(waiting.sender)
            })
            .for_each(|tx| {
                if tx.send(()).is_err() {
                    console_error!()
                }
            });
    }

    async fn emit_after(event: EmitterEvent, socket_response: &Response) {
        let mut emitter_task_set = futures::stream::FuturesUnordered::new();

        EMITTER.with_borrow_mut(|emitter| {
            let handlers = emitter
                .handlers_after
                .entry(event)
                .or_insert_with(|| Vec::with_capacity(MESSAGE_HANDLERS_CAPACITY));

            unsafe {
                let mut index = 0;
                while index < handlers.len() {
                    Self::enqueue_handler(&emitter_task_set, &mut index, handlers, socket_response);
                }
            }
        });

        while let Some(handler_result) = emitter_task_set.next().await {
            if let Err(err_code) = handler_result {
                console_error!(err_code);
            }
        }
    }

    /// Executes all [`Interceptor`]s for the given event.
    ///
    /// Interceptors will be unregistered if:
    /// - They are not an instance of [`InterceptorCallback::Mut`], or
    /// - Their call `limit` has reached zero.
    ///
    /// Returns a [`Vec`] of [`CallbackId`] with ids of all interceptors that got executed.
    ///
    /// # Safety
    ///
    /// Safety preconditions for calling this method are:
    /// - [`CallbackLimit`] on all of the `Interceptor`s must be either:
    ///     - `CallbackLimit::Unbounded | CallbackLimit::Bounded(limit)` with `limit > 0` if the interceptor's `callback` is an instance of `InterceptorCallback::Mut`,
    ///     - `CallbackLimit::Once` if the  interceptor's `callback` is an instance of `InterceptorCallback::Once`.
    ///
    /// Violating any of these will result in *[undefined behavior]*.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the `interceptors` map is currently borrowed.
    async unsafe fn emit_interceptors(
        event: EmitterEvent,
        socket_response: &mut Response,
    ) -> Vec<CallbackId> {
        let intercepted = EMITTER.with_borrow_mut(|emitter| {
            emitter
                .interceptors
                .entry(event)
                .or_insert_with(|| Vec::with_capacity(MESSAGE_INTERCEPTORS_CAPACITY))
                .iter()
                .map(|interceptor| interceptor.id)
                .collect()
        });

        unsafe {
            let mut index = 0;
            while index
                < EMITTER.with_borrow(|emitter| {
                    emitter
                        .interceptors
                        .get(&event)
                        .map(Vec::len)
                        .unwrap_or_default()
                })
            {
                if let Err(err_code) =
                    Self::execute_interceptor(&mut index, event, socket_response).await
                {
                    console_error!(err_code);
                }
            }
        }

        intercepted
    }

    /// Execute a single [`Interceptor`].
    ///
    /// If it's `callback` is not an instance of [`InterceptorCallback::Mut`][InterceptorCallback] or if it's call `limit` reaches zero, the `Interceptor` gets unregistered.
    ///
    /// # Safety
    ///
    /// Safety preconditions for calling this method are:
    /// - `index` must be smaller than `interceptors.len()`
    /// - [`CallbackLimit`] on all of the interceptors must be either:
    ///     - `CallbackLimit::Unbounded | CallbackLimit::Bounded(limit)` with `limit > 0` if the interceptor's `callback` is an instance of `InterceptorCallback::Mut`,
    ///     - `CallbackLimit::Once` if the interceptor's `callback` is an instance of `InterceptorCallback::Once`.
    ///
    /// Violating any of these will result in *[undefined behavior]*.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the [`Future`] returned by the `Interceptor` fails.
    // TODO: Check if it's better to store indexes to remove in a vec.
    unsafe fn execute_interceptor<'a>(
        index: &mut usize,
        event: EmitterEvent,
        socket_response: &'a mut Response,
    ) -> Pin<Box<dyn futures::Future<Output = JsResult<()>> + 'a>> {
        EMITTER.with_borrow_mut(|emitter| {
            let interceptors = emitter
                .interceptors
                .entry(event)
                .or_insert_with(|| Vec::with_capacity(MESSAGE_INTERCEPTORS_CAPACITY));
            // SAFETY: The caller must uphold the safety requirements for `execute_interceptor`;
            //         `index` has to be smaller than `interceptors.len()`.
            let interceptor = unsafe { interceptors.get_unchecked_mut(*index) };

            match &mut interceptor.limit {
                CallbackLimit::Once => {
                    // SAFETY: `CallbackLimit::Once` has to be imposed only on `FnOnce`'s.
                    let InterceptorCallback::Once(once_callback) =
                        interceptors.remove(*index).callback
                    else {
                        unsafe { unreachable_unchecked() }
                    };
                    once_callback(socket_response)
                }
                CallbackLimit::Bounded(limit) => {
                    // SAFETY: We know `limit` is bigger than 0.
                    let new_limit = unsafe { limit.unchecked_sub(1) };
                    let callback = match new_limit {
                        0 => &mut interceptors.remove(*index).callback,
                        _ => {
                            *limit = new_limit;
                            &mut interceptor.callback
                        }
                    };

                    // SAFETY: Any callback limit other than `CallbackLimit::Once` has to be imposed only on `FnMut`'s.
                    let &mut InterceptorCallback::Mut(ref mut callback) = callback else {
                        unsafe { unreachable_unchecked() }
                    };
                    *index += 1;
                    callback(socket_response)
                }
                CallbackLimit::Unbounded => {
                    // SAFETY: Same as above.
                    let InterceptorCallback::Mut(ref mut callback) = interceptor.callback else {
                        unsafe { unreachable_unchecked() }
                    };
                    *index += 1;
                    callback(socket_response)
                }
            }
        })
    }

    /// Enqueue a [`Handler`] in the given [`FuturesUnordered`] set.
    ///
    /// If it's `callback` is not an instance of [`Callback::Mut`] or if it's call `limit` reaches zero, the `Handler` gets unregistered.
    ///
    /// # Safety
    ///
    /// Safety preconditions for calling this method are:
    /// - `index` must be smaller than `handlers.len()`
    /// - [`CallbackLimit`] on all of the handlers must be either:
    ///     - `CallbackLimit::Unbounded | CallbackLimit::Bounded(limit)` with `limit > 0` if the handler's `callback` is an instance of `Callback::Mut`,
    ///     - `CallbackLimit::Once` if the handler's `callback` is an instance of `Callback::Once`.
    ///
    /// Violating any of these will result in *[undefined behavior]*.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    unsafe fn enqueue_handler<'a>(
        tasks: &FuturesUnordered<Pin<Box<dyn Future<Output = JsResult<()>> + 'a>>>,
        index: &mut usize,
        handlers: &mut Vec<Handler>,
        socket_response: &'a Response,
    ) {
        // SAFETY: The caller must uphold the safety requirements for `enqueue_callback`;
        //         `index` has to be smaller than `handlers.len()`.
        let handler = unsafe { handlers.get_unchecked_mut(*index) };
        let future = match &mut handler.limit {
            CallbackLimit::Once => {
                // SAFETY: `CallbackLimit::Once` has to be imposed only on `FnOnce`'s.
                let Callback::Once(once_callback) = handlers.swap_remove(*index).callback else {
                    unsafe { unreachable_unchecked() }
                };
                once_callback(socket_response)
            }
            CallbackLimit::Bounded(limit) => {
                // SAFETY: We know `limit` is bigger than 0.
                let new_limit = unsafe { limit.unchecked_sub(1) };
                let callback = match new_limit {
                    0 => &mut handlers.swap_remove(*index).callback,
                    _ => {
                        *limit = new_limit;
                        &mut handler.callback
                    }
                };

                // SAFETY: Any callback limit other than `CallbackLimit::Once` has to be imposed only on `FnMut`'s.
                let &mut Callback::Mut(ref mut callback) = callback else {
                    unsafe { unreachable_unchecked() }
                };
                *index += 1;
                callback(socket_response)
            }
            CallbackLimit::Unbounded => {
                // SAFETY: Same as above.
                let Callback::Mut(ref mut callback) = handler.callback else {
                    unsafe { unreachable_unchecked() }
                };
                *index += 1;
                callback(socket_response)
            }
        };

        tasks.push(future);
    }
}
