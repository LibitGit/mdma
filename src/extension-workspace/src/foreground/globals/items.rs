use std::{collections::HashMap, sync::OnceLock};

use futures_signals::signal_map::MutableBTreeMap;

use crate::{bindings::engine::communication::Item, utils::JsResult};

use super::{GlobalBTreeMap, ItemId};

static ITEMS: OnceLock<ItemBTreeMap> = OnceLock::new();

/// MutableBTreeMap storing all accessible items.
#[derive(Debug)]
pub struct ItemBTreeMap(MutableBTreeMap<ItemId, Item>);

impl GlobalBTreeMap<ItemId, Item> for ItemBTreeMap {
    fn get(&self) -> &MutableBTreeMap<ItemId, Item> {
        &self.0
    }
}

impl ItemBTreeMap {
    pub(super) fn init() -> JsResult<()> {
        ITEMS
            .set(Self(MutableBTreeMap::new()))
            .map_err(|_| common::err_code!())
    }

    pub fn get() -> &'static Self {
        ITEMS.wait()
    }

    pub(crate) fn merge(mut new_items: HashMap<ItemId, Item>) {
        let mut items_lock = Self::get().lock_mut();

        new_items.drain().for_each(|(item_id, mut new_item_data)| {
            if item_id < 0 {
                return;
            }
            if new_item_data.del.is_some_and(|del| del == 1) {
                items_lock.remove(&item_id);
                return;
            }

            items_lock
                .entry(item_id)
                .and_modify_cloned(|old_item_data| old_item_data.merge(&mut new_item_data))
                .or_insert_cloned(new_item_data);
        });
    }
}
