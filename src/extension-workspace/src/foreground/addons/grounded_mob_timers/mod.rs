// TODO: SI (, collision on hover ?)
// TODO: Usuwaj po x s od maxa.
mod html;

use std::{cell::RefCell, collections::BTreeMap};

use futures_signals::{
    signal::Mutable,
    signal_map::{MapDiff, SignalMapExt},
};
use js_sys::Function;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use crate::{globals::npcs::Npc, pathfinder::Pos, prelude::*};

const ADDON_NAME: AddonName = AddonName::GroundedMobTimers;
const TILE_SIZE: f64 = 32.0;

#[derive(Settings)]
struct Settings {
    /// Whether to display the timer on a mobs default position or the position where it was last killed.
    default_pos: Mutable<bool>,
    /// Whether to display a gray tile with a timer on the ground or an image of the mob that will spawn there.
    display_tiling: Mutable<bool>,
    /// Whether to remove the timer after it's timeout is exceeded.
    auto_remove: Mutable<bool>,
    /// Time in seconds to remove the timer after it's timeout is exceeded.
    auto_remove_sec: Mutable<u8>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_pos: Mutable::new(true),
            display_tiling: Mutable::new(true),
            auto_remove: Mutable::new(true),
            auto_remove_sec: Mutable::new(5),
        }
    }
}

impl Settings {
    fn init(&'static self) -> JsResult<()> {
        self.add_to_renderer()?;

        let future = Npcs::get()
            .signal_map_cloned()
            .for_each(|map_diff| self.on_npc_diff(map_diff));
        wasm_bindgen_futures::spawn_local(future);

        Ok(())
    }

    async fn on_npc_diff(&'static self, map_diff: MapDiff<i32, Npc>) {
        let Some((npc_id, npc)) = NPC_LEVELS_CACHE.with_borrow_mut(|npcs| {
            match map_diff {
                MapDiff::Remove { key } => return npcs.get(&key).map(|npc| (key, *npc)),
                MapDiff::Replace { entries } => *npcs = BTreeMap::from_iter(entries.into_iter()),
                MapDiff::Insert { key, value } | MapDiff::Update { key, value } => {
                    TIMERS.with_borrow_mut(|timers_map| timers_map.remove(&key));
                    npcs.insert(key, value);
                }
                MapDiff::Clear {} => npcs.clear(),
            }

            None
        }) else {
            return;
        };

        // TODO: Helper method for checking npc type using wt.
        //       In order to do that you need to rewrite the Values of NpcTemplates.
        let colossus = NpcTemplates::get()
            .lock_ref()
            .get(&npc.template_id)
            .unwrap_js()
            .warrior_type
            .is_some_and(|wt| wt > 89 && wt < 100);
        if colossus {
            return;
        }
        if Hero::get().stasis.get() {
            return;
        }

        match Town::get().lock_ref().visibility {
            None | Some(0) => {}
            Some(max) => {
                let Some(hero_x) = Hero::get().x.get() else {
                    return;
                };
                let Some(hero_y) = Hero::get().y.get() else {
                    return;
                };

                // Don't add timers on npcs killed outside of visible range.
                if (hero_x as i32 - npc.x as i32).abs() > max
                    || (hero_y as i32 - npc.y as i32).abs() > max
                {
                    return;
                }
            }
        }

        match TimerData::new(npc) {
            Ok(timer) => TIMERS.with_borrow_mut(|timers_map| {
                timers_map.insert(npc_id, timer);
            }),
            Err(err_code) => console_error!(err_code),
        }
    }
}

#[cfg(not(feature = "ni"))]
impl Settings {
    fn add_to_renderer(&'static self) -> JsResult<()> {
        Ok(())
    }
}

#[cfg(feature = "ni")]
impl Settings {
    fn add_to_renderer(&'static self) -> JsResult<()> {
        let api_data = get_engine().api_data().ok_or_else(|| err_code!())?;

        let renderer = get_engine().renderer().ok_or_else(|| err_code!())?;
        let after_call_draw_add_to_renderer =
            closure!(move || -> JsResult<()> { self.on_call_draw_add_to_renderer(&renderer) });
        get_game_api()
            .add_callback_to_event(
                &api_data.call_draw_add_to_renderer(),
                &after_call_draw_add_to_renderer,
            )
            .map_err(map_err!())?;

        Ok(())
    }

    pub fn on_call_draw_add_to_renderer(
        &self,
        renderer: &crate::bindings::engine::renderer::Renderer,
    ) -> JsResult<()> {
        if !Addons::is_active(ADDON_NAME) {
            return Ok(());
        }

        TIMERS.with_borrow_mut(|timers_map| {
            if self.auto_remove.get() {
                let removal_threshold = -1 * self.auto_remove_sec.get() as i32;
                timers_map.retain(|_, timer_data| timer_data.sec_left() >= removal_threshold);
            }

            timers_map
                .values()
                .for_each(|timer_data| match timer_data.get_drawable_obj() {
                    Ok(Some(drawable_obj)) => renderer.add_1(&drawable_obj),
                    Ok(None) => {}
                    Err(err_code) => console_error!(err_code),
                });
        });

        Ok(())
    }
}

