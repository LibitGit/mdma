use std::{iter::Filter, sync::OnceLock};

use futures_signals::{
    signal::{Mutable, Signal, SignalExt},
    signal_map::MutableBTreeMap,
    signal_vec::SignalVecExt,
};

use crate::{
    bindings::engine::{
        communication::{BusinessCards, Chat, Id, PeerData, Response},
        peer::Peer,
    },
    utils::JsResult,
};

use super::{GlobalBTreeMap, hero::Hero};

static PEERS: OnceLock<PeerBTreeMap> = OnceLock::new();

pub type PeerId = Id;

/// MutableBTreeMap storing clan members and friends data.
#[derive(Debug)]
pub struct PeerBTreeMap(MutableBTreeMap<PeerId, Peer>);

impl PeerBTreeMap {
    pub(super) fn init() -> JsResult<()> {
        PEERS
            .set(Self(MutableBTreeMap::new()))
            .map_err(|_| common::err_code!())
    }

    pub fn get() -> &'static Self {
        PEERS.wait()
    }

    //pub fn online_friends_signal(&self) -> impl Signal<Item = Vec<(PeerId, OtherData)>> {
    //    self.0
    //        .entries_cloned()
    //        .to_signal_cloned()
    //        .map(|mut entries| {
    //            entries.retain(|(_, peer_data)| {
    //                peer_data.online && peer_data.relation.is_some_and(|r| r == Relation::Friend)
    //            });
    //            entries
    //        })
    //}
    //
    //pub fn online_clan_members_signal(&self) -> impl Signal<Item = Vec<(PeerId, OtherData)>> {
    //    self.0
    //        .entries_cloned()
    //        .to_signal_cloned()
    //        .map(|mut entries| {
    //            entries.retain(|(_, peer_data)| {
    //                peer_data.online && peer_data.relation.is_some_and(|r| r == Relation::Clan)
    //            });
    //            entries
    //        })
    //}

    //pub fn online_entries_signal_vec(&self) -> impl SignalVec<Item = (PeerId, OtherData)> {
    //    self.0
    //        .entries_cloned()
    //        .filter(|(_, peer_data)| peer_data.online)
    //}

    pub fn online_len_signal(&self) -> impl Signal<Item = usize> {
        self.0
            .entries_cloned()
            .filter_signal_cloned(|(_, peer_data)| peer_data.online.signal())
            .len()
            .dedupe()
    }

    // FIXME: Probably very poor performance.
    pub fn online_from_keys_signal(&'static self) -> impl Signal<Item = Vec<(PeerId, Peer)>> {
        self.0
            .entries_cloned()
            .filter_signal_cloned(|(_, peer_data)| peer_data.online.signal())
            .to_signal_cloned()
        //.to_signal_cloned()
        //.dedupe_cloned()
        //.map(|entries| {
        //    //common::debug_log!("ENTRIES UPDATED");
        //    let peers_lock = self.lock_ref();
        //    entries
        //        .into_iter()
        //        .filter_map(|peer_id| {
        //            peers_lock
        //                .get(&peer_id)
        //                .map(|peer_data| (peer_id, peer_data.clone()))
        //        })
        //        .collect()
        //})
    }

    pub fn online_entries_signal(&self) -> impl Signal<Item = Vec<(PeerId, Peer)>> {
        self.0
            .entries_cloned()
            .filter_signal_cloned(|(_, peer_data)| peer_data.online.signal())
            .to_signal_cloned()
    }

    //pub(crate) fn for_each_online<F>(&self, f: F)
    //where
    //    F: FnMut((&PeerId, &OtherData)),
    //{
    //    self.lock_ref()
    //        .iter()
    //        .filter(|(_, peer_data)| peer_data.online)
    //        .for_each(f)
    //}

    // pub(crate) fn extract_from_message_event(socket_response: &mut Response) {
    pub(crate) fn extract_from_socket_response(socket_response: &mut Response) {
        if let Some(mut members) = socket_response.members.take() {
            //Remove hero from members.
            members.retain(|member| Hero::get().char_id != member.id);

            Self::update(members);
        }
        if let Some(friends) = socket_response.friends.take() {
            Self::update(friends);
            let _friends_max = socket_response.friends_max.take();
            let _enemies = socket_response.enemies.take();
            let _enemies_max = socket_response.enemies_max.take();
        }
    }

    pub(crate) fn update_from_business_cards(new_business_cards: BusinessCards) {
        let peers_lock = Self::get().lock_ref();

        new_business_cards
            .into_iter()
            .filter_map(|business_card| Some((business_card.account?, business_card.char_id?)))
            .for_each(|(account, char_id)| {
                if let Some(peer_data) = peers_lock.get(&char_id) {
                    peer_data.account.set_neq(Some(account))
                }
            });
    }

    //TODO: Await peers fetch before trying to remove any peer.
    pub(crate) fn update_from_chat_message(chat: Chat) {
        let Some(system_messages) = chat.get_system_messages() else {
            return;
        };
        let mut peers_lock = Self::get().lock_mut();

        // FIXME: It should never be empty, but this can happen if a chat message is received
        // before the peers get fetched.
        if peers_lock.is_empty() {
            return;
        }

        system_messages.iter().for_each(|msg| {
            // TODO: Do something when failed.
            msg.try_update_one_peer(&mut peers_lock);
        })
    }

    // TODO: Add clan when peer is a clan member.
    pub(crate) fn update<B: PeerData>(peers: Vec<B>) {
        let mut peers_lock = Self::get().lock_mut();

        let keys_to_remove: Vec<_> = peers_lock
            .keys()
            .filter(|char_id| !peers.iter().any(|peer| peer.id() == **char_id))
            .copied()
            .collect();

        keys_to_remove.into_iter().for_each(|char_id| {
            peers_lock.remove(&char_id);
        });

        peers.into_iter().for_each(|mut peer| {
            peers_lock
                .entry(peer.id())
                .and_modify_cloned(|old_peer_data| {
                    old_peer_data.nick.set_neq(peer.nick());
                    old_peer_data.prof.set_neq(peer.prof());
                    old_peer_data.relation.set_neq(peer.relation());
                    old_peer_data.online.set_neq(peer.is_online());
                    old_peer_data.lvl.set_neq(peer.lvl());
                    old_peer_data.operational_lvl.set_neq(peer.oplvl());
                    old_peer_data.x.set_neq(Some(peer.x()));
                    old_peer_data.y.set_neq(Some(peer.y()));
                    old_peer_data.map_name.set_neq(Some(peer.map_name()));
                })
                .or_insert_cloned_with(|| Peer {
                    account: Mutable::default(),
                    char_id: peer.id(),
                    clan: Mutable::new(None),
                    nick: Mutable::new(peer.nick()),
                    prof: Mutable::new(peer.prof()),
                    relation: Mutable::new(peer.relation()),
                    online: Mutable::new(peer.is_online()),
                    lvl: Mutable::new(peer.lvl()),
                    operational_lvl: Mutable::new(peer.oplvl()),
                    x: Mutable::new(Some(peer.x())),
                    y: Mutable::new(Some(peer.y())),
                    map_name: Mutable::new(Some(peer.map_name())),
                });
        });
    }
}

