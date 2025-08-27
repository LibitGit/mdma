use futures_signals::signal::Mutable;

use crate::globals::OtherId;

use super::{Clan, Profession, Relation};

// TODO: Documentation, mention when the data get's updated.
#[derive(Debug, Clone)]
pub struct Peer {
    /// Account id of the peer.
    ///
    /// Not present by default, but the implementation updates it whenever it is possible to do so.
    pub account: Mutable<Option<u32>>,
    /// Character id of the peer.
    pub char_id: OtherId,
    /// Peer's current clan if any.
    /// `Some` if the peer is part of hero's clan, `None` otherwise.
    // TODO: Set to some if peer is fetched via members action.
    pub clan: Mutable<Option<Clan>>,
    /// Current level of a peer.
    pub lvl: Mutable<u16>,
    /// Current operational level of a peer.
    /// If a peer's level is <= 300 the operational level will have the exact same value.
    pub operational_lvl: Mutable<u16>,
    /// Peer's in-game nick.
    pub nick: Mutable<String>,
    /// Peer's in-game profession.
    pub prof: Mutable<Profession>,
    /// Hero's relation relative to the peer.
    pub relation: Mutable<Relation>,
    /// Peer's current x-axis coordinate relative to the left border of a map.
    /// Updates if the peer is in the same location hero is currently in.
    /// `None` until updated if the peer leaves the location hero is currently in.
    pub x: Mutable<Option<u8>>,
    /// Peer's current y-axis coordinate relative to the top border of a map.
    /// Updates if the peer is in the same location hero is currently in.
    /// `None` until updated if the peer leaves the location hero is currently in.
    pub y: Mutable<Option<u8>>,
    /// Name of a location, the peer was in last time his data got fetched.
    /// Updates if the peer enters the same location hero is currently in.
    /// `None` until updated if the peer leaves the location hero is currently in.
    pub map_name: Mutable<Option<String>>,
    /// Specifies whether the peer is currently logged in.
    pub online: Mutable<bool>,
    // TODO: Last online ?
    // pub vip: Option<String>,
    // pub rights: Option<u8>,
    // pub is_blessed: Option<u8>,
    // pub dir: Option<u8>,
    // pub icon: Option<String>,
    // pub attr: Option<u8>,
}
