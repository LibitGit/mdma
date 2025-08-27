// TODO: Make it faster on dir change by sending keydown before (or ~10ms after) previous dir keyup
// TODO: Add repeating keydown events.
// TODO: Refactor to make more readable.
use std::cell::{Cell, RefCell};

#[cfg(any(not(feature = "ni"), feature = "antyduch"))]
use common::debug_log;
use futures::channel::{mpsc, oneshot};
#[cfg(feature = "antyduch")]
use futures::{FutureExt, StreamExt};
#[cfg(any(not(feature = "ni"), feature = "antyduch"))]
use pathfinding::directed::astar::astar;
use serde::{Deserialize, Serialize};

#[cfg(any(not(feature = "ni"), feature = "antyduch"))]
use crate::globals::collisions::Collisions;
#[cfg(feature = "antyduch")]
use crate::globals::port::task::{Targets, Task, Tasks};
use crate::prelude::*;

thread_local! {
    pub(crate) static NOTIFY: RefCell<Option<mpsc::UnboundedReceiver<bool>>> = const { RefCell::new(None) };
    static CANCEL_PATHFIND: RefCell<Option<oneshot::Sender<()>>> = const { RefCell::new(None) };
    static CURRENT_DIR: Cell<Option<char>> = const { Cell::new(None) };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl Pos {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn is_within_one_tile(&self, dest: &Pos) -> bool {
        let dx = (self.x as i16 - dest.x as i16).abs();
        let dy = (self.y as i16 - dest.y as i16).abs();
        dx <= 1 && dy <= 1
    }

    pub fn current_hero_pos() -> Option<Self> {
        Hero::get()
            .x
            .get()
            .zip_with(Hero::get().y.get(), |x, y| Pos::new(x as usize, y as usize))
    }
}

#[cfg(feature = "antyduch")]
// TODO: Add a break_on arg - closure checked each step.
pub async fn pathfind_to(dest: Pos) -> JsResult<()> {
    debug_log!("PATHFINDER CALLED");
    // Create a new cancellation channel for this call
    let (tx, rx) = oneshot::channel();

    // Cancel the previous one if any
    if let Some(sender) = CANCEL_PATHFIND.replace(Some(tx)) {
        let _ = sender.send(());
    }

    // Race between cancellation and actual pathfinding
    futures::select! {
        // TODO: Rx should be fused no?
        _ = rx.fuse() => {
            debug_log!("PATHFINDING CANCELLED");
            Ok(())
        },
        result = __pathfind_to(dest).fuse() => {
            // Clear the cancellation handle only if we completed
            CANCEL_PATHFIND.take();
            result
        }
    }
}

#[cfg(feature = "antyduch")]
// TODO: Use FRP in the impl? For instance for awaiting new hero coords ?
async fn __pathfind_to(dest: Pos) -> JsResult<()> {
    let mut current_pos = Pos::new(
        Hero::get().x.get().ok_or_else(|| err_code!())? as usize,
        Hero::get().y.get().ok_or_else(|| err_code!())? as usize,
    );

    'outer: loop {
        let mut previous_dir_opt = CURRENT_DIR.take();
        debug_log!("Finding path...");

        let Some(path) = find_path_or_closest(current_pos, dest) else {
            debug_log!("NO PATH FOUND!!!");
            if let Some(dir) = previous_dir_opt {
                debug_log!("Keyup");
                press_key(KeyEvent::Up, dir).await?
            }
            return Ok(());
        };

        if let Some(first_direction) = path
            .first()
            .zip(path.get(1))
            .map(|(&from, &to)| direction(from, to))
            && let Some(dir) = previous_dir_opt
        {
            press_key(KeyEvent::Up, dir).await?;
            delay_range(0, 10).await;

            if dir == first_direction {
                press_key(KeyEvent::Down, dir).await?;
            }
        }

        debug_log!("path:", serde_wasm_bindgen::to_value(&path)?);
        debug_log!(@f "current dir: {:?}", CURRENT_DIR.get());
        debug_log!(@f "previous dir opt: {:?}", previous_dir_opt);

        for (idx, &next_pos) in path.iter().skip(1).enumerate() {
            // Hero could skip steps along the path due to multiple ml.
            if current_pos == next_pos {
                continue;
            }

            debug_log!(
                serde_wasm_bindgen::to_value(&current_pos)?,
                "->",
                serde_wasm_bindgen::to_value(&next_pos)?,
            );

            let dir = direction(current_pos, next_pos);

            let Some(previous_dir) = previous_dir_opt.as_mut() else {
                press_key(KeyEvent::Down, dir).await?;

                previous_dir_opt = Some(dir);

                let Some(steps_registered) = wait_for_step_registered().await else {
                    debug_log!("STEP NOT REGISTERED!");
                    continue 'outer;
                };
                // FIXME: what if multiple steps taken.
                if steps_registered == 1 {
                    let lift_key = path
                        .get(idx + 2) // only interested in the second step
                        .map(|&future_pos| direction(next_pos, future_pos))
                        .is_none_or(|future_dir| future_dir != dir);

                    // If no future direction or future direction != dir lift up the key.
                    if lift_key {
                        debug_log!("Keyup");
                        press_key(KeyEvent::Up, dir).await?;
                    }
                }

                if let Some(true) | None = wait_for_step_received().await? {
                    debug_log!("TIMEOUT OR BACK!");
                    continue 'outer;
                }

                let hero = Hero::get();
                let new_pos = Pos::new(
                    hero.x.get().ok_or_else(|| err_code!())? as usize,
                    hero.y.get().ok_or_else(|| err_code!())? as usize,
                );

                current_pos = new_pos;

                // Check if hero took 2 steps (in 1 request) along the path.
                // If they - didn't search for a new path.
                // NOTE: Due to the check at the beginning of the loop we're not able to check for 5 steps, only for 2.
                let mut additional_steps = 0;
                if new_pos != next_pos {
                    if path
                        .get(idx + 2)
                        .is_none_or(|&future_pos| new_pos != future_pos)
                    {
                        debug_log!("Keyup");
                        press_key(KeyEvent::Up, dir).await?;
                        _ = CURRENT_DIR.take();

                        continue 'outer;
                    }

                    additional_steps += 1;
                }

                match path_poisoned(idx + additional_steps, &path) {
                    Some(true) => continue 'outer,
                    Some(false) => continue,
                    None => return Err(err_code!()),
                };
            };

            if *previous_dir == dir {
                debug_log!("previous_dir == dir");
                let Some(steps_registered) = wait_for_step_registered().await else {
                    debug_log!("STEP NOT REGISTERED!");
                    continue 'outer;
                };
                // FIXME: what if multiple steps taken.
                if steps_registered == 1 {
                    let lift_key = path
                        .get(idx + 2)
                        .map(|&future_pos| {
                            debug_log!(@f
                                "{next_pos:?} -> {future_pos:?} future_dir: {}",
                                direction(next_pos, future_pos).to_string().as_str()
                            );
                            direction(next_pos, future_pos)
                        })
                        .is_none_or(|future_dir| future_dir != dir);

                    // If no future direction or future direction != dir lift up the key.
                    if lift_key {
                        debug_log!("in previous_dir == dir");
                        press_key(KeyEvent::Up, dir).await?;
                    }
                }

                if let Some(true) | None = wait_for_step_received().await? {
                    debug_log!("TIMEOUT OR BACK!");
                    continue 'outer;
                }

                let hero = Hero::get();
                let new_pos = Pos::new(
                    hero.x.get().ok_or_else(|| err_code!())? as usize,
                    hero.y.get().ok_or_else(|| err_code!())? as usize,
                );

                current_pos = new_pos;

                // Check if hero took 2 steps (in 1 request) along the path.
                // If they didn't - search for a new path.
                // NOTE: Due to the check at the beginning of the loop we're not able to check for 5 steps, only for 2.
                let mut additional_steps = 0;
                if new_pos != next_pos {
                    if path
                        .get(idx + 2)
                        .is_none_or(|&future_pos| new_pos != future_pos)
                    {
                        debug_log!("in *previous_dir == dir, new_pos != next_pos");
                        press_key(KeyEvent::Up, dir).await?;
                        _ = CURRENT_DIR.take();

                        continue 'outer;
                    }

                    additional_steps += 1;
                }

                match path_poisoned(idx + additional_steps, &path) {
                    Some(true) => continue 'outer,
                    Some(false) => continue,
                    None => return Err(err_code!()),
                };
            }

            debug_log!("not same dir, pressing down");
            press_key(KeyEvent::Down, dir).await?;

            *previous_dir = dir;

            let Some(steps_registered) = wait_for_step_registered().await else {
                debug_log!("STEP NOT REGISTERED!");
                continue 'outer;
            };
            // FIXME: what if multiple steps taken.
            if steps_registered == 1 {
                let lift_key = path
                    .get(idx + 2)
                    .map(|&future_pos| {
                        debug_log!(@f
                            "{next_pos:?} -> {future_pos:?} future_dir: {}",
                            direction(next_pos, future_pos).to_string().as_str()
                        );
                        direction(next_pos, future_pos)
                    })
                    .is_none_or(|future_dir| future_dir != dir);

                // If no future direction or future direction != dir lift up the key.
                if lift_key {
                    debug_log!("Keyup");
                    press_key(KeyEvent::Up, dir).await?;
                }
            }

            if let Some(true) | None = wait_for_step_received().await? {
                debug_log!("TIMEOUT OR BACK!");
                continue 'outer;
            }

            let hero = Hero::get();
            let new_pos = Pos::new(
                hero.x.get().ok_or_else(|| err_code!())? as usize,
                hero.y.get().ok_or_else(|| err_code!())? as usize,
            );

            current_pos = new_pos;

            // Check if hero took 2 steps (in 1 request) along the path.
            // If they didn't - search for a new path.
            // NOTE: Due to the check at the beginning of the loop we're not able to check for 5 steps, only for 2.
            let mut additional_steps = 0;
            if new_pos != next_pos {
                if path
                    .get(idx + 2)
                    .is_none_or(|&future_pos| new_pos != future_pos)
                {
                    debug_log!("Keyup");
                    press_key(KeyEvent::Up, dir).await?;
                    _ = CURRENT_DIR.take();

                    continue 'outer;
                }

                additional_steps += 1;
            }

            match path_poisoned(idx + additional_steps, &path) {
                Some(true) => continue 'outer,
                Some(false) => continue,
                None => return Err(err_code!()),
            };
        }

        break;
    }

    if let Some(dir) = CURRENT_DIR.take() {
        Port::send(
            Task::builder()
                .task(Tasks::KeyUp)
                .target(Targets::Background)
                .key(dir)
                .build()?
                .to_value()?,
        )
        .await;
    }

    debug_log!("Road finished.");

    Ok(())
}

