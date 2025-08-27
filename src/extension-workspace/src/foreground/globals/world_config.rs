use std::cell::RefCell;

use crate::bindings::engine::communication::WorldConfigData;

thread_local! {
    static WORLD_CONFIG: RefCell<WorldConfig> = const { RefCell::new(WorldConfig::new()) };
}

#[derive(Debug)]
pub struct WorldConfig {
    pub world_name: String,
    pub npc_resp: f32,
}

impl WorldConfig {
    const FRACTION_WORLD: &'static str = "perkun";

    const fn new() -> Self {
        Self {
            world_name: String::new(),
            npc_resp: 0.0,
        }
    }

    pub fn world_name() -> String {
        WORLD_CONFIG.with_borrow(|cfg| cfg.world_name.clone())
    }

    pub fn npc_resp() -> f32 {
        WORLD_CONFIG.with_borrow(|cfg| cfg.npc_resp)
    }

    pub fn has_fractions() -> bool {
        WORLD_CONFIG.with_borrow(|cfg| cfg.world_name == Self::FRACTION_WORLD)
    }

    pub(crate) fn merge(new_world_config: WorldConfigData) {
        let WorldConfigData {
            world_name,
            npc_resp,
        } = new_world_config;

        WORLD_CONFIG.with_borrow_mut(|world_config| {
            if let Some(world_name) = world_name {
                world_config.world_name = world_name;
            }
            if let Some(npc_resp) = npc_resp {
                world_config.npc_resp = npc_resp;
            }
        });
    }
}
