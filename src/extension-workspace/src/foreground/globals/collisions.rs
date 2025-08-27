use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    ops::{Deref, DerefMut},
};

use common::err_code;
use itertools::Itertools;

use crate::{
    bindings::engine::communication::{NpcData, NpcDelData},
    pathfinder::Pos,
    utils::{JsResult, UnwrapJsExt},
};

use super::{GlobalBTreeMap, npcs::Npcs, town::Town};

thread_local! {
    static NPC_COLLISIONS: RefCell<NpcCollisions> = const { RefCell::new(NpcCollisions::new()) };
    static MAP_COLLISIONS: RefCell<MapCollisions> = const { RefCell::new(MapCollisions::new()) };
    static GATEWAYS: RefCell<Gateways> = const { RefCell::new(Gateways::new()) };
}

#[derive(Debug)]
pub struct NpcCollisions(BTreeSet<(u8, u8)>);

impl Deref for NpcCollisions {
    type Target = BTreeSet<(u8, u8)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NpcCollisions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl NpcCollisions {
    const fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub(crate) fn on_npcs_del(list: &Vec<NpcDelData>) {
        let npcs_lock = Npcs::get().lock_ref();

        NPC_COLLISIONS.with_borrow_mut(|npc_collisions| {
            list.iter()
                .filter_map(|npc| {
                    let id = npc.id.as_ref()?;
                    npcs_lock.get(id).map(|npc| (npc.x, npc.y))
                })
                .for_each(|collision_pos| {
                    npc_collisions.remove(&collision_pos);
                });
        })
    }

    pub(crate) fn on_npcs(new_npcs: &Vec<NpcData>) {
        NPC_COLLISIONS.with_borrow_mut(|npc_collisions| {
            new_npcs
                .iter()
                .filter_map(|npc| {
                    npc.has_collision()
                        .is_none_or(|has_cl| has_cl)
                        .then_some(npc)
                })
                .for_each(|npc| {
                    if let Some(x) = npc.x
                        && let Some(y) = npc.y
                    {
                        npc_collisions.insert((x, y));
                        return;
                    }

                    console_error!();
                });
        })
    }

    pub(crate) fn reload() {
        NPC_COLLISIONS.with_borrow_mut(|npc_collisions| npc_collisions.clear())
    }

    pub fn collision_at(dest: Pos) -> bool {
        NPC_COLLISIONS
            .with_borrow(|npc_collisions| npc_collisions.contains(&(dest.x as u8, dest.y as u8)))
    }
}

#[derive(Debug)]
pub struct MapCollisions(Vec<u8>);

impl Deref for MapCollisions {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MapCollisions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl MapCollisions {
    const fn new() -> Self {
        Self(Vec::new())
    }

    // TODO: Make this production ready.
    pub(crate) fn on_collisions(new_collisions: String) {
        MAP_COLLISIONS.with_borrow_mut(|map_collisions| {
            map_collisions.clear();

            if new_collisions.is_empty() {
                let town_lock = Town::get().lock_ref();
                if let Some(width) = town_lock.x
                    && let Some(height) = town_lock.y
                {
                    let total_size = width as usize * height as usize;
                    map_collisions.resize(total_size, 0);
                } else {
                    // Handle missing dimensions, possibly log or return
                    common::debug_log!("Cannot resize collisions: Town dimensions not available.");
                }
                return;
            }
            let mut idx = 0;

            new_collisions.encode_utf16().for_each(|ch| {
                if ch > 95 && ch < 123 {
                    let repeat = (ch - 95) * 6;
                    let repeat = repeat as usize;

                    match map_collisions.len() < idx + repeat {
                        true => map_collisions.resize(idx + repeat, 0),
                        false => map_collisions[idx..idx + repeat].fill(0),
                    }

                    idx += repeat;
                    return;
                }

                let a = ch - 32;
                for j in 0..6 {
                    let val = if a & (1 << j) != 0 { 1 } else { 0 };

                    match map_collisions.len() <= idx {
                        true => map_collisions.push(val),
                        false => map_collisions[idx] = val,
                    }

                    idx += 1;
                }
            });
        })
    }

    pub(crate) fn reload() {
        MAP_COLLISIONS.with_borrow_mut(|map_collisions| map_collisions.clear());
    }

    pub fn collision_at(dest: Pos) -> JsResult<bool> {
        Town::get()
            .lock_ref()
            .x
            .map(|width| {
                MAP_COLLISIONS.with_borrow(|map_collisions| {
                    map_collisions[dest.x + dest.y * width as usize] == 1
                })
            })
            .ok_or_else(|| err_code!())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Gateway {
    // dest_map_id: i32,
    // x: u8,
    // y: u8,
    // key: i32, // what is this? - 2 for siedziba kb
    // min_lvl: u16,
}

#[derive(Debug)]
pub struct Gateways(BTreeMap<Pos, Gateway>);

impl Deref for Gateways {
    type Target = BTreeMap<Pos, Gateway>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Gateways {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Gateways {
    const fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn gateway_at(pos: &Pos) -> bool {
        GATEWAYS.with_borrow(|gateways| gateways.contains_key(pos))
    }

    pub(crate) fn on_gateways(mut new_gateways: Vec<i32>) {
        GATEWAYS.with_borrow_mut(|gateways| {
            gateways.clear();

            new_gateways
                .drain(..)
                .chunks(5)
                .into_iter()
                .for_each(|mut chunk| {
                    let Some(_id) = chunk.next() else {
                        console_error!();
                        return;
                    };
                    let Some(x) = chunk.next() else {
                        console_error!();
                        return;
                    };
                    let Some(y) = chunk.next() else {
                        console_error!();
                        return;
                    };
                    let Some(_key) = chunk.next() else {
                        console_error!();
                        return;
                    };
                    let Some(_min_lvl) = chunk.next() else {
                        console_error!();
                        return;
                    };

                    gateways.insert(
                        Pos::new(x as usize, y as usize),
                        Gateway {
                            // dest_map_id: id,
                            // x: x as u8,
                            // y: y as u8,
                            // key,
                            // min_lvl: min_lvl as u16,
                        },
                    );
                });
        })
    }

    pub(crate) fn reload() {
        GATEWAYS.with_borrow_mut(|gateways| gateways.clear());
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Collisions;

impl Collisions {
    pub fn on_path<'a, A>(path: A) -> Option<bool>
    where
        A: IntoIterator<Item = &'a Pos>,
    {
        Some(
            path.into_iter()
                .any(|&pos| Self::collision_at(pos).unwrap_js()),
        )
    }

    pub fn collision_at(dest: Pos) -> JsResult<bool> {
        Ok(MapCollisions::collision_at(dest)? || NpcCollisions::collision_at(dest))
    }
}
