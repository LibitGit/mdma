use common::map_err;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{
    bindings::engine::communication::{Id, Profession},
    globals::{GlobalBTreeMap, players_online::PlayersOnline, world_config::WorldConfig},
    utils::JsResult,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type ShowEqManager;

    #[wasm_bindgen(catch, method, js_name = "update")]
    fn __update(this: &ShowEqManager, player_data: &JsValue) -> JsResult<JsValue>;
}

#[derive(Serialize)]
pub struct ShowEqPlayerData {
    id: Id,
    account: u32,
    lvl: u16,
    nick: String,
    prof: Profession,
    icon: String,
    world: String,
}

impl ShowEqPlayerData {
    pub fn new_with_other(other: &super::other::Other) -> Option<Self> {
        let id = other.char_id;
        let account = other.account;
        //.or_else(|| Some(globals.players_online.lock_ref().get(&id)?.account))
        //.or_else(|| globals.others.lock_ref().get(&id)?.account)?;

        Some(Self {
            id,
            account,
            icon: String::new(),
            prof: other.prof.get(),
            lvl: other.lvl.get(),
            nick: other.nick.get_cloned(),
            world: WorldConfig::world_name(),
        })
    }

    pub fn new_with_peer(peer: &super::peer::Peer) -> Option<Self> {
        let id = peer.char_id;
        let account = peer
            .account
            .get()
            .or_else(|| Some(PlayersOnline::get().lock_ref().get(&id)?.account))?;
        //.or_else(|| globals.peers.lock_ref().get(&id)?.account)?;

        Some(Self {
            id,
            account,
            icon: String::new(),
            prof: peer.prof.get(),
            lvl: peer.lvl.get(),
            nick: peer.nick.get_cloned(),
            world: WorldConfig::world_name(),
        })
    }
}

impl ShowEqManager {
    pub fn update(&self, player_data: &ShowEqPlayerData) -> JsResult<()> {
        let player_data = serde_wasm_bindgen::to_value(player_data).map_err(map_err!())?;
        self.__update(&player_data)?;

        Ok(())
    }
}
