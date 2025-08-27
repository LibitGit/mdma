use std::sync::OnceLock;

use common::err_code;
use futures_signals::{
    signal::{self, Signal, SignalExt},
    signal_map::MutableBTreeMap,
    signal_vec::SignalVecExt,
};
use wasm_bindgen::JsValue;

use crate::{
    bindings::engine::communication::{Id, NpcData, NpcDelData, NpcTemplate},
    utils::{JsResult, UnwrapJsExt},
};

use super::GlobalBTreeMap;

static NPCS: OnceLock<Npcs> = OnceLock::new();
static NPC_TEMPLATES: OnceLock<NpcTemplates> = OnceLock::new();

#[derive(Debug)]
pub struct Npcs(MutableBTreeMap<Id, Npc>);

impl Npcs {
    pub(super) fn init() -> JsResult<()> {
        NPCS.set(Npcs(MutableBTreeMap::new()))
            .map_err(|_| err_code!())
    }

    pub fn get() -> &'static Self {
        NPCS.wait()
    }

    pub fn on_npcs(new_npcs: Vec<NpcData>) {
        let mut npcs_lock = Self::get().0.lock_mut();
        new_npcs.into_iter().for_each(|npc_data| {
            npcs_lock.insert_cloned(npc_data.id.unwrap_js(), npc_data.try_into().unwrap_js());
        });
    }

    pub fn on_npcs_del(list: &Vec<NpcDelData>) {
        let mut npcs_lock = Self::get().0.lock_mut();
        list.iter()
            .filter_map(|npc| npc.id.as_ref())
            .for_each(|id| {
                npcs_lock.remove(id);
            });
    }

    pub(crate) fn on_clear() {
        Self::get().0.lock_mut().clear();
    }
}

impl GlobalBTreeMap<Id, Npc> for Npcs {
    fn get(&self) -> &MutableBTreeMap<Id, Npc> {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Npc {
    pub id: Id,
    pub template_id: Id,
    pub x: u8,
    pub y: u8,
    pub walkover: bool,
    pub group: Option<u16>,
}

impl TryFrom<NpcData> for Npc {
    type Error = JsValue;

    fn try_from(value: NpcData) -> Result<Self, Self::Error> {
        let NpcData {
            id,
            template_id,
            x,
            y,
            walkover,
            group,
        } = value;

        Ok(Self {
            id: id.ok_or_else(|| err_code!())?,
            template_id: template_id.ok_or_else(|| err_code!())?,
            x: x.ok_or_else(|| err_code!())?,
            y: y.ok_or_else(|| err_code!())?,
            walkover: walkover.unwrap_or_default(),
            group
        })
    }
}

impl Npc {
    pub fn has_collision(&self) -> bool {
        let templates_lock = NpcTemplates::get().lock_ref();
        let Some(npc_tpl) = templates_lock.get(&self.template_id) else {
            // debug_log!("NO TEMPLATE FOUND FOR", self.id);
            return true;
        };
        let Some(npc_type) = npc_tpl.npc_type else {
            // debug_log!("NO TYPE FOUND FOR", self.template_id);
            return true;
        };

        npc_type != 4 && npc_type != 7 && !self.walkover
    }
}

#[derive(Debug)]
pub struct NpcTemplates(MutableBTreeMap<Id, NpcTemplate>);

impl NpcTemplates {
    pub(super) fn init() -> JsResult<()> {
        NPC_TEMPLATES
            .set(NpcTemplates(MutableBTreeMap::new()))
            .map_err(|_| err_code!())
    }

    pub fn get() -> &'static Self {
        NPC_TEMPLATES.wait()
    }

    pub fn on_npc_tpls(new_npc_tpls: Vec<NpcTemplate>) {
        let mut npcs_lock = Self::get().0.lock_mut();
        new_npc_tpls
            .into_iter()
            .filter_map(|npc_tpl| npc_tpl.id.map(|npc_id| (npc_id, npc_tpl)))
            .for_each(|(npc_id, npc_tpl)| {
                npcs_lock.insert_cloned(npc_id, npc_tpl);
            });
    }

    //fn has_colossus(&self) -> bool {
    //    self.0
    //        .lock_ref()
    //        .iter()
    //        .any(|(_, npc_tpl)| npc_tpl.warrior_type.is_some_and(|wt| wt > 89 && wt < 100))
    //}

    pub(crate) fn has_colossus_signal() -> impl Signal<Item = bool> {
        signal::not(
            Self::get()
                .entries_cloned()
                .filter(|(_, npc_tpl)| npc_tpl.warrior_type.is_some_and(|wt| wt > 89 && wt < 100))
                .is_empty(),
        )
        .dedupe()
    }

    pub(crate) fn on_clear() {
        Self::get().0.lock_mut().clear();
    }
}

impl GlobalBTreeMap<Id, NpcTemplate> for NpcTemplates {
    fn get(&self) -> &MutableBTreeMap<Id, NpcTemplate> {
        &self.0
    }
}