#[derive(Clone, Copy)]
struct TimerData {
    /// Where the timer should be displayed.
    pos: Pos,
    /// When the timer should be removed from the list (max respawn time + 5s)
    timeout: u32,
    /// Specifies from when the mob can spawn.
    start_highlight: u32,
    /// Npc id used for draw ordering.
    npc_id: Id,
    /// Id of the map which the timer is going to be displayed on.
    map_id: Id,
}

thread_local! {
    static TIMERS: RefCell<BTreeMap<Id, TimerData>> = const { RefCell::new(BTreeMap::new()) };
    static NPC_LEVELS_CACHE: RefCell<BTreeMap<Id, Npc>> = const { RefCell::new(BTreeMap::new()) };
}

impl TimerData {
    fn new(npc: Npc) -> JsResult<Self> {
        let lvl = Self::calculate_lvl(&npc)?;
        let mut resp_base_seconds =
            40.0 + 10.85 * 200f64.min(lvl) - 0.02721 * 200f64.min(lvl).powi(2);
        // Adjust for world npc_resp speed.
        resp_base_seconds /= WorldConfig::npc_resp() as f64;
        // Adjust for resp boost from players online.
        resp_base_seconds /= (PlayersOnline::currently_online() / 500).max(1).min(2) as f64;

        let resp_multiplier = 0.1;
        let top_range_resp = resp_base_seconds * resp_multiplier + resp_base_seconds;
        let bottom_range_resp = resp_base_seconds - resp_base_seconds * resp_multiplier;
        let now = js_sys::Date::now() / 1000.0;
        let timeout = (now + top_range_resp).round() as u32;
        let start_highlight = (now + bottom_range_resp).round() as u32;

        Ok(Self {
            pos: Pos::new(npc.x as usize, npc.y as usize),
            timeout,
            start_highlight,
            npc_id: npc.id,
            map_id: Town::get().lock_ref().id.ok_or_else(|| err_code!())?,
        })
    }

    fn calculate_lvl(npc: &Npc) -> JsResult<f64> {
        let npc_templates_lock = NpcTemplates::get().lock_ref();
        let npc_template = npc_templates_lock
            .get(&npc.template_id)
            .ok_or_else(|| err_code!())?;

        npc_template
            .elastic_level_factor
            .map(|elf| {
                let others_lock = Others::get().lock_ref();

                // Highest lvl from party on the same map as hero (including hero lvl).
                let highest_lvl_from_party = Party::get()
                    .lock_ref()
                    .keys()
                    .filter_map(|member_id| Some(others_lock.get(member_id)?.lvl.get()))
                    .max_by(Ord::cmp)
                    .unwrap_or_default()
                    .max(Hero::get().lvl.get());

                highest_lvl_from_party as f64 + elf as f64
            })
            .or_else(|| {
                let Some(group) = npc.group else {
                    return npc_template.level.map(|lvl| lvl as f64);
                };

                // Highest lvl from npc group.
                // We **need** to use the cache since the npcs were already removed from Npcs.
                NPC_LEVELS_CACHE.with_borrow(|npcs| {
                    npcs.values()
                        .filter_map(|npc| {
                            if npc.group? != group {
                                return None;
                            }

                            npc_templates_lock
                                .get(&npc.template_id)
                                .and_then(|npc_tpl| npc_tpl.level)
                        })
                        .max_by(Ord::cmp)
                        .map(|group_lvl| group_lvl as f64)
                })
            })
            .ok_or_else(|| err_code!())
    }

    fn should_highlight(&self) -> bool {
        (js_sys::Date::now() / 1000.0).round() >= self.start_highlight as f64
    }

    fn to_parsed_time(&self) -> String {
        let now = (js_sys::Date::now() / 1000.0).round() as i32;
        let mut diff = self.timeout as i32 - now;
        let sec = diff % 60;
        diff -= sec;
        let tmin = diff % 3600;
        let min = tmin / 60;
        diff -= tmin;
        let hou = diff / 3600;

        // Handle overflow case
        if hou > 99 {
            return "99:99:99".to_string();
        }

        // Ensure non-negative values
        let sec = if sec < 0 { 0 } else { sec };
        let min = if min < 0 { 0 } else { min };
        let hou = if hou < 0 { 0 } else { hou };

        // Format based on what values are non-zero
        if hou > 0 {
            // Show hours:minutes:seconds
            let shou = if hou < 10 {
                format!("0{}", hou)
            } else {
                hou.to_string()
            };
            let smin = if min < 10 {
                format!("0{}", min)
            } else {
                min.to_string()
            };
            let ssec = if sec < 10 {
                format!("0{}", sec)
            } else {
                sec.to_string()
            };
            format!("{}:{}:{}", shou, smin, ssec)
        } else if min > 0 {
            // Show minutes:seconds (no leading zero for minutes when no hours)
            let ssec = if sec < 10 {
                format!("0{}", sec)
            } else {
                sec.to_string()
            };
            format!("{}:{}", min, ssec)
        } else {
            // Show just seconds
            sec.to_string()
        }
    }

