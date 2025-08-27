use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_signals::signal::{Mutable, MutableSignal, Receiver, Signal, SignalExt, channel};
use wasm_bindgen::intern;
use web_sys::{
    DomRect, Element, MutationObserver, MutationObserverInit, MutationRecord, Node, ResizeObserver,
};

use crate::prelude::*;

/// Signal of attribute mutation records, driven by a MutationObserver on the Element
pub struct DomMutationSignal {
    _observer: MutationObserver,
    receiver: Receiver<Vec<MutationRecord>>,
}

impl Signal for DomMutationSignal {
    type Item = Vec<MutationRecord>;

    #[inline]
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.receiver.poll_change_unpin(cx)
    }
}

impl DomMutationSignal {
    pub fn new<A: AsRef<Node>>(element: A) -> Self {
        let (sender, receiver) = channel(vec![]);

        let observer = {
            let mut closed = false;
            let mutation_callback = closure!(move |mutation_list: Vec<MutationRecord>| {
                let upgrade_lvl_changed = mutation_list.iter().any(|mutation| {
                    mutation
                        .attribute_name()
                        .is_some_and(|name| name == intern(s!("data-upgrade")))
                });
                if !upgrade_lvl_changed {
                    return;
                }
                if closed {
                    return;
                }
                if sender.send(mutation_list).is_err() {
                    //debug_log!(err);
                    sender.close();
                    closed = true;
                }
            });

            MutationObserver::new(&mutation_callback)
                .map_err(map_err!())
                .unwrap_js()
        };
        let mutation_observer_init = MutationObserverInit::new();
        mutation_observer_init.set_attributes(true);

        observer
            .observe_with_options(element.as_ref(), &mutation_observer_init)
            .unwrap_js();

        Self {
            _observer: observer,
            receiver,
        }
    }
}

#[derive(Default)]
pub struct OverflowSignalOptions {
    subtree: bool,
    _depth: Option<usize>,
}

impl OverflowSignalOptions {
    pub const fn builder() -> OverflowSignalOptionsBuilder {
        OverflowSignalOptionsBuilder::new()
    }
}

pub struct OverflowSignalOptionsBuilder {
    subtree: bool,
    depth: Option<usize>,
}

impl OverflowSignalOptionsBuilder {
    const fn new() -> Self {
        Self {
            subtree: false,
            depth: None,
        }
    }

    pub const fn with_subtree(mut self, depth: usize) -> Self {
        self.subtree = true;
        self.depth = Some(depth);
        self
    }

    pub const fn build(self) -> OverflowSignalOptions {
        OverflowSignalOptions {
            subtree: self.subtree,
            _depth: self.depth,
        }
    }
}

/// `Signal` which uses a `ResizeObserver` to determine if an element has overflow (scrollWidth > clientWidth).
/// The signal returns `true` when an element has oveflow, `false` otherwise.
pub struct OverflowSignal {
    _observer: ResizeObserver,
    signal: MutableSignal<bool>,
}

impl Signal for OverflowSignal {
    type Item = bool;

    #[inline]
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.signal.poll_change_unpin(cx)
    }
}

impl OverflowSignal {
    pub fn new(element: &Element) -> Self {
        Self::new_with_options(element, OverflowSignalOptions::default())
    }

    // TODO: Should it only react on the original element's rezize?
    pub fn new_with_options(element: &Element, options: OverflowSignalOptions) -> Self {
        let mut initial = element.scroll_width() > element.client_width();

        if options.subtree && !initial {
            // TODO: Add recursion for specified depth.
            let children = element.children();
            for index in 0..children.length() {
                let child = children.get_with_index(index).unwrap_js();
                if child.scroll_width() > child.client_width() {
                    initial = true;
                    break;
                }
            }
        }

        let overflow = Mutable::new(initial);
        let signal = overflow.signal();

        let observer = {
            ResizeObserver::new(&closure!(
                {
                    let element = element.clone(),
                    //let overflow = overflow.clone(),
                },
                move || {
                    let mut new_overflow = element.scroll_width() > element.client_width();

                    if new_overflow {
                        overflow.set_neq(true);
                        return;
                    }

                    let children = element.children();

                    for index in 0..children.length() {
                        let child = children.get_with_index(index).unwrap_js();
                        if child.scroll_width() > child.client_width() {
                            new_overflow = true;
                            break;
                        }
                    }

                    overflow.set_neq(new_overflow);
                },
            ))
            .unwrap_js()
        };

        observer.observe(element);
        if options.subtree {
            let children = element.children();

            for index in 0..children.length() {
                let child = children.get_with_index(index).unwrap_js();
                observer.observe(&child);
            }
        }

        Self {
            _observer: observer,
            signal,
        }
    }
}

/// Signal of DomRect, driven by a ResizeObserver on the Element
pub struct DomRectSignal {
    _observer: ResizeObserver,
    receiver: Receiver<DomRect>,
}

impl Signal for DomRectSignal {
    type Item = DomRect;

    #[inline]
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.receiver.poll_change_unpin(cx)
    }
}

impl DomRectSignal {
    pub fn new(element: &Element) -> Self {
        let (sender, receiver) = channel(element.get_bounding_client_rect());

        let observer = {
            let element = element.clone();
            let mut closed = false;

            ResizeObserver::new(&closure!(move || {
                if closed {
                    return;
                }
                if sender.send(element.get_bounding_client_rect()).is_err() {
                    //debug_log!(err);
                    sender.close();
                    closed = true;
                }
            }))
            .unwrap_js()
        };

        observer.observe(element);

        Self {
            _observer: observer,
            receiver,
        }
    }
}

/// Signal of attribute mutation records, driven by a MutationObserver on the Element
pub struct FreeEquipmentSlotsSignal {
    receiver: Receiver<u8>,
}

impl Signal for FreeEquipmentSlotsSignal {
    type Item = u8;

    #[inline]
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.receiver.poll_change_unpin(cx)
    }
}

impl FreeEquipmentSlotsSignal {
    pub fn new() -> Self {
        let hero_eqipment = get_engine().hero_equipment().unwrap_js();
        let (sender, receiver) = channel(hero_eqipment.get_free_slots());

        Emitter::register_after_on(EmitterEvent::Item, move |_| {
            if sender.send(hero_eqipment.get_free_slots()).is_err() {
                //common::debug_log!("ERROR ON FreeEquipmentSlots");
                sender.close();
            }

            Box::pin(async { Ok(()) })
        })
        .unwrap_js();

        Self { receiver }
    }
}
