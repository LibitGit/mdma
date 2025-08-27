use std::sync::OnceLock;

use futures_signals::signal::{Mutable, MutableLockMut, MutableLockRef, MutableSignalRef};

use crate::{bindings::engine::communication::TownData, utils::JsResult};

static TOWN: OnceLock<Town> = OnceLock::new();

#[derive(Debug)]
pub struct Town(Mutable<TownData>);

impl Town {
    pub(super) fn init() -> JsResult<()> {
        TOWN.set(Self(Mutable::default()))
            .map_err(|_| common::err_code!())
    }

    pub fn get() -> &'static Self {
        TOWN.wait()
    }

    #[inline]
    pub(crate) fn lock_ref(&self) -> MutableLockRef<'_, TownData> {
        self.0.lock_ref()
    }

    #[inline]
    fn lock_mut(&self) -> MutableLockMut<'_, TownData> {
        self.0.lock_mut()
    }

    #[inline]
    pub(crate) fn signal_ref<B, F>(&self, f: F) -> MutableSignalRef<TownData, F>
    where
        F: FnMut(&TownData) -> B,
    {
        self.0.signal_ref(f)
    }

    pub(crate) fn merge(new_town: TownData) {
        let mut town_lock = Self::get().lock_mut();

        if new_town.name.is_some() {
            town_lock.name = new_town.name;
        }
        if new_town.id.is_some() {
            town_lock.id = new_town.id;
        }
        if new_town.x.is_some() {
            town_lock.x = new_town.x
        }
        if new_town.y.is_some() {
            town_lock.y = new_town.y
        }
        if new_town.visibility.is_some() {
            town_lock.visibility = new_town.visibility
        }
    }

    pub(crate) fn reload() {
        Self::get().0.replace(TownData::default());
    }
}
