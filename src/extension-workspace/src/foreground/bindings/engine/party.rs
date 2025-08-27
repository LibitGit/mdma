use futures_signals::signal::Mutable;
use js_sys::Object;
use wasm_bindgen::prelude::*;

use crate::globals::{GlobalBTreeMap, OtherId, others::OtherBTreeMap, peers::PeerBTreeMap};

use super::{PartyMemberData, Profession};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type PartyManager;

    #[wasm_bindgen(method, js_name = "getMembers")]
    pub(crate) fn get_members(this: &PartyManager) -> Object;
}

#[derive(Debug, Clone)]
pub struct PartyMember {
    pub account: u32,
    pub char_id: OtherId,
    pub commander: Mutable<bool>,
    pub hp_cur: Mutable<u32>,
    pub hp_max: Mutable<u32>,
    pub icon: Mutable<String>,
    pub nick: Mutable<String>,
    pub profession: Mutable<Option<Profession>>,
}

impl PartyMember {
    // TODO: Check profession from online list.
    pub(crate) fn new(party_member_data: PartyMemberData) -> Self {
        let PartyMemberData {
            account,
            commander,
            hp_cur,
            hp_max,
            icon,
            char_id,
            nick,
        } = party_member_data;

        let profession = OtherBTreeMap::get()
            .lock_ref()
            .get(&char_id)
            .map(|other_data| other_data.prof.get())
            .or_else(|| {
                PeerBTreeMap::get()
                    .lock_ref()
                    .get(&char_id)
                    .map(|peer| peer.prof.get())
            });
        Self {
            account,
            char_id,
            commander: Mutable::new(commander),
            hp_cur: Mutable::new(hp_cur),
            hp_max: Mutable::new(hp_max),
            icon: Mutable::new(icon),
            nick: Mutable::new(nick),
            profession: Mutable::new(profession),
        }
    }

    pub(crate) fn update(&self, new_party_member_data: PartyMemberData) {
        let PartyMemberData {
            commander,
            hp_cur,
            hp_max,
            icon,
            nick,
            ..
        } = new_party_member_data;

        self.commander.set_neq(commander);
        self.hp_cur.set_neq(hp_cur);
        self.hp_max.set_neq(hp_max);
        self.icon.set_neq(icon);
        self.nick.set_neq(nick);
    }
}
