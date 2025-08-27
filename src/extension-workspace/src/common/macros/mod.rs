//TODO: Use tt parsing instead of whatever this is currently.
//TODO: Use $to_clone:ident instead of statements ?
//TODO: Add rest of implementations.
//TODO: Use future_to_promise for async closures that return Result.
#[macro_export]
macro_rules! closure {
    // sync no calle with move no args FnOnce
    (
        @once
        $({ $($var_statement:stmt),+ $(,)? },)?
        move || $(-> $return_type:ty)? $code:block $(,)?
    ) => {{
        use std::boxed::Box;

        use ::wasm_bindgen::prelude::*;

        $($($var_statement)+)?

        let closure = move || $(-> $return_type)? { $code };
        let closure = Closure::once_into_js(closure);

        closure.unchecked_into::<::js_sys::Function>()
    }};
    // sync no calle with move with args FnOnce
    (
        @once
        $({ $($var_statement:stmt),+ $(,)? },)?
        move |$($arg_name:ident : $arg_type:ty ),* $(,)?| $(-> $return_type:ty)? $code:block $(,)?
    ) => {{
        use std::boxed::Box;

        use ::wasm_bindgen::prelude::*;

        $($($var_statement)+)?

        let closure = move |$($arg_name: $arg_type,)*| $(-> $return_type)? { $code };
        let closure = Closure::once_into_js(closure);

        closure.unchecked_into::<::js_sys::Function>()
    }};
    // sync no callee no move no args
    (|| $(-> $return_type:ty)? $code:block $(,)?) => {{
        use std::boxed::Box;

        use ::wasm_bindgen::prelude::*;

        let closure = || $(-> $return_type)? { $code };
        let closure = Closure::wrap(Box::new(closure) as Box<dyn FnMut() $(-> $return_type)?>);

        closure.into_js_value().unchecked_into::<::js_sys::Function>()
    }};
    // sync no callee with move no args
    (
        $({ $($var_statement:stmt),+ $(,)? },)?
        move || $(-> $return_type:ty)? $code:block $(,)?
    ) => {{
        use std::boxed::Box;

        use ::wasm_bindgen::prelude::*;

        $($($var_statement)+)?

        let closure = move || $(-> $return_type)? { $code };
        let closure = Closure::wrap(Box::new(closure) as Box<dyn FnMut() $(-> $return_type)?>);

        closure.into_js_value().unchecked_into::<::js_sys::Function>()
    }};
    // async no callee no move no args
    // async no callee with move no args
    // sync no callee no move
    (|$($arg_name:ident : $arg_type:ty ),* $(,)?| $(-> $return_type:ty)? $code:block $(,)?) => {{
        use std::boxed::Box;

        use ::wasm_bindgen::prelude::*;

        let closure = |$($arg_name: $arg_type,)*| $(-> $return_type)? { $code };
        let closure = Closure::wrap(Box::new(closure) as Box<dyn FnMut($($arg_type,)*) $(-> $return_type)?>);

        closure.into_js_value().unchecked_into::<::js_sys::Function>()
    }};
    // sync no callee with move
    (
        $({ $($var_statement:stmt),+ $(,)? },)?
        move |$($arg_name:ident : $arg_type:ty ),* $(,)?| $(-> $return_type:ty)? $code:block $(,)?
    ) => {{
        use std::boxed::Box;

        use ::wasm_bindgen::prelude::*;

        $($($var_statement)+)?

        let closure = move |$($arg_name: $arg_type,)*| $(-> $return_type)? { $code };
        let closure = Closure::wrap(Box::new(closure) as Box<dyn FnMut($($arg_type,)*) $(-> $return_type)?>);

        closure.into_js_value().unchecked_into::<::js_sys::Function>()
    }};
    //async no callee no move
    (|$($arg_name:ident : $arg_type:ty ),* $(,)?| async move $code:block $(,)?) => {{
        use std::boxed::Box;

        use ::wasm_bindgen::prelude::*;

        let closure = |$($arg_name: $arg_type),*| ::wasm_bindgen_futures::spawn_local(async move $code);
        let closure = Closure::wrap(Box::new(closure) as Box<dyn FnMut($($arg_type,)*)>);

        closure.into_js_value().unchecked_into::<::js_sys::Function>()
    }};
    //async no callee with move
    (
        $({ $($var_statement:stmt),+ $(,)? },)?
        move |$($arg_name:ident : $arg_type:ty ),* $(,)?| async move $code:block $(,)?
    ) => {{
        use std::boxed::Box;

        use ::wasm_bindgen::prelude::*;

        $($($var_statement)+)?

        let closure = move |$($arg_name: $arg_type),*| {
            $($($var_statement)+)?

            ::wasm_bindgen_futures::spawn_local(async move $code);
        };
        let closure = Closure::wrap(Box::new(closure) as Box<dyn FnMut($($arg_type,)*)>);

        closure.into_js_value().unchecked_into::<::js_sys::Function>()
    }};
}