impl GlobalBTreeMap<PeerId, Peer> for PeerBTreeMap {
    fn get(&self) -> &MutableBTreeMap<PeerId, Peer> {
        &self.0
    }
}

pub trait FilterOnline {
    //fn get_online<'a>(self) -> Filter<Self, fn(&(&'a PeerId, &'a OtherData)) -> bool>
    //where
    //    Self: Iterator<Item = (&'a PeerId, &'a OtherData)> + Sized,
    //{
    //    self.filter(|(_, other_data): &(&'a PeerId, &'a OtherData)| other_data.online)
    //}

    fn filter_online<'a, F>(
        self,
        mut f: F,
    ) -> Filter<Self, impl FnMut(&(&'a PeerId, &'a Peer)) -> bool>
    where
        Self: Iterator<Item = (&'a PeerId, &'a Peer)> + Sized,
        F: FnMut((&'a PeerId, &'a Peer)) -> bool,
    {
        self.filter(move |(peer_id, other_data)| match other_data.online.get() {
            true => f((*peer_id, *other_data)),
            false => false,
        })
    }

    //fn map_online<'a, F, R>(
    //    self,
    //    mut f: F,
    //) -> FilterMap<Self, impl FnMut((&'a PeerId, &'a OtherData)) -> Option<R>>
    //where
    //    Self: Iterator<Item = (&'a PeerId, &'a OtherData)> + Sized,
    //    F: FnMut((&'a PeerId, &'a OtherData)) -> R,
    //{
    //    self.filter_map(move |(peer_id, other_data)| match other_data.online {
    //        true => Some(f((peer_id, other_data))),
    //        false => None,
    //    })
    //}

    fn filter_map_online<'a, F, R>(self, mut f: F) -> impl Iterator<Item = R>
    where
        F: FnMut((&'a PeerId, &'a Peer)) -> Option<R>,
        Self: Iterator<Item = (&'a PeerId, &'a Peer)> + Sized,
    {
        self.filter_map(move |(peer_id, other_data)| match other_data.online.get() {
            true => f((peer_id, other_data)),
            false => None,
        })
    }
}

impl<I: Iterator> FilterOnline for I {}
