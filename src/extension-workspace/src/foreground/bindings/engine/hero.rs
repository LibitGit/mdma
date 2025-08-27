use common::err_code;
#[cfg(all(feature = "ni", not(feature = "antyduch")))]
use common::map_err;
use js_sys::{Function, Object};
use serde::Serialize;
use wasm_bindgen::prelude::*;
#[cfg(feature = "ni")]
use web_sys::CanvasRenderingContext2d;

#[cfg(not(feature = "antyduch"))]
use crate::utils::UnwrapJsExt;
use crate::{globals::OtherId, utils::JsResult};

use super::others::Other;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type Hero;

    #[wasm_bindgen(catch, method, js_name = "autoGoTo")]
    fn try_auto_go_to(this: &Hero, dst: &Object, click_in_map_canvas: bool) -> JsResult<JsValue>;

    #[wasm_bindgen(method, getter = "clearAfterFollowAction")]
    fn get_clear_after_follow_action(this: &Hero) -> Option<Function>;

    #[wasm_bindgen(method, setter = "clearAfterFollowAction")]
    fn set_clear_after_follow_action(this: &Hero, value: &Function);

    #[wasm_bindgen(catch, method, js_name = "clearAfterFollowAction")]
    fn clear_after_follow_action(this: &Hero) -> JsResult<JsValue>;

    #[wasm_bindgen(catch, method, js_name = "addAfterFollowAction")]
    pub fn try_add_after_follow_action(
        this: &Hero,
        other: &Other,
        action: &Function,
    ) -> JsResult<JsValue>;

    #[wasm_bindgen(method, js_name = "setStartClickOnMapMove")]
    fn set_start_click_on_map_move(this: &Hero, start_click_on_map_move: bool);

    #[wasm_bindgen(method, getter = "run")]
    pub fn get_run(this: &Hero) -> Option<Function>;

    #[wasm_bindgen(method, setter = "run")]
    pub fn set_run(this: &Hero, value: &Function);

    #[wasm_bindgen(method, getter = "rx")]
    pub(crate) fn rx(this: &Hero) -> Option<f64>;

    #[wasm_bindgen(method, getter = "ry")]
    pub(crate) fn ry(this: &Hero) -> Option<f64>;

    #[wasm_bindgen(method, getter = "fh")]
    pub(crate) fn fh(this: &Hero) -> Option<f64>;

    #[wasm_bindgen(method, getter = "fw")]
    pub(crate) fn fw(this: &Hero) -> Option<f64>;

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type HeroData;

    #[wasm_bindgen(catch, method, getter, js_name = d)]
    pub(crate) fn data(this: &Hero) -> JsResult<HeroData>;

    #[wasm_bindgen(method, getter = "opt")]
    pub(crate) fn options_value(this: &HeroData) -> Option<f64>;

    #[wasm_bindgen(method, setter, js_name = opt)]
    pub(crate) fn set_options_value(this: &HeroData, value: i32);

    #[wasm_bindgen(catch, method, getter)]
    pub(crate) fn x(this: &HeroData) -> JsResult<f64>;

    #[wasm_bindgen(catch, method, getter)]
    pub(crate) fn y(this: &HeroData) -> JsResult<f64>;
}

#[cfg(feature = "ni")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, js_name = "autoPath")]
    pub(crate) fn auto_path(this: &Hero) -> Option<auto_path::AutoPath>;

    #[wasm_bindgen(method, getter = "setOutfitData")]
    pub(crate) fn get_set_outfit_data(this: &Hero) -> Option<Function>;

    #[wasm_bindgen(method, setter = "setOutfitData")]
    pub(crate) fn set_set_outfit_data(this: &Hero, value: &Function);

    #[wasm_bindgen(catch, method, js_name = "draw")]
    pub(crate) fn draw(this: &Hero, ctx: &CanvasRenderingContext2d) -> JsResult<JsValue>;

    #[wasm_bindgen(method, setter = "draw")]
    pub(crate) fn set_draw(this: &Hero, value: &Function);

    #[wasm_bindgen(method, getter = "draw")]
    pub(crate) fn get_draw(this: &Hero) -> Option<Function>;

    #[wasm_bindgen(catch, method, js_name = "getOrder")]
    pub(crate) fn get_order(this: &Hero) -> JsResult<JsValue>;

    #[wasm_bindgen(method, setter = "getOrder")]
    pub(crate) fn set_get_order(this: &Hero, value: &Function);

    #[wasm_bindgen(method, getter = "getOrder")]
    pub(crate) fn get_get_order(this: &Hero) -> Option<Function>;
}

