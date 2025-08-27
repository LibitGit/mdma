use std::sync::OnceLock;

use futures_signals::signal_map::MutableBTreeMap;

use crate::{
    bindings::engine::{communication::PartyData, party::PartyMember},
    utils::JsResult,
};

use super::{GlobalBTreeMap, OtherId};

static PARTY: OnceLock<PartyBTreeMap> = OnceLock::new();

/// MutableBTreeMap storing all players in hero's party.
#[derive(Debug)]
pub struct PartyBTreeMap(MutableBTreeMap<OtherId, PartyMember>);

impl PartyBTreeMap {
    pub(super) fn init() -> JsResult<()> {
        PARTY
            .set(Self(MutableBTreeMap::new()))
            .map_err(|_| common::err_code!())
    }

    pub fn get() -> &'static Self {
        PARTY.wait()
    }

    pub(crate) fn merge(new_party: PartyData) {
        let mut party_lock = Self::get().lock_mut();
        let Some(new_members) = new_party.members else {
            return party_lock.clear();
        };
        let keys_to_remove: Vec<_> = party_lock
            .keys()
            .filter(|char_id| !new_members.contains_key(char_id))
            .copied()
            .collect();

        keys_to_remove.into_iter().for_each(|char_id| {
            party_lock.remove(&char_id);
        });

        new_members
            .into_iter()
            .for_each(
                |(char_id, new_party_member_data)| match party_lock.get(&char_id) {
                    Some(party_member_data) => party_member_data.update(new_party_member_data),
                    None => {
                        party_lock.insert_cloned(char_id, PartyMember::new(new_party_member_data));
                    }
                },
            );
    }
}

impl GlobalBTreeMap<OtherId, PartyMember> for PartyBTreeMap {
    fn get(&self) -> &MutableBTreeMap<OtherId, PartyMember> {
        &self.0
    }
}
