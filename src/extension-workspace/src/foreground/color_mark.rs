use std::{cell::RefCell, collections::HashMap};

use common::{closure, err_code};
use futures_signals::signal_map::{MapDiff, SignalMapExt};
use js_sys::Function;
use serde::Serialize;
use serde_repr::{Deserialize_repr, Serialize_repr};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use crate::{globals::others::OtherBTreeMap, prelude::*};

thread_local!(static ACTIVE_COLOR_MARKS: RefCell<HashMap<OtherId, Vec<ColorMark>>> = RefCell::new(HashMap::new()));

#[derive(Debug, Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Green = 0,
    Red,
    Lime,
    Pink,
}

impl Color {
    pub fn as_str(self) -> &'static str {
        use Color::*;

        match self {
            Green => "green",
            Red => "red",
            Lime => "lime",
            Pink => "pink",
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ColorMark {
    pub other_id: Id,
    pub margin: f64,
    pub color: Color,
    pub rx: f64,
    pub ry: f64,
    pub fh: f64,
    pub fw: f64,
    pub original_draw: Function,
    pub original_get_order: Function,
    pub addon_name: AddonName,
}

impl ColorMark {
    const WITHOUT_SORT: f64 = 20000.0;

    #[cfg(not(feature = "ni"))]
    #[inline]
    pub fn init_with_player_data(
        _color: Color,
        _addon_name: AddonName,
        _other_data: &Other,
    ) -> JsResult<()> {
        Ok(())
    }

    // TODO: Better name.
    #[cfg(feature = "ni")]
    pub fn init_with_player_data(
        color: Color,
        addon_name: AddonName,
        other_data: &Other,
    ) -> JsResult<()> {
        let other_id = other_data.char_id;
        let other = get_engine()
            .others()
            .unwrap_js()
            .get_by_id(other_id)
            .ok_or_else(|| err_code!())?;

        ACTIVE_COLOR_MARKS.with_borrow_mut(|color_marks_map| {
            // 1 - Online Peers
            // 2 - Better Who Is Here
            // 3 - Kastrat
            let color_marks_vec = color_marks_map
                .entry(other_id)
                .or_insert_with(|| Vec::with_capacity(3));
            let new_color_mark = color_marks_vec
                .first()
                .cloned()
                .map(|mut first_color_mark| {
                    first_color_mark.color = color;
                    first_color_mark.addon_name = addon_name;

                    Ok::<Self, JsValue>(first_color_mark)
                })
                .unwrap_or_else(|| {
                    let margin = 5.0;
                    let fw = other.fw().ok_or_else(|| err_code!())? + margin;
                    let fh = other.fh().ok_or_else(|| err_code!())? + margin;
                    let rx = other.rx().ok_or_else(|| err_code!())?;
                    let ry = other.ry().ok_or_else(|| err_code!())?;
                    let original_draw = other.get_draw().ok_or_else(|| err_code!())?;
                    let original_get_order = other.get_get_order().ok_or_else(|| err_code!())?;

                    other.set_get_order(&Self::get_order_factory());

                    Ok(Self {
                        fw,
                        fh,
                        rx,
                        ry,
                        other_id: other_data.char_id,
                        margin,
                        color,
                        original_draw,
                        original_get_order,
                        addon_name,
                    })
                })?;

            other.set_draw(&closure!(
                {
                    let other = other.clone(),
                    let original_draw = new_color_mark.original_draw.clone(),
                },
                move |ctx: CanvasRenderingContext2d| {
                    let old_shadow_color = ctx.shadow_color();
                    let old_shadow_blur = ctx.shadow_blur();

                    ctx.set_shadow_color(color.as_str());
                    ctx.set_shadow_blur(8.0);

                    if let Err(err_code) = original_draw.call1(&other, &ctx).map_err(map_err!()) {
                        console_error!(err_code);
                    }

                    ctx.set_shadow_color(&old_shadow_color);
                    ctx.set_shadow_blur(old_shadow_blur);
                },
            ));

            color_marks_vec.push(new_color_mark);

            Ok(())
        })
    }

    pub(crate) fn observe_others_map() {
        let future = OtherBTreeMap::get()
            .signal_map_cloned()
            .for_each(|map_diff| {
                ACTIVE_COLOR_MARKS.with_borrow_mut(|color_marks_map| match map_diff {
                    MapDiff::Insert { key, .. } => {
                        if color_marks_map.insert(key, Vec::with_capacity(3)).is_some() {
                            // FIXME: Error here when leaving stasis with kastrat on.
                            console_error!()
                        }
                    }
                    MapDiff::Remove { key } => {
                        if color_marks_map.remove(&key).is_none() {
                            console_error!();
                        }
                    }
                    MapDiff::Clear {} => {
                        color_marks_map.clear();
                    }
                    _ => {}
                });
                async {}
            });
        wasm_bindgen_futures::spawn_local(future);
    }

    pub fn has_mark(color: Color, addon_name: AddonName, other_id: &Id) -> bool {
        ACTIVE_COLOR_MARKS.with_borrow(|color_marks_map| {
            color_marks_map
                .get(other_id)
                .is_some_and(|color_marks_vec| {
                    color_marks_vec.iter().any(|color_mark| {
                        color_mark.color == color && color_mark.addon_name == addon_name
                    })
                })
        })
    }

    #[cfg(feature = "ni")]
    pub fn on_call_draw_add_to_renderer(
        renderer: &crate::bindings::engine::renderer::Renderer,
    ) -> JsResult<()> {
        ACTIVE_COLOR_MARKS.with_borrow_mut(|color_marks_map| {
            for (other_id, color_marks_vec) in color_marks_map {
                let Some(color_mark) = color_marks_vec.last_mut() else {
                    // TODO: This should never get called ideally.
                    return;
                };

                if let Err(err_code) = color_mark.update_pos(*other_id) {
                    console_error!(err_code);
                    continue;
                }
                match serde_wasm_bindgen::to_value(color_mark).map_err(common::map_err!()) {
                    Ok(drawable_obj) => renderer.add_1(&drawable_obj),
                    Err(err_code) => console_error!(err_code),
                }
            }
        });

        Ok(())
    }

    // TODO: Attach div to base, update z-index of color_mark and other, rewrite update_pos and remove Serializing on si.
    // #[cfg(not(feature = "ni"))]
    // pub fn on_call_draw_add_to_renderer(
    //     _renderer: &crate::bindings::engine::renderer::Renderer,
    // ) -> JsResult<()> {
    //     use futures_signals::signal::SignalExt;

    //     let hero = get_engine().hero().ok_or_else(|| err_code!())?;

    //     let original_run = hero.get_run().ok_or_else(|| err_code!())?;
    //     let new_run = closure!(
    //         { let hero = hero.clone() },
    //         move || -> JsResult<JsValue> {
    //             let res = original_run.call0(&hero);

    //             ACTIVE_COLOR_MARKS.with_borrow_mut(|color_marks_map| {
    //                 for (other_id, color_marks_vec) in color_marks_map {
    //                     let Some(color_mark) = color_marks_vec.last_mut() else {
    //                         // TODO: This should never get called ideally.
    //                         return;
    //                     };

    //                     if let Err(err_code) = color_mark.update_pos(*other_id) {
    //                         console_error!(err_code);
    //                         continue;
    //                     }
    //                     match serde_wasm_bindgen::to_value(color_mark).map_err(common::map_err!()) {
    //                         Ok(drawable_obj) => renderer.add_1(&drawable_obj),
    //                         Err(err_code) => console_error!(err_code),
    //                     }
    //                 }
    //             });

    //             res
    //         }
    //     );

    //     hero.set_run(&new_run);

    //     Ok(())
    // }

    pub fn init(color: Color, addon_name: AddonName, other_id: OtherId) -> JsResult<()> {
        Self::init_with_player_data(
            color,
            addon_name,
            Others::get()
                .lock_ref()
                .get(&other_id)
                .ok_or_else(|| err_code!())?,
        )
    }

    pub fn remove(color: Color, addon_name: AddonName, other_id: Id) {
        ACTIVE_COLOR_MARKS.with_borrow_mut(|color_marks_map| {
            let Some(color_marks_vec) = color_marks_map.get_mut(&other_id) else {
                return;
            };
            let Some(pos) = color_marks_vec.iter().rev().position(|color_mark| {
                color_mark.color == color && color_mark.addon_name == addon_name
            }) else {
                return;
            };
            let pos = color_marks_vec.len() - 1 - pos;
            let old_color_mark = color_marks_vec.remove(pos);
            let Some(other) = get_engine()
                .others()
                .unwrap_js()
                .get_by_id(old_color_mark.other_id)
            else {
                return;
            };

            // If this was the only color mark remove the entry completely and
            // restore the original functions.
            if color_marks_vec.is_empty() {
                // TODO: Instead of removing entries manage them according to globals.others ?
                //color_marks_map.remove(&other_id);
                other.set_get_order(&old_color_mark.original_get_order);
                other.set_draw(&old_color_mark.original_draw);
                return;
            }
            // If this wasn't the only color mark and
            // the mark removed wasn't currently being displayed
            // the original functions should stay unchanged.
            if pos != color_marks_vec.len() {
                return;
            }

            // SAFETY: We know that the vec is not empty due to the previous check.
            let new_display_color = unsafe { color_marks_vec.last().unwrap_unchecked().color };
            if old_color_mark.color == new_display_color {
                return;
            }

            other.set_draw(&closure!(
                { let other = other.clone() },
                move |ctx: CanvasRenderingContext2d| {
                    let old_shadow_color = ctx.shadow_color();
                    let old_shadow_blur = ctx.shadow_blur();

                    ctx.set_shadow_color(new_display_color.as_str());
                    ctx.set_shadow_blur(8.0);

                    if let Err(err_code) = old_color_mark.original_draw.call1(&other, &ctx).map_err(map_err!()) {
                        console_error!(err_code);
                    }

                    ctx.set_shadow_color(&old_shadow_color);
                    ctx.set_shadow_blur(old_shadow_blur);
                },
            ));
        });
    }

    #[cfg(feature = "ni")]
    pub fn update_pos(&mut self, other_id: Id) -> JsResult<()> {
        let other = get_engine()
            .others()
            .unwrap_js()
            .get_by_id(other_id)
            .or_else(|| get_engine().hero().map(JsCast::unchecked_into))
            .ok_or_else(|| err_code!())?;

        self.fw = other.fw().ok_or_else(|| err_code!())? + self.margin;
        self.fh = other.fh().ok_or_else(|| err_code!())? + self.margin;
        self.rx = other.rx().ok_or_else(|| err_code!())?;
        self.ry = other.ry().ok_or_else(|| err_code!())?;

        Ok(())
    }

    fn get_order_factory() -> Function {
        let closure = || Self::WITHOUT_SORT;

        Closure::<dyn Fn() -> f64>::wrap(Box::new(closure))
            .into_js_value()
            .unchecked_into()
    }
}

struct DrawableColorMark {
    rx: f64,
    ry: f64,
    fh: f64,
    fw: f64,
    color: Color,
}

impl DrawableColorMark {
    fn draw(self, ctx: CanvasRenderingContext2d) {
        let offset = get_engine().map().unwrap_js().get_offset().unwrap_js();
        let left = self.rx * 32.0 + 16.0 - self.fw / 2.0 - offset[0];
        let top = self.ry * 32.0 - self.fh + 34.0 - offset[1];

        ctx.set_line_width(3.0);
        ctx.set_stroke_style_str(self.color.as_str());
        ctx.stroke_rect(left, top, self.fw, self.fh);
    }
}

impl From<&ColorMark> for DrawableColorMark {
    fn from(value: &ColorMark) -> Self {
        let ColorMark {
            color,
            rx,
            ry,
            fh,
            fw,
            ..
        } = value;
        Self {
            rx: *rx,
            ry: *ry,
            fh: *fh,
            fw: *fw,
            color: *color,
        }
    }
}

impl Serialize for ColorMark {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
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

        let drawable_color_mark: DrawableColorMark = self.into();
        let draw = closure!(@once move |ctx: CanvasRenderingContext2d| {
            drawable_color_mark.draw(ctx);
        });
        ColorMarkExport {
            rx: self.rx,
            ry: self.ry,
            draw,
            get_order: ColorMark::get_order_factory(),
            d: ColorMarkData {
                id: self.other_id as f64 - 0.1,
            },
        }
        .serialize(serializer)
    }
}