#[cfg(feature = "antyduch")]
#[derive(Debug)]
enum KeyEvent {
    Up,
    Down,
}

#[cfg(feature = "antyduch")]
async fn press_key(event_type: KeyEvent, direction: char) -> JsResult<()> {
    debug_log!(@f "event_type: {event_type:?}, dir: {direction:?}");

    match event_type {
        KeyEvent::Down => {
            if let Some(prev) = CURRENT_DIR.replace(Some(direction)) {
                // console_error!();
                debug_log!(
                    "DOUBLE CURRENT DIR BUG!",
                    &prev.to_string(),
                    direction.to_string().as_str()
                );
                Port::send(
                    Task::builder()
                        .task(Tasks::KeyUp)
                        .target(Targets::Background)
                        .key(prev)
                        .build()?
                        .to_value()?,
                )
                .await;
                delay_range(0, 10).await;
            }
        }
        KeyEvent::Up => match CURRENT_DIR.replace(None) {
            Some(prev_dir) if prev_dir == direction => {
                debug_log!("prev_dir == direction")
            }
            Some(prev_dir) => {
                console_error!();
                debug_log!(
                    "WRONG DIR BUG!",
                    prev_dir.to_string().as_str(),
                    direction.to_string().as_str()
                );
            }
            None => {
                // console_error!();
                debug_log!("MISSING DIR!", direction.to_string().as_str());
            }
        },
    }

    let task = match event_type {
        KeyEvent::Down => Tasks::KeyDown,
        KeyEvent::Up => Tasks::KeyUp,
    };

    Port::send(
        Task::builder()
            .task(task)
            .target(Targets::Background)
            .key(direction)
            .build()?
            .to_value()?,
    )
    .await;

    Ok(())
}

