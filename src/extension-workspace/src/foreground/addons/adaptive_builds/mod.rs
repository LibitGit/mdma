mod html;

use futures_signals::signal::{Mutable, SignalExt};
use proc_macros::{Setting, Settings};

use crate::prelude::*;

const ADDON_NAME: AddonName = AddonName::AdaptiveBuilds;

#[derive(Setting)]
struct ColossusBuild {
    active: Mutable<bool>,
    build: Mutable<u8>,
}

impl Default for ColossusBuild {
    fn default() -> Self {
        Self {
            active: Mutable::default(),
            build: Mutable::new(1),
        }
    }
}

#[derive(Settings, Default)]
struct Settings {
    colossus: ColossusBuild,
    #[setting(skip)]
    scroll_active: Mutable<bool>,
}

pub(crate) fn init() -> JsResult<()> {
    let settings = Settings::new(ADDON_NAME);

    let future = settings
        .colossus
        .build
        .signal()
        .switch(|_| settings.colossus.active.signal())
        .switch(move |colossus_active| {
            NpcTemplates::has_colossus_signal().map(move |map_has_colossus| {
                if !colossus_active || !Addons::is_active(ADDON_NAME) {
                    return false;
                }

                //common::debug_log!("MAP HAS COLOSSUS:", map_has_colossus);
                map_has_colossus
            })
        })
        .for_each(|change_build| {
            if change_build {
                let collosus_build = settings.colossus.build.get();
                // Update skill view if it's open
                let skills = get_engine().skills().map(|_| "&skillshop=1").unwrap_or_default();
                let task = format!("builds&action=updateCurrent&id={collosus_build}{skills}");
                if send_task(&task).is_err() {
                    console_error!()
                }
                let _ = message("[MDMA::RS] Zmieniam zestaw na kolosy...");
            }

            async {}
        });
    wasm_bindgen_futures::spawn_local(future);

    html::init(settings)
}