    /// Check if expired .
    fn sec_left(&self) -> i32 {
        let now = (js_sys::Date::now() / 1000.0).round() as i32;
        self.timeout as i32 - now
    }
}

#[cfg(feature = "ni")]
impl TimerData {
    fn draw(&self, ctx: &CanvasRenderingContext2d) -> JsResult<()> {
        ctx.save();

        let rx = self.pos.x as f64;
        let ry = self.pos.y as f64;
        let offset = get_engine()
            .map()
            .ok_or_else(|| err_code!())?
            .get_offset()
            .ok_or_else(|| err_code!())?;
        let left = rx * 32.0 - offset[0];
        let top = ry * 32.0 - offset[1];

        Self::draw_tile(ctx, left, top);
        self.prepare_font(ctx);
        self.draw_text(ctx, left, top)?;

        ctx.restore();

        Ok(())
    }

    fn draw_tile(ctx: &CanvasRenderingContext2d, left: f64, top: f64) {
        ctx.set_line_width(3.0);
        ctx.set_stroke_style_str("rgba(206, 206, 206, 0.4");
        ctx.stroke_rect(left, top, TILE_SIZE, TILE_SIZE);
        ctx.set_fill_style_str("rgba(206, 206, 206, 0.4");
        ctx.fill_rect(left, top, TILE_SIZE, TILE_SIZE);
    }

    fn prepare_font(&self, ctx: &CanvasRenderingContext2d) {
        ctx.set_font("11px Arimo");
        ctx.set_line_cap("round");
        ctx.set_line_join("round");
        ctx.set_text_align("center");

        let should_highlight = self.should_highlight();

        ctx.set_fill_style_str(if self.sec_left() <= 0 {
            "red"
        } else if should_highlight {
            "darkorange"
        } else {
            "rgba(206, 206, 206, 0.8)"
        });

        ctx.set_stroke_style_str(if should_highlight {
            "black"
        } else {
            "rgba(0, 0, 0, 0.8)"
        });
    }

    fn draw_text(&self, ctx: &CanvasRenderingContext2d, left: f64, top: f64) -> JsResult<()> {
        let parsed_time = self.to_parsed_time();
        let x = left + TILE_SIZE / 2.0;
        let y = top + TILE_SIZE / 2.0 + 3.0;

        ctx.stroke_text_with_max_width(&parsed_time, x, y, TILE_SIZE)
            .map_err(map_err!())?;
        ctx.fill_text_with_max_width(&parsed_time, x, y, TILE_SIZE)
            .map_err(map_err!())
    }

    fn get_drawable_obj(self) -> JsResult<Option<JsValue>> {
        #[derive(Serialize)]
        struct ColorMarkExport {
            rx: f64,
            ry: f64,
            #[serde(with = "serde_wasm_bindgen::preserve")]
            draw: Function,
            #[serde(rename = "getOrder", with = "serde_wasm_bindgen::preserve")]
            get_order: Function,
            d: ColorMarkData,
        }

        #[derive(Serialize)]
        struct ColorMarkData {
            id: f64,
        }

        if Town::get().lock_ref().id != Some(self.map_id) {
            return Ok(None);
        }

        let draw = closure!(@once move |ctx: &CanvasRenderingContext2d| {
            if let Err(err_code) = self.draw(ctx) {
                console_error!(err_code);
            }
        });
        let rx = self.pos.x as f64;
        let ry = self.pos.y as f64;
        let export = ColorMarkExport {
            rx,
            ry,
            draw,
            get_order: self.get_order_factory(),
            d: ColorMarkData {
                id: self.npc_id as f64,
            },
        };

        let value = serde_wasm_bindgen::to_value(&export).map_err(map_err!(from))?;

        Ok(Some(value))
    }

    fn get_order_factory(self) -> Function {
        let closure = move || {
            0.001 // Just above the map (0)
        };

        Closure::<dyn Fn() -> f64>::wrap(Box::new(closure))
            .into_js_value()
            .unchecked_into()
    }
}

pub(crate) fn init() -> JsResult<()> {
    let settings = Settings::new(ADDON_NAME);
    settings.init()?;

    html::init(settings)
}