/// Waits for a step to be received from the web socket. With a 300ms timeout.
#[cfg(feature = "antyduch")]
async fn wait_for_step_received() -> JsResult<Option<bool>> {
    let Some(receiver) = NOTIFY.with_borrow_mut(Option::take) else {
        debug_log!("NO RECEIVER");
        return Err(err_code!());
    };

    struct NotifyGuard(Option<mpsc::UnboundedReceiver<bool>>);

    impl NotifyGuard {
        fn try_next(&mut self) -> Result<Option<bool>, mpsc::TryRecvError> {
            if let Some(receiver) = self.0.as_mut() {
                return receiver.try_next();
            }

            Ok(None)
        }

        async fn next(&mut self) -> Option<bool> {
            self.0.as_mut()?.next().await
        }
    }

    impl Drop for NotifyGuard {
        fn drop(&mut self) {
            debug_log!("DROP CALLED");
            NOTIFY.with_borrow_mut(|slot| {
                *slot = self.0.take();
            });
        }
    }

    let mut receiver = NotifyGuard(Some(receiver));

    while let Ok(Some(_)) = receiver.try_next() {}

    futures::select! {
        back = receiver.next().fuse() => Ok(back),
        // TODO: Use common::sleep instead.
        _ = delay(300).fuse() => {
            debug_log!("TIMEOUT");
            Ok(None)
    },
    }
}

