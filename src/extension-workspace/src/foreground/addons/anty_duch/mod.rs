mod html;

// TODO: Walking back and forth for 10min and check for leaks?

use std::ops::Deref;

use futures::StreamExt;
use futures_signals::{
    map_ref,
    signal::{self, Mutable, Signal, SignalExt},
    signal_vec::SignalVecExt,
};
use proc_macros::{ActiveSettings, Setting};

use crate::{
    addons::kastrat::TargetData,
    bindings::engine::types::MapMode,
    globals::port::task::{Targets, Task, Tasks},
    pathfinder::{Pos, pathfind_to},
    prelude::*,
};

use super::kastrat::Target;

// use super::kastrat::Target;

const ADDON_NAME: AddonName = AddonName::AntyDuch;
pub const MIN_DIFF: f64 = 101.0;

#[derive(Clone, Setting)]
struct Delay {
    min: Mutable<usize>,
    max: Mutable<usize>,
}

impl Delay {
    fn new(min: usize, max: usize) -> Self {
        Self {
            min: Mutable::new(min),
            max: Mutable::new(max),
        }
    }
}

impl Default for Delay {
    fn default() -> Self {
        Self {
            min: Mutable::new(0),
            max: Mutable::new(0),
        }
    }
}

#[derive(Clone, Setting)]
struct Position {
    x: Mutable<u16>,
    y: Mutable<u16>,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            x: Mutable::new(0),
            y: Mutable::new(0),
        }
    }
}

#[derive(Clone, Setting)]
struct Level {
    min: Mutable<u16>,
    max: Mutable<u16>,
}

impl Level {
    fn contains_signal<B: Signal<Item = u16>>(
        &self,
        other_level: B,
    ) -> impl Signal<Item = bool> + use<B> {
        map_ref! {
            let min = self.min.signal(),
            let max = self.max.signal(),
            let cmp = other_level => {
                (*min..=*max).contains(cmp)
            }
        }
    }
}

impl Default for Level {
    fn default() -> Self {
        Self {
            min: Mutable::new(1),
            max: Mutable::new(500),
        }
    }
}

#[derive(Clone, Setting, Default)]
struct AntiAfk {
    return_pos: Position,
    return_active: Mutable<bool>,
}