#[cfg(not(feature = "ni"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, getter = "ml")]
    pub(crate) fn ml(this: &Hero) -> Option<js_sys::Array>;

    #[wasm_bindgen(method, catch, js_name = "searchPath")]
    fn __searchPath(this: &Hero, dx: u8, dy: u8) -> JsResult<JsValue>;
}

pub mod auto_path {
    use super::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen (extends = ::js_sys::Object)]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub(crate) type AutoPath;

        #[wasm_bindgen(method, getter = "road")]
        pub(crate) fn get_road(this: &AutoPath) -> Option<js_sys::Array>;

        #[wasm_bindgen(method, setter = "road")]
        pub(crate) fn set_road(this: &AutoPath, value: &js_sys::Array);
    }
}

#[derive(Serialize, Clone, Copy)]
pub(crate) struct AutoGoToData {
    x: u8,
    y: u8,
}

impl AutoGoToData {
    pub(crate) fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }
}

#[cfg(feature = "antyduch")]
impl Hero {
    pub(crate) fn auto_go_to(&self, dest: &AutoGoToData) -> JsResult<()> {
        use crate::{
            console_error,
            pathfinder::{Pos, pathfind_to},
        };

        let dest = Pos::new(dest.x as usize, dest.y as usize);
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(err_code) = pathfind_to(dest).await {
                console_error!(err_code)
            }
        });

        Ok(())
    }

    pub(crate) async fn auto_go_to_other(&self, other_id: OtherId) -> JsResult<bool> {
        use crate::{
            globals::{GlobalBTreeMap, others::OtherBTreeMap as Others},
            pathfinder::{Pos, pathfind_to},
        };

        let others_lock = Others::get().lock_ref();
        let other = others_lock.get(&other_id).ok_or_else(|| err_code!())?;
        let dest = Pos::new(other.x.get() as usize, other.y.get() as usize);

        drop(others_lock);
        pathfind_to(dest).await?;

        Ok(true)
    }
}

#[cfg(all(feature = "ni", not(feature = "antyduch")))]
impl Hero {
    pub(crate) fn auto_go_to(&self, dest: &AutoGoToData) -> JsResult<()> {
        let dest = serde_wasm_bindgen::to_value(dest).map_err(map_err!(from))?;
        self.try_auto_go_to(dest.unchecked_ref(), false)
            .map_err(map_err!())?;

        Ok(())
    }

    /// Returs true if got to destination or within 1 tile range and false otherwise.
    pub(crate) async fn auto_go_to_other(&self, other_id: OtherId) -> JsResult<bool> {
        use crate::globals::{GlobalBTreeMap, hero, others::OtherBTreeMap as Others};
        use futures::StreamExt;

        let in_range = hero::Hero::is_non_peer_in_invite_range(
            Others::get()
                .lock_ref()
                .get(&other_id)
                .ok_or_else(|| err_code!())?,
        );

        if in_range {
            return Ok(true);
        }

        let other = crate::bindings::get_engine()
            .others()
            .unwrap_js()
            .get_by_id(other_id)
            .ok_or_else(|| err_code!())?;
        let (mut sender, mut receiver) = futures::channel::mpsc::channel(0);
        let old = self
            .get_clear_after_follow_action()
            .ok_or_else(|| err_code!())?;
        let new = common::closure!(
            {
                let mut sender = sender.clone(),
                let hero = self.clone(),
            },
            move || -> JsResult<JsValue> {
                if let Err(_err) = sender.try_send(false) {
                    common::debug_log!(@f "{_err}");
                }
                hero.set_clear_after_follow_action(&old);
                hero.clear_after_follow_action()
            },
        );
        self.set_clear_after_follow_action(&new);
        self.try_add_after_follow_action(
            &other,
            &common::closure!(@once move || {
                sender.try_send(true).unwrap_js()
            }),
        )
        .map_err(map_err!())?;

        receiver.next().await.ok_or_else(|| err_code!())
    }
}

#[cfg(all(not(feature = "ni"), not(feature = "antyduch")))]
impl Hero {
    pub fn auto_path(&self) -> Option<auto_path::AutoPath> {
        Some(crate::utils::window().unchecked_into())
    }