// FIXME: Does not work properly when >2 steps are sent (which should be never no?)
/// Waits for a step to be registered in the steps (ml on SI) array.
#[cfg(feature = "antyduch")]
async fn wait_for_step_registered() -> Option<u8> {
    let mut i = 0;
    while i < 500 {
        let step = 20;
        i = i + step as i32;
        delay_range(step, step + 10).await;

        let total_steps = get_engine()
            .steps_to_send()
            .unwrap_js()
            .get_steps()
            .unwrap_js()
            .length() as u8;
        if total_steps > 0 {
            debug_log!(
                if total_steps > 1 {
                    "MULTIPLE STEPS REGISTERED!"
                } else {
                    "Step registered!"
                },
                total_steps
            );
            return Some(total_steps);
        }
    }

    None
}

#[cfg(feature = "antyduch")]
// TODO: impl Iter<Item = &Pos> to remove from_step.
fn path_poisoned(from_step: usize, path: &Vec<Pos>) -> Option<bool> {
    Collisions::on_path(path.iter().skip(from_step))
}

#[cfg(feature = "antyduch")]
fn direction(from: Pos, to: Pos) -> char {
    let dx = to.x as i32 - from.x as i32;
    let dy = to.y as i32 - from.y as i32;

    if dx == 0 && dy == -1 {
        return 'w';
    }
    if dx == 0 && dy == 1 {
        return 's';
    }
    if dx == -1 && dy == 0 {
        return 'a';
    }

    'd'
}

#[cfg(any(not(feature = "ni"), feature = "antyduch"))]
pub(crate) fn get_heuristic(dest: Pos) -> impl Fn(&Pos) -> usize {
    move |a| {
        (a.x as isize - dest.x as isize).abs() as usize
            + (a.y as isize - dest.y as isize).abs() as usize
    }
}

#[cfg(any(not(feature = "ni"), feature = "antyduch"))]
pub(crate) fn successors(p: &Pos) -> Vec<(Pos, usize)> {
    let x = p.x;
    let y = p.y;
    let mut result = Vec::new();
    let directions = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let town_lock = Town::get().lock_ref();
    let Some(width) = town_lock.x else {
        return result;
    };
    let Some(height) = town_lock.y else {
        return result;
    };

    drop(town_lock);

    for (dx, dy) in directions {
        let nx = x as isize + dx;
        let ny = y as isize + dy;

        if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
            continue;
        }

        let new_dest = Pos::new(nx as usize, ny as usize);

        if !Collisions::collision_at(new_dest).unwrap_js() {
            result.push((new_dest, 1));
        }
    }

    // log::debug!("result: {result:?}");
    result
}

#[cfg(any(not(feature = "ni"), feature = "antyduch"))]
pub fn find_path_or_closest(start: Pos, dest: Pos) -> Option<Vec<Pos>> {
    // First try the direct path
    if !Gateways::gateway_at(&dest) {
        if let Some((path, _)) = astar(&start, successors, get_heuristic(dest), |p| *p == dest) {
            return Some(path);
        }
    } else {
        debug_log!(@f "GATEWAY AT {dest:?}");
    }

    debug_log!("No direct path found!");

    // Neighbor offsets including diagonals
    let neighbor_offsets = [
        (0, -1),
        (0, 1),
        (-1, 0),
        (1, 0),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
    ];

    // Try alternative neighbors and select the best (shortest) one
    let mut best_path: Option<(Vec<Pos>, usize)> = None;

    for (dx, dy) in neighbor_offsets {
        let alt_dest = Pos::new(
            dest.x.wrapping_add_signed(dx),
            dest.y.wrapping_add_signed(dy),
        );

        if Gateways::gateway_at(&alt_dest) {
            continue;
        }
        if let Some((path, cost)) = astar(&start, successors, get_heuristic(alt_dest), |p| {
            *p == alt_dest
        }) {
            if best_path
                .as_ref()
                .map(|(_, best_cost)| cost < *best_cost)
                .unwrap_or(true)
            {
                best_path = Some((path, cost));
            }
        }
    }

    best_path.map(|(path, _)| path)
}