impl AntiAfk {
    fn exit_stasis_on_signal<B: Signal<Item = ()> + 'static>(&'static self, signal: B) {
        let future = signal.for_each(move |_| async move {
            if !Addons::is_active(ADDON_NAME) {
                return;
            }

            delay_range(2_000, 10_000).await;

            if let Err(err_code) = exit_stasis().await {
                console_error!(err_code);
            }
            if !self.return_active.get() {
                return;
            }

            let dest = Pos::new(
                self.return_pos.x.get() as usize,
                self.return_pos.y.get() as usize,
            );
            let Some(current_hero_pos) = Pos::current_hero_pos() else {
                console_error!();
                return;
            };

            if dest == current_hero_pos {
                return;
            }

            delay_range(2000, 5000).await;

            if let Err(err_code) = pathfind_to(dest).await {
                console_error!(err_code);
            }
        });
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[derive(Clone, Setting, Default)]
struct Kastrat {
    attack_toggle: Mutable<bool>,
    return_pos: Position,
    return_active: Mutable<bool>,
    lvl: Level,
    #[setting(skip)]
    target: Target,
}

#[derive(Clone, Setting)]
struct Berserker {
    delay: Delay,
    npc_name: Mutable<String>,
    fast_fight: Mutable<bool>,
}

impl Default for Berserker {
    fn default() -> Self {
        Self {
            delay: Delay::new(100, 500),
            npc_name: Mutable::default(),
            fast_fight: Mutable::default(),
        }
    }
}

#[derive(ActiveSettings)]
struct ActiveSettings {
    anti_afk: AntiAfk,
    kastrat: Kastrat,
    berserker: Berserker,
}

impl Default for ActiveSettings {
    fn default() -> Self {
        Self {
            anti_afk: AntiAfk::default(),
            kastrat: Kastrat::default(),
            berserker: Berserker::default(),
        }
    }
}

struct Settings;

impl Settings {
    fn can_attack_on_map(target: &Target, map_mode: MapMode) -> bool {
        #[cfg(debug_assertions)]
        if map_mode == MapMode::Arena {
            return true;
        }
        if target.is_wanted() && map_mode == MapMode::AgreePvp {
            return true;
        }

        matches!(
            map_mode,
            MapMode::Pvp | MapMode::InstanceSolo | MapMode::InstanceGrp
        )
    }
}

trait Targetable {
    fn targetable_signal(&self, active_settings: &Kastrat) -> impl Signal<Item = bool> + use<Self>;
}

impl Targetable for Other {
    fn targetable_signal(&self, active_settings: &Kastrat) -> impl Signal<Item = bool> + use<> {
        let not_in_battle = self
            .emo
            .signal_vec()
            .filter(|emotion| emotion.name == EmotionName::Battle)
            .is_empty();
        let char_id = self.char_id;
        let not_from_party = Party::get()
            .signal_vec_keys()
            .filter(move |&member_id| member_id == char_id)
            .is_empty();

        map_ref! {
            let from_lvl = active_settings.lvl.contains_signal(self.lvl.signal()),
            let not_in_battle = not_in_battle,
            let not_friendly = signal::not(self.friendly_signal()),
            let not_from_party = not_from_party => {
                *from_lvl && *not_in_battle && *not_friendly && *not_from_party
            }
        }
    }
}

pub(crate) fn init() -> JsResult<()> {
    let active_settings = ActiveSettings::new(ADDON_NAME);
    let kastrat = &active_settings.kastrat;

    kastrat.target.init();

    let future = Addons::active_signal(ADDON_NAME)
        .ok_or_else(|| err_code!())?
        .switch(|addon_active| {
            kastrat
                .attack_toggle
                .signal()
                .map(move |attack_active| addon_active && attack_active)
        })
        .dedupe()
        .switch(|active| {
            Hero::get()
                .coords_signal()
                .map(move |coords| active.then_some(coords).flatten())
        })
        .dedupe()
        .switch(move |hero_coords| {
            let Some((hero_x, hero_y)) = hero_coords else {
                return signal::always(None).boxed_local();
            };
            let (hero_x, hero_y) = (hero_x as f64, hero_y as f64);

            Others::get()
                .entries_cloned()
                .filter_signal_cloned(move |(_, other)| other.targetable_signal(kastrat))
                .map_signal(|(_, other)| other.coords_signal().map(move |_| other.clone()))
                .to_signal_cloned()
                .map(move |others| {
                    // This is to prevent skipping set_candidate if current target leaves the map
                    // and there are no players closer than the target was.
                    {
                        let target_lock = kastrat.target.lock_ref();
                        if let Some(target_id) = target_lock.as_ref().map(|t| t.char_id) {
                            if !others.iter().any(|other| other.char_id == target_id) {
                                drop(target_lock);
                                kastrat.target.clear(ADDON_NAME);
                            }
                        }
                    }

                    others
                        .into_iter()
                        .map(|candidate| {
                            let distance = (candidate.x.get() as f64 - hero_x)
                                .hypot(candidate.y.get() as f64 - hero_y);
                            (candidate, distance)
                        })
                        .min_by(|(_, distance_a), (_, distance_b)| distance_a.total_cmp(distance_b))
                        .map(|(other, dist)| (other, dist, (hero_x, hero_y)))
                })
                .boxed_local()
        })
        .for_each(move |best_candidate| async move {
            kastrat.target.set_candidate(best_candidate, ADDON_NAME);
        });
    wasm_bindgen_futures::spawn_local(future);

    let future = async move {
        loop {
            let future = kastrat
                .target
                .signal_ref(move |value| {
                    signal::option(value.as_ref().map(TargetData::in_attack_range_signal)).map(
                        |in_range| {
                            // debug_log!(@f "in_range: {in_range:?}");
                            if let None | Some(false) = in_range {
                                return false;
                            }
                            let map_mode = get_engine()
                                .map()
                                .unwrap_js()
                                .map_data()
                                .unwrap_js()
                                .get_pvp()
                                .unwrap_or(MapMode::NonPvp);
                            if !Settings::can_attack_on_map(&kastrat.target, map_mode) {
                                return false;
                            }
                            if Hero::in_battle() {
                                return false;
                            }

                            true
                        },
                    )
                })
                .flatten()
                .dedupe()
                .wait_for(true);
            future.await;

            #[cfg(debug_assertions)]
            let _ = message(&format!(
                "[MDMA::RS] Atakuję \"{}\"...",
                kastrat
                    .target
                    .lock_ref()
                    .as_ref()
                    .unwrap_js()
                    .nick
                    .lock_ref()
                    .deref()
            ));

            let mut attack_req = format!("fight&a=attack&id=");
            attack_req.push_str(
                kastrat
                    .target
                    .lock_ref()
                    .as_ref()
                    .unwrap_js()
                    .char_id
                    .to_string()
                    .as_str(),
            );

            // Fallback 100ms delay
            if futures::join!(send_request(attack_req), delay(100))
                .0
                .is_err()
            {
                console_error!()
            }
        }
    };
    wasm_bindgen_futures::spawn_local(future);

    let future = kastrat
        .target
        .signal_ref(move |value| {
            signal::option(value.as_ref().map(TargetData::in_attack_range_signal))
                .map(|jd| jd.is_some_and(|cond| cond))
        })
        .flatten()
        .dedupe()
        .for_each(move |attacking| {
            match attacking && Addons::is_active(ADDON_NAME) && kastrat.attack_toggle.get() {
                true => get_engine().idle_json().unwrap_js().set_diff(MIN_DIFF),
                false => get_engine().idle_json().unwrap_js().set_default_diff(),
            }

            async {}
        });
    wasm_bindgen_futures::spawn_local(future);

    let future = Addons::active_signal(ADDON_NAME)
        .ok_or_else(|| err_code!())?
        .to_stream()
        .skip(1)
        .for_each(move |active| {
            let is_attacking = kastrat
                .target
                .lock_ref()
                .deref()
                .as_ref()
                .is_some_and(|target| target.in_attack_range())
                && kastrat.attack_toggle.get();

            match active && is_attacking {
                true => get_engine().idle_json().unwrap_js().set_diff(MIN_DIFF),
                false => get_engine().idle_json().unwrap_js().set_default_diff(),
            }

            async {}
        });
    wasm_bindgen_futures::spawn_local(future);

    wasm_bindgen_futures::spawn_local(async move {
        wait_for_without_timeout(
            || get_engine().get_all_init().is_some_and(|all_init| all_init),
            2000,
        )
        .await;

        let kastrat = &active_settings.kastrat;
        let target = &kastrat.target;
        // TODO: Verify this works when the target moves.
        let target_x = target
            .signal_ref(|target_opt| {
                signal::option(
                    target_opt
                        .as_ref()
                        .map(|target_data| target_data.x.signal()),
                )
                .map(Option::flatten)
            })
            .flatten();
        let target_y = target
            .signal_ref(|target_opt| {
                signal::option(
                    target_opt
                        .as_ref()
                        .map(|target_data| target_data.y.signal()),
                )
                .map(Option::flatten)
            })
            .flatten();
        let signal = map_ref! {
            let target_x = target_x,
            let target_y = target_y => {
                    target_x.zip(*target_y)
            }
        };
        let future = signal.for_each(move |coords| {
            let ret = async {};
            if !Addons::is_active(ADDON_NAME) || !active_settings.kastrat.attack_toggle.get() {
                return ret;
            }
            if coords.is_none() && kastrat.return_active.get() {
                let dest = Pos::new(
                    kastrat.return_pos.x.get() as usize,
                    kastrat.return_pos.y.get() as usize,
                );
                wasm_bindgen_futures::spawn_local(async move {
                    delay_range(1000, 2000).await;
                    if active_settings.kastrat.target.lock_ref().is_none() {
                        if let Err(err_code) = pathfind_to(dest).await {
                            console_error!(err_code);
                        }
                    }
                });

                return ret;
            }

            let Some((target_x, target_y)) = coords else {
                return ret;
            };
            let dest = Pos::new(target_x as usize, target_y as usize);
            debug_log!(@f "new dest: {dest:?}");
            wasm_bindgen_futures::spawn_local(async move {
                if let Err(err_code) = pathfind_to(dest).await {
                    console_error!(err_code);
                }
            });

            ret
        });
        wasm_bindgen_futures::spawn_local(future);
    });

    // Stop in place if attack toggled off.
    let future = active_settings
        .kastrat
        .attack_toggle
        .signal()
        .for_each(|attack| async move {
            if attack {
                return;
            }

            let Some(hero_x) = Hero::get().x.get() else {
                return;
            };
            let Some(hero_y) = Hero::get().y.get() else {
                return;
            };

            let dest = Pos::new(hero_x as usize, hero_y as usize);
            wasm_bindgen_futures::spawn_local(async move {
                debug_log!("stop in place");
                if let Err(err_code) = pathfind_to(dest).await {
                    console_error!(err_code);
                }
            });
        });
    wasm_bindgen_futures::spawn_local(future);

    // Antyduch, try auto move:
    // 1. Every 3 - 3:40 minutes
    // 2. On stasis change
    // 3. On stasis_incoming_seconds change
    let anti_afk = &active_settings.anti_afk;
    anti_afk.exit_stasis_on_signal(
        signal::from_stream(futures::stream::repeat(()).then(|_| delay_range(180_000, 220_000)))
            .map(|_| ()),
    );
    anti_afk.exit_stasis_on_signal(
        signal::or(
            Hero::get().stasis.signal(),
            Hero::get()
                .stasis_incoming_seconds
                .signal_ref(Option::is_some),
        )
        .dedupe()
        .map(|_| ()),
    );

    // Berserker try walk to and attack mob:
    // 1. On new npc which has the specified nick
    // 2. On npc name setting change
    let berserker = &active_settings.berserker;
    let future = berserker
        .npc_name
        .signal_cloned()
        .switch_signal_vec(|set_npc_name| {
            let set_npc_name = set_npc_name.to_lowercase();
            Npcs::get()
                .entries_cloned()
                .filter_map(move |(npc_id, npc)| {
                    (NpcTemplates::get()
                        .lock_ref()
                        .get(&npc.template_id)?
                        .nick
                        .as_deref()?
                        .to_lowercase()
                        == set_npc_name)
                        .then(|| (npc_id, Pos::new(npc.x as usize, npc.y as usize)))
                })
        })
        .to_signal_cloned()
        .map(|npcs| npcs.into_iter().next()) // we're only interested in the first npc
        .for_each(move |npc_opt| async move {
            wait_for_without_timeout(
                || get_engine().get_all_init().is_some_and(|all_init| all_init),
                2000,
            )
            .await;

            let Some((id, dest)) = npc_opt else {
                return;
            };

            // Walk to npc
            if let Err(err_code) = go_to(dest).await {
                console_error!(err_code);
                return;
            };

            // Wait at npc location
            delay_range(berserker.delay.min.get(), berserker.delay.max.get()).await;

            // Max 10 tries idc
            for _ in 0..=10 {
                // Go again before attack
                if let Err(err_code) = go_to(dest).await {
                    console_error!(err_code);
                    return;
                };

                debug_log!("attacking:", id);
                let task = format!(
                    "fight&a=attack&id=-{id}{}",
                    match berserker.fast_fight.get() {
                        true => "&ff=1",
                        false => "",
                    }
                );
                // Attack npc
                if let Err(err_code) = send_task(&task) {
                    console_error!(err_code);
                    return;
                }

                // Wait for kill
                delay_range(3_000, 5_000).await;

                // If killed return else recurse
                if !Npcs::get().lock_ref().contains_key(&id) {
                    return;
                }
            }
        });
    wasm_bindgen_futures::spawn_local(future);

    html::init(active_settings)
}

async fn go_to(dest: Pos) -> JsResult<()> {
    // Max 10 tries idc
    for _ in 0..=10 {
        if !Addons::is_active(ADDON_NAME) {
            return Ok(());
        }

        debug_log!(@f "pathfinding to: {dest:?}");
        pathfind_to(dest).await?;

        let Some(hero_pos) = Pos::current_hero_pos() else {
            return Ok(());
        };

        if hero_pos.is_within_one_tile(&dest) {
            return Ok(());
        }

        // Wait before next try
        delay_range(2_000, 5_000).await;
    }

    Ok(())
}

async fn exit_stasis() -> JsResult<()> {
    wait_for_without_timeout(
        || get_engine().get_all_init().is_some_and(|all_init| all_init),
        2000,
    )
    .await;

    let directions: [((isize, isize), char); 4] =
        [((0, -1), 'w'), ((0, 1), 's'), ((-1, 0), 'a'), ((1, 0), 'd')];
    let current_pos = Pos::new(
        Hero::get().x.get().ok_or_else(|| err_code!())? as usize,
        Hero::get().y.get().ok_or_else(|| err_code!())? as usize,
    );

    let stasis_incoming = Hero::get().stasis_incoming_seconds.get().is_some();
    let directions: Vec<char> = directions
        .into_iter()
        .filter_map(|((dx, dy), key)| {
            let next_pos = Pos::new(
                current_pos.x.wrapping_add_signed(dx),
                current_pos.y.wrapping_add_signed(dy),
            );

            debug_log!(
                "COL AT",
                next_pos.x,
                next_pos.y,
                ":",
                MapCollisions::collision_at(next_pos).unwrap_js()
            );
            let no_collision = match stasis_incoming {
                false => !MapCollisions::collision_at(next_pos).ok()?,
                true => !Collisions::collision_at(next_pos).ok()?,
            };

            no_collision.then_some(key)
        })
        .collect();

    let rand_index = (js_sys::Math::random() * directions.len() as f64).floor() as usize;
    let Some(&random_dir) = directions.get(rand_index) else {
        return Err(err_code!());
    };

    debug_log!("RANDOM DIRECTION IS:", random_dir.to_string().as_str());

    Port::send(
        Task::builder()
            .task(Tasks::KeyDown)
            .target(Targets::Background)
            .key(random_dir)
            .build()?
            .to_value()?,
    )
    .await;

    let (min_delay, max_delay) = match stasis_incoming {
        true => (200, 400),
        false => (400, 600),
    };
    delay_range(min_delay, max_delay).await;

    Port::send(
        Task::builder()
            .task(Tasks::KeyUp)
            .target(Targets::Background)
            .key(random_dir)
            .build()?
            .to_value()?,
    )
    .await;

    Ok(())
}
