use std::{collections::HashMap, sync::OnceLock};

use futures_signals::signal_map::MutableBTreeMap;

use crate::{
    bindings::engine::{
        communication::{Emotion, OtherData},
        other::Other,
    },
    color_mark::{Color, ColorMark},
    utils::JsResult,
};

use super::{GlobalBTreeMap, OtherId, addons::AddonName, peers::PeerBTreeMap, town::Town};

static OTHERS: OnceLock<OtherBTreeMap> = OnceLock::new();

/// MutableBTreeMap storing all players on the current map.
#[derive(Debug)]
pub struct OtherBTreeMap(MutableBTreeMap<OtherId, Other>);

impl OtherBTreeMap {
    pub(super) fn init() -> JsResult<()> {
        OTHERS
            .set(Self(MutableBTreeMap::new()))
            .map_err(|_| common::err_code!())?;

        ColorMark::observe_others_map();

        Ok(())
    }

    pub fn get() -> &'static Self {
        OTHERS.wait()
    }

    pub(crate) fn reload() {
        Self::get().lock_mut().clear();
    }

    pub(crate) fn merge(mut new_others: HashMap<OtherId, OtherData>) {
        new_others
            .drain()
            .for_each(|(char_id, new_other_data)| Self::on_other(char_id, new_other_data));
    }

    fn on_other(char_id: OtherId, new_other_data: OtherData) {
        let mut others_lock = Self::get().lock_mut();

        if new_other_data.del.is_some_and(|del| del == 1) {
            others_lock.remove(&char_id);

            if let Some(peer_data) = PeerBTreeMap::get().lock_mut().get(&char_id) {
                peer_data.map_name.set_neq(None);
                peer_data.x.set_neq(None);
                peer_data.y.set_neq(None);
            };

            return;
        }
        if let Some(x) = new_other_data.x
            && let Some(y) = new_other_data.y
            && let Some(peer_data) = PeerBTreeMap::get().lock_mut().get(&char_id)
        {
            peer_data.x.set_neq(Some(x));
            peer_data.y.set_neq(Some(y));

            // TODO: Update the map name only if other.action == "CREATE"
            if let Some(town_name) = Town::get().lock_ref().name.clone() {
                peer_data.map_name.set_neq(Some(town_name));
            }
        }

        match others_lock.get(&char_id) {
            Some(old_other_data) => old_other_data.update(char_id, new_other_data),
            None => match Other::new(char_id, new_other_data) {
                Ok(other) => {
                    others_lock.insert_cloned(char_id, other);
                }
                Err(err_code) => console_error!(err_code),
            },
        };
    }

    pub fn init_color_mark(color: Color, addon_name: AddonName, other_id: OtherId) -> JsResult<()> {
        ColorMark::init(color, addon_name, other_id)
    }

    pub(crate) fn init_emotion(emotion: Emotion) {
        if emotion.source_type != 1 {
            return;
        }
        if let Some(other) = Self::get().lock_ref().get(&emotion.source_id) {
            other.init_emotion(emotion);
        }
    }
}

impl GlobalBTreeMap<OtherId, Other> for OtherBTreeMap {
    fn get(&self) -> &MutableBTreeMap<OtherId, Other> {
        &self.0
    }
}
