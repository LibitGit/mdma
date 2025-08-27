//TODO: Do not send requests on captcha.
#![feature(closure_track_caller)]
#![feature(closure_lifetime_binder)]
#![feature(option_zip)]
// #![deny(
//     bad_style,
//     dead_code,
//     improper_ctypes,
//     non_shorthand_field_patterns,
//     no_mangle_generic_items,
//     overflowing_literals,
//     path_statements,
//     patterns_in_fns_without_body,
//     private_interfaces,
//     private_bounds,
//     unconditional_recursion,
//     unused,
//     unused_allocation,
//     unused_comparisons,
//     unused_parens,
//     while_true,
//     missing_debug_implementations,
//     missing_copy_implementations,
//     //missing_docs,
//     trivial_casts,
//     trivial_numeric_casts,
//     unused_import_braces,
//     //unused_results
// )]

pub mod prelude {
    pub use crate::bindings::prelude::*;
    pub use crate::globals::prelude::*;
    pub use crate::utils::*;
    pub use crate::{class, console_error, s, string};
    pub use common::{closure, debug_log, err_code, map_err};
    pub use proc_macros::{ActiveSettings, Setting, Settings};
}

pub use obfstr::obfstr as s;
pub use wasm_bindgen::prelude::*;

mod addon_window;
mod addons;
mod bindings;
pub mod disable_items;
mod dispatcher;
mod interface;
mod pathfinder;
mod utils;
#[macro_use]
mod macros;
// TODO: Move to a different directory/module ?
mod color_mark;
pub mod globals;

#[wasm_bindgen]
pub async fn main(original_init: js_sys::Function) -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    #[cfg(debug_assertions)]
    console_log::init_with_level(log::Level::Trace).unwrap();

    let is_main_page = utils::window().location().href().is_ok_and(|href| {
        href == s!("https://www.margonem.pl/") || href == s!("https://www.margonem.com/")
    });
    if is_main_page {
        web_sys::console::log_5(
            &JsValue::from_str(s!("%c MDMA %c %c Rust ")),
            &JsValue::from_str(s!(
                "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &JsValue::from_str(s!("")),
            &JsValue::from_str(s!(
                "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &JsValue::from_str(s!("Addon manager does not work on the main page, yet :)")),
        );
        return Ok(());
    }

    let communication = prelude::get_engine()
        .communication()
        .ok_or_else(|| common::err_code!())?;

    if let Err(err) = init_manager(&communication).await {
        web_sys::console::error_5(
            &JsValue::from_str(s!("%c MDMA %c %c Rust ")),
            &JsValue::from_str(s!(
                "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &JsValue::from_str(s!("")),
            &JsValue::from_str(s!(
                "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
            )),
            &err,
        );
        let _ = prelude::message(s!("[MDMA::RS] Błąd podczas wczytywania zestawu!"));
    }

    communication.try_init_game(&original_init)
}

async fn init_manager(
    communication: &bindings::engine::communication::Communication,
) -> Result<(), JsValue> {
    let manager_globals = match globals::Globals::init().await {
        Ok(manager_globals) => manager_globals,
        Err(globals::GlobalsError::Unrecoverable(err_code)) => return Err(err_code),
        Err(globals::GlobalsError::Unauthorized) => return interface::render_unauthorized(),
    };

    #[cfg(feature = "ni")]
    {
        // TODO: Make checks that items module is ready.
        prelude::get_engine()
            .items_manager()
            .ok_or_else(|| common::err_code!())?
            .observe_update_placeholder()?;
        prelude::get_game_api().observe_call_event()?;
        // globals::emitter::Emitter::on_call_draw_add_to_renderer()?;
    }

    communication.observe_send()?;

    interface::init_windows_layer()?;

    globals::addons::init_addons!();

    interface::init_interface(manager_globals)?;

    wasm_bindgen_futures::spawn_local(globals::Globals::start_peers_map_update_interval());
    wasm_bindgen_futures::spawn_local(globals::Globals::start_players_online_update_interval());
    wasm_bindgen_futures::spawn_local(
        bindings::engine::settings::Settings::init_server_option_config(),
    );

    Ok(())
}
