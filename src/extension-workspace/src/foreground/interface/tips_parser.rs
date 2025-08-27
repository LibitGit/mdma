use std::cell::RefCell;

//TODO: Remove game tip on mdma tip shown.
use dominator::DomHandle;
use futures_signals::signal::Mutable;

type Left = Mutable<Option<i32>>;
type Top = Mutable<Option<i32>>;
type Offset = (Left, Top);

thread_local! {
    pub(crate) static OFFSET: Offset = Offset::default();
    pub(crate) static HANDLE: RefCell<Option<(DomHandle, Mutable<bool>)>> = const { RefCell::new(None) };
}

//TODO: Tip doesnt hide when another tip moves in onto mouse.
macro_rules! tip {
    ($this:ident, { $($input:tt)* } $(,)?) => {{
        use ::dominator::{clone, html, with_node, window_size, WindowSize};
        use ::futures_signals::map_ref;
        use ::futures_signals::signal::{Mutable, SignalExt};
        use ::discard::Discard;
        use ::obfstr::obfstr as s;
        use ::wasm_bindgen::intern;

        use $crate::interface::tips_parser::{OFFSET, HANDLE};
        use $crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT};

        let shown = Mutable::new(false);
        let on_mouse_over = clone!(shown => move |event: $crate::utils::window_events::MouseOver| {
            if shown.get() {
                return
            }

            shown.set(true);
            OFFSET.with(|(left, top)| {
                left.set(Some(event.mouse_x()));
                top.set(Some(event.mouse_y()));
            });

            let tip: ::dominator::Dom = ::dominator::html!(s!("div"), {
                .class(s!("tip-wrapper"))
                .child(html!(s!("div"), {
                    .class(s!("content"))
                    $($input)*
                }))
                .with_node!(tip_node => {
                    .style_signal("left", OFFSET.with(|(left, _)| map_ref! {
                        let left = left.signal(),
                        let window_width = window_size().map(|WindowSize { width, .. }| width as i32) =>
                            left.map(|left| {
                            let rect_width = tip_node.get_bounding_client_rect().width() as i32;
                            let mut offset_left = match left + 30 > window_width - 15 - rect_width  {
                                true => (left - 30 - rect_width).to_string(),
                                false => (left + 30).to_string(),
                            };
                            offset_left.push_str(intern(s!("px")));
                            offset_left
                        })
                    }))
                })
                .with_node!(tip_node => {
                    .style_signal("top", OFFSET.with(|(_, top)| map_ref! {
                        let top = top.signal(),
                        let window_height = window_size().map(|WindowSize { height, .. }| height as i32) =>
                        top.map(|top| {
                            let rect_height = tip_node.get_bounding_client_rect().height() as i32;
                            let mut offset_top = top.clamp(8, window_height - 8 - rect_height).to_string();
                            offset_top.push_str(intern(s!("px")));
                            offset_top
                        })
                    }))
                })
            });

            if let Some(handle) = WINDOWS_ROOT.try_append_dom(tip) {
                HANDLE.with_borrow_mut(|handle_ref| {
                    if let Some((old_tip, _)) = std::mem::replace(handle_ref, Some((handle, shown.clone()))) {
                        old_tip.discard();
                        //old_shown.set_neq(false);
                    }
                });
            }
        });

        ::dominator::apply_methods!($this, {
            .event(on_mouse_over)
            .event(|event: ::dominator::events::MouseMove| {
                OFFSET.with(|(left, top)| {
                    left.set(Some(event.mouse_x()));
                    top.set(Some(event.mouse_y()));
                });
            })
            .event(clone!(shown => move |_: ::dominator::events::MouseLeave| {
                // TODO: remove if below is true.
                shown.set_neq(false);
                OFFSET.with(|(left, top)| {
                    left.set(None);
                    top.set(None);
                });

                HANDLE.with(|handle_ref| {
                    // TODO: Verify that wildcard is always the tip from this macro invocation not
                    // some tip from a wrapper or child elements.
                    if let Some((handle, _)) = handle_ref.take() {
                        handle.discard();
                    }
                });
            }))
            .after_removed(move |_| {
                if !shown.get() {
                    return;
                }
                shown.set(false);
                HANDLE.with(|handle_ref| {
                    // TODO: Verify that wildcard is always the tip from this macro invocation not
                    // some tip from a wrapper or child elements.
                    if let Some((handle, _)) = handle_ref.take() {
                        handle.discard();
                    }
                });
            })
        })
    }};
    ($this:ident, $(@statements { $($var_statement:stmt),+ $(,)? },)? $active_signal:expr => { $($input:tt)* } $(,)?) => {{
        use ::dominator::{html, clone, with_node, window_size, WindowSize};
        use ::futures_signals::map_ref;
        use ::futures_signals::signal::SignalExt;
        use ::discard::Discard;
        use ::obfstr::obfstr as s;
        use ::wasm_bindgen::intern;

        use $crate::interface::tips_parser::{OFFSET, HANDLE};
        use $crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT};

        let mouse_entered = Mutable::new(false);
        let active_signal = $active_signal;

        $(
            $(
                    $var_statement
            )+
        )?
        let active = map_ref! {
            let active_signal = active_signal.dedupe(),
            let mouse_entered = mouse_entered.signal() => {
                *active_signal && *mouse_entered
            }
        };

        let mut skip = true;
        ::wasm_bindgen_futures::spawn_local(active.for_each(clone!(mouse_entered => move |active| {
            match active {
                true => {
                    let tip = ::dominator::html!(s!("div"), {
                        .class(s!("tip-wrapper"))
                        .style(s!("position"), s!("absolute"))
                        .child(html!("div", {
                            .class(s!("content"))
                            $($input)*
                        }))
                        .with_node!(tip_node => {
                            .style_signal("left", OFFSET.with(|(left, _)| map_ref! {
                                let left = left.signal(),
                                let window_width = window_size().map(|WindowSize { width, .. }| width as i32) =>
                                left.map(|left| {
                                    let rect_width = tip_node.get_bounding_client_rect().width() as i32;
                                    let mut offset_left = match left + 30 > window_width - 15 - rect_width  {
                                        true => (left - 30 - rect_width).to_string(),
                                        false => (left + 30).to_string(),
                                    };
                                    offset_left.push_str(intern(s!("px")));
                                    offset_left
                                })
                            }))
                        })
                        .with_node!(tip_node => {
                            .style_signal("top", OFFSET.with(|(_, top)| map_ref! {
                                let top = top.signal(),
                                let window_height = window_size().map(|WindowSize { height, .. }| height as i32) =>
                                top.map(|top| {
                                    let rect_height = tip_node.get_bounding_client_rect().height() as i32;
                                    let mut offset_top = top.clamp(8, window_height - 8 - rect_height).to_string();
                                    offset_top.push_str(intern(s!("px")));
                                    offset_top
                                })
                            }))
                        })
                    });
                    HANDLE.with_borrow_mut(|handle_ref| {
                        if let Some(handle) = WINDOWS_ROOT.try_append_dom(tip) {
                            if let Some((old_tip, _old_shown)) = handle_ref.take() {
                                old_tip.discard();
                                // common::debug_log!("old_shown:", _old_shown.get());
                                //common::js_imports::breakpoint();
                                //old_shown.set_neq(false);
                            }
                            *handle_ref = Some((handle, mouse_entered.clone()));
                            //if let Some((old_tip, old_shown)) = std::mem::replace(handle_ref, Some((handle, mouse_entered.clone()))) {
                            // common::debug_log!("AFTER SET");
                            //    old_tip.discard();
                            //    old_shown.set_neq(false);
                            //}
                        }
                    });

                    skip = false;
                }
                false => {
                    match skip {
                        true => skip = false,
                        // TODO: Verify that wildcard is always the tip from this macro invocation not
                        // some tip from a wrapper or child elements.
                        false => HANDLE.with(|handle_ref| if let Some((handle, _)) = handle_ref.take() {
                            // common::debug_log!("deleting handle...");
                            //common::js_imports::breakpoint();
                            handle.discard();
                        }),
                    };
                },
            };

            async {}
        })));

        let on_mouse_over = clone!(mouse_entered => move |event: $crate::utils::window_events::MouseOver| {
            OFFSET.with(|(left, top)| {
                left.set(Some(event.mouse_x()));
                top.set(Some(event.mouse_y()));
            });
            let from_handle = HANDLE.with_borrow(|handle_ref| handle_ref.as_ref().is_none_or(|(_, old_shown)| !old_shown.get()));
            if from_handle || !mouse_entered.get() {
                mouse_entered.set(true);
            }
            //mouse_entered.set_neq(true);
        });

        ::dominator::apply_methods!($this, {
            .event(on_mouse_over)
            .event(|event: ::dominator::events::MouseMove| {
                OFFSET.with(|(left, top)| {
                    left.set(Some(event.mouse_x()));
                    top.set(Some(event.mouse_y()));
                });
            })
            .event(clone!(mouse_entered => move |_: ::dominator::events::MouseLeave| {
                OFFSET.with(|(left, top)| {
                    left.set(None);
                    top.set(None);
                });

                mouse_entered.set_neq(false);
            }))
            .after_removed(move |_| {
                mouse_entered.set_neq(false);
            })
        })
    }};
}

pub(crate) use tip;

macro_rules! info_bubble {
    ($this:ident, { $($input:tt)* }) => {{
        use $crate::interface::tips_parser::tip;

        ::dominator::apply_methods!($this, {
            .child(html!("div", {
                .class("info-bubble")
                .tip!({ $($input)* })
            }))
        })
    }};
}

pub(crate) use info_bubble;
