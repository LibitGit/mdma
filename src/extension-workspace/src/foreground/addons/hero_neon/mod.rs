// TODO(maybe): Below/over hero's whoisHereGlow, transparency (check if needs additional color step for initial transparency to not be 1)

// TODO: Add linear/radial gradient like in discord's new roles.
mod html;

use std::{cell::Cell, ops::Deref};

use futures_signals::signal::Mutable;
#[cfg(feature = "ni")]
use js_sys::Function;
use proc_macros::ActiveSettings;
#[cfg(feature = "ni")]
use serde::Serialize;
#[cfg(feature = "ni")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "ni")]
use web_sys::CanvasRenderingContext2d;

use crate::prelude::*;

const ADDON_NAME: AddonName = AddonName::HeroNeon;
const DEFAULT_RADIUS: f64 = 28.0;
const DEFAULT_ROTATION_STEP: u16 = 10;
const DEFAULT_OFFSET: f32 = 0.3;

#[derive(Default)]
struct Rgb {
    red: u8,
    green: u8,
    blue: u8,
}

impl Rgb {
    fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

fn hex_to_rgb(hex: &str) -> JsResult<Rgb> {
    let hex = hex.strip_prefix('#').ok_or_else(|| err_code!())?;
    let red = u8::from_str_radix(&hex[0..2], 16).map_err(map_err!(from))?;
    let green = u8::from_str_radix(&hex[2..4], 16).map_err(map_err!(from))?;
    let blue = u8::from_str_radix(&hex[4..6], 16).map_err(map_err!(from))?;

    Ok(Rgb::new(red, green, blue))
}

#[derive(ActiveSettings)]
struct ActiveSettings {
    /// Radius of the neon.
    radius: Mutable<f64>,
    /// Offset for when the color should start fading out (0 to 1).
    offset: Mutable<f32>,
    /// Color displayed at the beginning of an animation cycle.    
    start_color: Mutable<String>,
    /// Color displayed at the end of an animation cycle.    
    end_color: Mutable<String>,
    /// Whether to interpolate between start and end colors.
    interpolate: Mutable<bool>,
    /// Step of color interpolation animation.
    step: Mutable<u16>,
    #[cfg(not(feature = "ni"))]
    #[setting(skip)]
    left: Mutable<f64>,
    #[cfg(not(feature = "ni"))]
    #[setting(skip)]
    top: Mutable<f64>,
    #[setting(skip)]
    interpolation_progress: Cell<u16>,
}

impl Default for ActiveSettings {
    fn default() -> Self {
        Self {
            radius: Mutable::new(DEFAULT_RADIUS),
            offset: Mutable::new(DEFAULT_OFFSET),
            start_color: Mutable::new(String::from("#000000")),
            end_color: Mutable::new(String::from("#000000")),
            interpolate: Mutable::default(),
            step: Mutable::new(DEFAULT_ROTATION_STEP),
            #[cfg(not(feature = "ni"))]
            left: Mutable::default(),
            #[cfg(not(feature = "ni"))]
            top: Mutable::default(),
            interpolation_progress: Cell::default(),
        }
    }
}

impl ActiveSettings {
    fn interpolate_color(&self) -> JsResult<Rgb> {
        let start = hex_to_rgb(self.start_color.lock_ref().deref())?;
        let end = hex_to_rgb(self.end_color.lock_ref().deref())?;

        // Update raw progress (0-1000 cycle: 0-500 forward, 500-1000 backward)
        let current_raw = self.interpolation_progress.get();
        let new_raw = (current_raw + self.step.get()) % 1000;
        self.interpolation_progress.set(new_raw);

        // Convert to triangle wave (0-500-0)
        let triangle_progress = if new_raw <= 500 {
            new_raw // 0 to 500
        } else {
            1000 - new_raw // 500 back to 0
        };

        let t = triangle_progress as f64 / 500.0;

        Ok(Rgb::new(
            Self::lerp_u8(start.red, end.red, t),
            Self::lerp_u8(start.green, end.green, t),
            Self::lerp_u8(start.blue, end.blue, t),
        ))
    }

    /// Linear interpolation helper function.
    fn lerp_u8(start: u8, end: u8, t: f64) -> u8 {
        let result = start as f64 + t * (end as f64 - start as f64);
        result.round().clamp(0.0, 255.0) as u8
    }
}

#[cfg(feature = "ni")]
impl ActiveSettings {
    fn draw(&self, ctx: &CanvasRenderingContext2d) -> JsResult<()> {
        ctx.save();

        let hero = get_engine().hero().ok_or_else(|| err_code!())?;
        let rx = hero.rx().ok_or_else(|| err_code!())?;
        let ry = hero.ry().ok_or_else(|| err_code!())?;
        let offset = get_engine()
            .map()
            .ok_or_else(|| err_code!())?
            .get_offset()
            .ok_or_else(|| err_code!())?;

        // Calculate center of the rectangle
        let center_x = rx * 32.0 + 16.0 - offset[0];
        let center_y = ry * 32.0 + 24.0 - offset[1];
        let radius = self.radius.get() * std::f64::consts::SQRT_2;

        let gradient = ctx
            .create_radial_gradient(center_x, center_y, 0.0, center_x, center_y, radius)
            .map_err(map_err!())?;

        let color = match self.interpolate.get() {
            true => self.interpolate_color()?,
            false => hex_to_rgb(self.start_color.lock_ref().deref())?,
        };

        gradient
            .add_color_stop(
                self.offset.get(),
                &format!("rgba({}, {}, {}, 1)", color.red, color.green, color.blue),
            )
            .map_err(map_err!())?;
        gradient
            .add_color_stop(
                0.7,
                &format!("rgba({}, {}, {}, 0)", color.red, color.green, color.blue),
            )
            .map_err(map_err!())?;

        ctx.set_fill_style_canvas_gradient(&gradient);
        ctx.begin_path();
        ctx.arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI)
            .map_err(map_err!())?;
        ctx.fill();

