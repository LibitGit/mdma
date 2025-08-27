use std::{collections::BTreeMap, sync::OnceLock};

use common::{err_code, map_err};
use futures_signals::{signal::Mutable, signal_map::MutableBTreeMap};
use gloo_net::http::{Method, RequestBuilder};
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use wasm_bindgen::intern;

use crate::{bindings::is_pl, s, utils::JsResult};

use super::{GlobalBTreeMap, OtherId, world_config::WorldConfig};

static PLAYERS_ONLINE: OnceLock<PlayersOnline> = OnceLock::new();

#[derive(Debug)]
pub struct PlayersOnline {
    /// Online players map where the key is the character id of a player.
    players_map: MutableBTreeMap<OtherId, PlayerOnlineData>,
    last_updated: Mutable<String>,
}

impl PlayersOnline {
    pub(super) fn init() -> JsResult<()> {
        PLAYERS_ONLINE
            .set(Self {
                players_map: MutableBTreeMap::new(),
                last_updated: Mutable::default(),
            })
            .map_err(|_| err_code!())
    }

    pub fn get() -> &'static Self {
        PLAYERS_ONLINE.wait()
    }

    pub(super) async fn update() -> JsResult<bool> {
        let world_name = WorldConfig::world_name();

        if world_name.is_empty() {
            return Ok(false);
        }

        let url = format!(
            "{}{}{}",
            match is_pl().map_err(map_err!())? {
                true => intern("https://staticinfo.margonem.pl/online/"),
                false => intern("https://staticinfo.margonem.com/online/"),
            },
            if world_name == "experimental" {
                "fobos"
            } else {
                world_name.as_str()
            },
            s!(".json"),
        );
        let response = RequestBuilder::new(&url)
            .method(Method::GET)
            .build()
            .map_err(map_err!(from))?
            .send()
            .await
            .map_err(map_err!(from))?;

        Self::get().last_updated.set_neq(
            response
                .headers()
                .get("last-modified")
                .ok_or_else(|| err_code!())?,
        );

        let players_list: Vec<PlayerOnlineData> = response.json().await.map_err(map_err!(from))?;
        let players_map = players_list
            .into_iter()
            .map(|player| (player.char_id, player))
            .collect::<BTreeMap<_, _>>();

        Self::get()
            .players_map
            .lock_mut()
            .replace_cloned(players_map);

        Ok(true)
    }

    pub fn currently_online() -> usize {
        Self::get().players_map.lock_ref().len()
    }
}

impl GlobalBTreeMap<OtherId, PlayerOnlineData> for PlayersOnline {
    fn get(&self) -> &MutableBTreeMap<OtherId, PlayerOnlineData> {
        &self.players_map
    }
}

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct PlayerOnlineData {
    #[serde(rename = "a")]
    #[serde_as(as = "DisplayFromStr")]
    pub account: u32,
    #[serde(rename = "c")]
    #[serde_as(as = "DisplayFromStr")]
    pub char_id: OtherId,
    //#[serde(rename = "l")]
    //#[serde_as(as = "DisplayFromStr")]
    //pub level: u16,
    //#[serde(rename = "n")]
    //pub nick: String,
    //#[serde(rename = "p")]
    //#[serde_as(as = "DisplayFromStr")]
    //pub prof: Profession,
    //#[serde(rename = "r")]
    //#[serde_as(as = "DisplayFromStr")]
    //pub rank: u8,
}
