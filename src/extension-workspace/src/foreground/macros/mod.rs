#[macro_export]
macro_rules! console_error {
    () => {{
        use ::wasm_bindgen::{JsValue, intern};

        let _ = $crate::bindings::message(intern($crate::s!("[MDMA::RS] Wystąpił błąd!")));
        let error_code = ::common::err_code!();
        ::web_sys::console::error_5(
            &JsValue::from_str($crate::s!("%c MDMA %c %c Rust ")),
            &JsValue::from_str($crate::s!(
                "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &JsValue::from_str($crate::s!("")),
            &JsValue::from_str($crate::s!(
                "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &error_code,
        );
        $crate::utils::logging::__internal_console_error(error_code);
    }};
    ($error_code:expr) => {{
        use ::wasm_bindgen::{JsValue, intern};

        let _ = $crate::bindings::message(intern($crate::s!("[MDMA::RS] Wystąpił błąd!")));
        ::web_sys::console::error_5(
            &JsValue::from_str($crate::s!("%c MDMA %c %c Rust ")),
            &JsValue::from_str($crate::s!(
                "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &JsValue::from_str($crate::s!("")),
            &JsValue::from_str($crate::s!(
                "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &$error_code,
        );
        $crate::utils::logging::__internal_console_error($error_code);
    }};
}

// #[macro_export]
// macro_rules! s {
//     //($(let $name:ident = $s:expr;)*) => { ... };
//     //($name:ident = $s:expr) => { ... };
//     //($buf:ident <- $s:expr) => { ... };
//     ($s:expr) => {{
//         // #[cfg(debug_assertions)]
//         // {
//         //     const _: &'static str = $s;
//         //     $s
//         // }

//         // #[cfg(not(debug_assertions))]
//         ::obfstr::obfstr!($s)
//     }};
// }

#[macro_export]
macro_rules! string {
    //($(let $name:ident = $s:expr;)*) => { ... };
    //($name:ident = $s:expr) => { ... };
    //($buf:ident <- $s:expr) => { ... };
    ($s:expr) => {{
        #[cfg(debug_assertions)]
        {
            const _: &'static str = $s;
            String::from($s)
        }

        #[cfg(not(debug_assertions))]
        ::obfstr::obfstring!($s)
    }};
}

///Always make sure each `setting` has the same name as the struct value.
#[macro_export]
macro_rules! init_active_setting {
    ($addon_name:expr, $setting:ident, $port:expr, $hero:expr) => {{
        use ::serde_json::json;
        use ::futures_signals::signal::SignalExt;
        use ::futures::stream::StreamExt;
        use ::dominator::clone;
        use ::common::debug_log;

        let port = $port;
        let hero = $hero;
        let future = $setting
            .signal_ref(|change| json!({ stringify!($setting): change }))
            .to_stream()
            .skip(1)
            .for_each(clone!(port, hero => move |json_change| {
                debug_log!(&format!("{json_change:?}"));
                let port = port.clone();
                let hero = hero.clone();
                async move {
                    port.send_active_settings_change($addon_name, json_change, &hero).await
                }
            }));
        ::wasm_bindgen_futures::spawn_local(future);
    }};
}

///Always make sure each `setting` has the same name as the struct value.
#[macro_export]
macro_rules! init_setting {
    ($addon_name:expr, $setting:ident, $port:expr, $hero:expr) => {{
        use ::serde_json::json;
        use ::futures_signals::signal::SignalExt;
        use ::futures::stream::StreamExt;
        use ::dominator::clone;
        use ::common::debug_log;

        let port = $port;
        let hero = $hero;
        let future = $setting
            .signal_ref(|change| json!({ stringify!($setting): change }))
            .to_stream()
            .skip(1)
            .for_each(clone!(port, hero => move |json_change| {
                debug_log!(&format!("{json_change:?}"));
                let port = port.clone();
                let hero = hero.clone();
                async move {
                    port.send_settings_change($addon_name, json_change, &hero).await
                }
            }));
        ::wasm_bindgen_futures::spawn_local(future);
    }};
}

#[macro_export]
macro_rules! class {
    ($this:ident, $($style:ident $( - $specifier:ident)* $( [ $($value:tt)+ ] )?)+) => {{
        ::dominator::apply_methods!($this, {
            $(.class(concat!(stringify!($style), $( '-', stringify!($specifier), )* $( '[', $( stringify!($value), )+ ']' )?)))+
        })
    }};
}

#[macro_export]
macro_rules! make_event {
    ($name:ident => $event:path) => {
        #[derive(Debug)]
        pub(crate) struct $name {
            event: $event,
        }

        //impl $name {
        //    //#[inline]
        //    //pub(crate) fn prevent_default(&self) {
        //    //    self.event.prevent_default();
        //    //}
        //    //
        //    //#[inline]
        //    //pub(crate) fn stop_propagation(&self) {
        //    //    self.event.stop_propagation();
        //    //}
        //
        //    #[inline]
        //    pub(crate) fn stop_immediate_propagation(&self) {
        //        self.event.stop_immediate_propagation();
        //    }
        //    //
        //    //#[inline]
        //    //pub(crate) fn target(&self) -> Option<::web_sys::EventTarget> {
        //    //    self.event.target()
        //    //}
        //    //
        //    //#[inline]
        //    //pub(crate) fn dyn_target<A>(&self) -> Option<A>
        //    //where
        //    //    A: JsCast,
        //    //{
        //    //    self.target()?.dyn_into().ok()
        //    //}
        //}
    };
}

#[macro_export]
macro_rules! make_keyboard_event {
    ($name:ident) => {
        $crate::make_event!($name => web_sys::KeyboardEvent);

        //impl $name {
        //     //TODO: return enum or something
        //     #[inline] pub(crate) fn key(&self) -> String { self.event.key() }
        //
        //     #[inline] pub(crate) fn ctrl_key(&self) -> bool { self.event.ctrl_key() || self.event.meta_key() }
        //     #[inline] pub(crate) fn shift_key(&self) -> bool { self.event.shift_key() }
        //     #[inline] pub fn alt_key(&self) -> bool { self.event.alt_key() }
        //     #[inline] pub fn repeat(&self) -> bool { self.event.repeat() }
        //}
    };
}

#[macro_export]
macro_rules! make_mouse_event {
    ($name:ident => $event:path) => {
        $crate::make_event!($name => $event);

        impl $name {
            //#[inline] pub fn x(&self) -> i32 { self.event.client_x() }
            //#[inline] pub fn y(&self) -> i32 { self.event.client_y() }
            //
            //#[inline] pub fn movement_x(&self) -> i32 { self.event.movement_x() }
            //#[inline] pub fn movement_y(&self) -> i32 { self.event.movement_y() }
            //
            //#[inline] pub fn offset_x(&self) -> i32 { self.event.offset_x() }
            //#[inline] pub fn offset_y(&self) -> i32 { self.event.offset_y() }
            //
            //#[inline] pub fn page_x(&self) -> i32 { self.event.page_x() }
            //#[inline] pub fn page_y(&self) -> i32 { self.event.page_y() }
            //
            //#[inline] pub fn screen_x(&self) -> i32 { self.event.screen_x() }
            //#[inline] pub fn screen_y(&self) -> i32 { self.event.screen_y() }
            //
            //#[inline] pub fn ctrl_key(&self) -> bool { self.event.ctrl_key() || self.event.meta_key() }
            //#[inline] pub fn shift_key(&self) -> bool { self.event.shift_key() }
            //#[inline] pub fn alt_key(&self) -> bool { self.event.alt_key() }

            // TODO maybe deprecate these ?
            #[inline] pub fn mouse_x(&self) -> i32 { self.event.client_x() }
            #[inline] pub fn mouse_y(&self) -> i32 { self.event.client_y() }

            //pub fn button(&self) -> ::dominator::events::MouseButton {
            //    use ::dominator::events::MouseButton;
            //
            //    match self.event.button() {
            //        0 => MouseButton::Left,
            //        1 => MouseButton::Middle,
            //        2 => MouseButton::Right,
            //        3 => MouseButton::Button4,
            //        4 => MouseButton::Button5,
            //        _ => unreachable!("Unexpected MouseEvent.button value"),
            //    }
            //}
        }
    };
}

#[macro_export]
macro_rules! static_event_impl {
    ($name:ident => $type:literal) => {
        impl dominator::traits::StaticEvent for $name {
            const EVENT_TYPE: &'static str = $type;

            #[inline]
            fn unchecked_from_event(event: web_sys::Event) -> Self {
                Self {
                    event: event.unchecked_into(),
                }
            }
        }
    };
}