        ctx.restore();

        Ok(())
    }

    fn get_order_factory() -> Function {
        let closure = move || {
            // let epsilon =   1.1; // The order needs to be below the pet.
            // hero.ry().unwrap_js() - epsilon 
            0.0499 // 0.0001 below collisions (addon_1)
        };

        Closure::<dyn Fn() -> f64>::wrap(Box::new(closure))
            .into_js_value()
            .unchecked_into()
    }

    /// None when e.g. switching maps
    fn get_drawable_obj(&'static self) -> JsResult<Option<JsValue>> {
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

        let hero = get_engine().hero().ok_or_else(|| err_code!())?;
        let draw = closure!(@once move |ctx: &CanvasRenderingContext2d| {
            if let Err(err_code) = self.draw(ctx) {
                console_error!(err_code);
            }
        });
        let rx = match hero.rx() {
            Some(rx) => rx,
            None => return Ok(None),
        };
        let ry = match hero.ry() {
            Some(ry) => ry,
            None => return Ok(None),
        };
        let export = ColorMarkExport {
            rx,
            ry,
            draw,
            get_order: Self::get_order_factory(),
            d: ColorMarkData {
                id: Hero::get().char_id as f64,
            },
        };

        let value = serde_wasm_bindgen::to_value(&export).map_err(map_err!(from))?;

        Ok(Some(value))
    }

    fn add_to_renderer(&'static self) -> JsResult<()> {
        let api_data = get_engine().api_data().ok_or_else(|| err_code!())?;
        let renderer = get_engine().renderer().ok_or_else(|| err_code!())?;
        let after_call_draw_add_to_renderer = closure!(move || {
            if !Addons::is_active(ADDON_NAME) {
                return;
            }

            match self.get_drawable_obj() {
                Ok(Some(drawable_obj)) => renderer.add_1(&drawable_obj),
                Ok(None) => {}
                Err(err_code) => console_error!(err_code),
            }
        });

        get_game_api()
            .add_callback_to_event(
                &api_data.call_draw_add_to_renderer(),
                &after_call_draw_add_to_renderer,
            )
            .map_err(map_err!())?;

        Ok(())
    }
}

#[cfg(not(feature = "ni"))]
impl ActiveSettings {
    fn add_to_renderer(&'static self) -> JsResult<()> {
        use futures_signals::signal::SignalExt;

        let hero = get_engine().hero().ok_or_else(|| err_code!())?;

        let hero_clone = hero.clone();
        let future = self.radius.signal().for_each(move |radius| {
            if let Err(err_code) = self.update_pos_no_draw(radius, &hero_clone) {
                console_error!(err_code);
            }

            async {}
        });
        wasm_bindgen_futures::spawn_local(future);

        let original_run = hero.get_run().ok_or_else(|| err_code!())?;
        let new_run = closure!(
            { let hero = hero.clone() },
            move || -> JsResult<JsValue> {
                let res = original_run.call0(&hero);

                if !Addons::is_active(ADDON_NAME) {
                    return res;
                }

                self.update_pos(&hero)?;

                res
            }
        );

        hero.set_run(&new_run);

        Ok(())
    }

    fn update_pos_no_draw(
        &self,
        radius: f64,
        hero: &crate::bindings::engine::hero::Hero,
    ) -> JsResult<()> {
        let rx = hero.rx().ok_or_else(|| err_code!())?;
        let ry = hero.ry().ok_or_else(|| err_code!())?;

        // Calculate center of the rectangle
        let center_x = rx * 32.0 + 16.0 - radius;
        let center_y = ry * 32.0 + 24.0 - radius;

        self.left.set_neq(center_x);
        self.top.set_neq(center_y);

        Ok(())
    }

    fn update_pos(&self, hero: &crate::bindings::engine::hero::Hero) -> JsResult<()> {
        let rx = hero.rx().ok_or_else(|| err_code!())?;
        let ry = hero.ry().ok_or_else(|| err_code!())?;

        // Calculate center of the rectangle
        let center_x = rx * 32.0 + 16.0 - self.radius.get();
        let center_y = ry * 32.0 + 24.0 - self.radius.get();

        self.left.set_neq(center_x);
        self.top.set_neq(center_y);
        // A bit hacky method for redrawing.
        self.interpolate.set(self.interpolate.get());
        Ok(())
    }

    fn color_signal(&'static self) -> impl futures_signals::signal::Signal<Item = Rgb> {
        futures_signals::map_ref! {
            let start_color = self.start_color.signal_cloned(),
            let _ = self.end_color.signal_cloned(),
            let interpolate = self.interpolate.signal() => {
                let res = match interpolate {
                    true => self.interpolate_color(),
                    false => hex_to_rgb(start_color),
                };

                match res {
                    Ok(res) => res,
                    Err(err_code) => {
                        console_error!(err_code);

                        Rgb::default()
                    }
                }
            }
        }
    }
}

pub(crate) fn init() -> JsResult<()> {
    let active_settings = ActiveSettings::new(ADDON_NAME);
    active_settings.add_to_renderer()?;

    html::init(active_settings)
}