    // FIXME: If user stops in place and then moves along the path the fn still resolves with true.
    pub async fn auto_go_to_other(&self, other_id: OtherId) -> JsResult<bool> {
        use crate::{
            globals::{
                GlobalBTreeMap,
                emitter::{Emitter, EmitterEvent},
                hero::Hero,
                others::OtherBTreeMap as Others,
            },
            pathfinder::{Pos, find_path_or_closest},
        };

        let in_range = Hero::is_non_peer_in_invite_range(
            Others::get()
                .lock_ref()
                .get(&other_id)
                .ok_or_else(|| err_code!())?,
        );

        if in_range {
            return Ok(true);
        }

        let (other_x, other_y) = {
            let others_lock = Others::get().lock_ref();
            let other = others_lock.get(&other_id).ok_or_else(|| err_code!())?;

            (other.x.get(), other.y.get())
        };
        let start = Pos::new(
            Hero::get().x.get().ok_or_else(|| err_code!())? as usize,
            Hero::get().y.get().ok_or_else(|| err_code!())? as usize,
        );
        let dest = Pos::new(other_x as usize, other_y as usize);

        let Some(mut road) = find_path_or_closest(start, dest) else {
            return Ok(false);
        };

        road.reverse();

        // Run searchPath to not trigger anti-cheat.
        self.__searchPath(other_x, other_y)?;

        let road_array = js_sys::Array::from_iter(
            road.iter()
                .map(|pos| serde_wasm_bindgen::to_value(pos).unwrap_js()),
        );
        // SAFETY: We know auto_path is Some on SI.
        unsafe {
            self.auto_path().unwrap_unchecked().set_road(&road_array);
        }
        let (tx, rx) = futures::channel::oneshot::channel::<bool>();
        let mut tx = Some(tx);

        let callback_id =
            Emitter::register_limited(EmitterEvent::Hero, road.len(), move |socket_response| {
                let ret = Box::pin(async { Ok(()) });
                let Some(hero) = socket_response.h.as_ref() else {
                    if let Some(tx) = tx.take() {
                        tx.send(false).unwrap_js();
                    }
                    return ret;
                };
                let Some((hero_x, hero_y)) = hero.x.zip(hero.y) else {
                    if let Some(tx) = tx.take() {
                        tx.send(false).unwrap_js();
                    }
                    return ret;
                };
                common::debug_log!("hero:", hero_x, hero_y);
                let current_pos = Pos::new(hero_x as usize, hero_y as usize);

                let pos = road.iter().position(|pos| *pos == current_pos);
                match pos {
                    None => {
                        if let Some(tx) = tx.take() {
                            tx.send(false).unwrap_js();
                        }
                    }
                    Some(index) => {
                        match index {
                            // If we're at dest notify the channel.
                            0 => {
                                if let Some(tx) = tx.take() {
                                    tx.send(true).unwrap_js();
                                }
                            }
                            // Shorten the search vec to make sure we notify when hero changes direction.
                            _ => road.truncate(index + 1),
                        }
                    }
                };

                ret
            })?;

        let res = rx.await.unwrap_or_default();

        common::debug_log!("res:", res, "unregistering...");
        Emitter::unregister_handler(&EmitterEvent::Hero, callback_id);

        let in_range = Hero::is_non_peer_in_invite_range(
            Others::get()
                .lock_ref()
                .get(&other_id)
                .ok_or_else(|| err_code!())?,
        );

        if in_range {
            return Ok(true);
        }

        Ok(res)
    }

    pub(crate) fn auto_go_to(&self, dest: &AutoGoToData) -> JsResult<()> {
        use crate::{
            globals::hero::Hero,
            pathfinder::{Pos, find_path_or_closest},
        };

        let start = Pos::new(
            Hero::get().x.get().ok_or_else(|| err_code!())? as usize,
            Hero::get().y.get().ok_or_else(|| err_code!())? as usize,
        );
        let dest_pos = Pos::new(dest.x as usize, dest.y as usize);

        let Some(mut road) = find_path_or_closest(start, dest_pos) else {
            return Ok(());
        };

        road.reverse();

        let road_array = js_sys::Array::from_iter(
            road.iter()
                .map(|pos| serde_wasm_bindgen::to_value(pos).unwrap_js()),
        );

        self.__searchPath(dest.x, dest.y)?;
        // SAFETY: We know auto_path is Some on SI.
        unsafe {
            self.auto_path().unwrap_unchecked().set_road(&road_array);
        }

        Ok(())
    }
}
