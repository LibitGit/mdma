use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[cfg(feature = "ni")]
use js_sys::{Array, JsString};
use common::{closure, err_code, throw_err_code};
use discard::Discard;
use dominator::animation::{MutableAnimation, Percentage};
use dominator::events::{Click, ContextMenu, MouseButton, MouseEnter, MouseLeave, MouseMove};
use dominator::traits::StaticEvent;
use dominator::{clone, apply_methods, html, with_node, Dom, EventOptions};
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::{SignalVec, SignalVecExt};
use js_sys::Function;
use wasm_bindgen::{intern, prelude::*};
use web_sys::{Event, HtmlCanvasElement, HtmlDivElement, HtmlInputElement};

use crate::addon_window::{prelude::*, ITEM_FRAME};
use crate::addons::znacznik::html::Listeners;
use crate::addons::znacznik::ItemContainer;
use crate::bindings::engine::types::{EquipmentItemGroup, ItemClass};
use crate::disable_items::{DisabledItems, DISABLED_ITEMS};
use crate::interface::{tips_parser::tip, ThreadLocalShadowRoot, WINDOWS_ROOT};
use crate::prelude::*;

use super::{
    animation_signal, ActiveSettings, ItemState, SelectingItem, Settings, SlotType, UpgradingMode, ADDON_NAME
};

thread_local! {
    pub(super) static ANIMATION_ACTIVE: Mutable<ItemState> = {
        let animation = Mutable::default();
        let future = animation.signal().for_each(|state| {
            let to_animate = match state {
                ItemState::Animation { item_id } => Some(item_id),
                _ => None,
            };
            ANIMATE.with_borrow_mut(|animate| *animate = to_animate);

            async {}
        });
        wasm_bindgen_futures::spawn_local(future);

        animation
    };
    pub(super) static SELECTING_ITEM: SelectingItem = SelectingItem::default();
    static ANIMATE: RefCell<Option<Id>> = const { RefCell::new(None) };
}

impl ActiveSettings {
    fn render(
        &'static self,
        settings: &'static Settings,
    ) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_left(decors::SettingsButton::new())
            .push_right(decors::CloseButton::new())
            .push_right(decors::CollapseButton::new())
            .push_right(decors::CounterBubble::builder().counter_signal(self.buffer_queue.signal_vec_cloned().len()).mixin(|b| {
                apply_methods!(b, {
                    .tip!({
                        .text_signal(map_ref! {
                            let current_size = self.buffer_queue.signal_vec_cloned().len(),
                            let buffer_limit = settings.buffer_limit_signal(self) => {
                                format!("Stan schowka: {current_size} / {buffer_limit}")
                            }
                        })
                    })
                })
            }).build())
            .build();
        let header = WindowHeader::new(decor).class_list("w[200]");

        let content = WindowContent::builder()
            .class_list("f-d[column]")
            .render_mode(settings)
            .usages_preview(settings)
            .upgrade_button(settings, self);

        AddonWindow::builder(ADDON_NAME)
            .has_item_slots(true)
            .header(header)
            .content(content)
            .build()
    }
}

impl WindowContent {
    fn upgrade_button(self, settings: &'static Settings, active_settings: &'static ActiveSettings) -> Self {
        let on_click = move |_| {
            if !can_upgrade(settings) {
                let _ = message("[MDMA::RS] Brak przedmiotów do ulepszania!");
                return;
            }

            let question = match (settings.upgrade_button.unique.get(), settings.upgrade_button.heroic.get()) {
                (false, false) => None,
                (true, false) => Some(intern("Czy na pewno chcesz użyć unikatowych przedmiotów z ekwipunku do ulepszenia?")),
                (false, true) => Some(intern("Czy na pewno chcesz użyć heroicznych przedmiotów z ekwipunku do ulepszenia?")),
                (true, true) => Some(intern("Czy na pewno chcesz użyć unikatowych oraz heroicznych przedmiotów z ekwipunku do ulepszenia?")),
            };

            if let Some(question) = question {
                let callback = closure!(move || {
                    settings.upgrade_button.on_click(active_settings, settings);
                });
                let ask_data = AskAlertData::new(question, callback);
                ask_alert(ask_data).unwrap_js();

                return
            }

            settings.upgrade_button.on_click(active_settings, settings);
        };

        fn can_upgrade(settings: &Settings) -> bool {
            let stats_validator = settings.upgrade_button.stats_validator_factory();
            let is_empty_buffer = Items::get()
                .lock_ref()
                .iter()
                .filter(|&(_, item_data)| settings.filter_buffer_item(item_data, stats_validator))
                .peekable()
                .peek()
                .is_none();
            if is_empty_buffer {
                return false;
            }

            let current_mode = settings.mode.get();
            if current_mode == UpgradingMode::Single {
                return settings.item_slots.occupied(SlotType::Single);
            }
            if current_mode == UpgradingMode::Hybrid
                && settings.item_slots.occupied(SlotType::Single)
            {
                return true;
            }

            [SlotType::Armor, SlotType::Jewelry, SlotType::Weapons]
                .into_iter()
                .any(|slot_type| {
                    settings.item_slots.occupied(slot_type)
                })
        }

        self.section(
            ContentSection::new()
                .class_list("d[flex] j-c[center] p-top[6]")
                .visible_signal(settings.upgrade_button.active.signal())
                .button(
                    Button::builder()
                        .class_list("w[min-content]")
                        .disabled_signal(settings.upgrade_button.disabled_signal_factory(settings))
                        .mixin(|b| apply_methods!(b, {
                            .tip!(settings.upgrade_button.disabled_signal_factory(settings) => {
                                .text("Brak przedmiotów do spalenia według aktualnych kryteriów.")
                            })
                        }))
                        .text("Ulepsz")
                        .on_click(on_click),
                ),
        )
    }

    fn usages_preview(self, settings: &'static Settings) -> Self {
        self.section(
            ContentSection::new()
                .class_list("t-a[center] p-top[6]")
                .text_signal(settings.usages.signal().map(|usages| {
                    let Some(count) = usages.count else {
                        return String::new();
                    };
                    let Some(limit) = usages.limit else {
                        return String::new();
                    };

                    format!("Dzienny limit: {count}/{limit}")
                })),
        )
    }

    fn render_mode(self, settings: &'static Settings) -> Self {
        let events: Mutable<Option<Listeners>> = Mutable::new(None);
        let future = SELECTING_ITEM
            .with(|selecting_item| selecting_item.active.signal())
            .for_each(move |active| {
                if let Some(listeners) = events.lock_mut().take() {
                    DISABLED_ITEMS.with_borrow_mut(DisabledItems::end_disable_items);
                    listeners.discard();
                }

                match active {
                    false => SELECTING_ITEM
                        .with(|selecting_item| selecting_item.hovering_over.clear_canvas(settings)),
                    true => events.set(Some(Listeners::new(
                        click_listener_factory(settings),
                        mouse_move_listener_factory(settings),
                    ))),
                }

                async {}
            });
        wasm_bindgen_futures::spawn_local(future);

        self.section(
            ContentSection::new()
                .visible_signal(settings.mode.signal_ref(|mode| {
                    matches!(*mode, UpgradingMode::Single | UpgradingMode::Hybrid)
                }))
                .class_list("d[flex] j-c[center] p-top[6] p-left[5] p-right[5] a-i[center]")
                .input(settings.get_input(SlotType::Single))
                .section(
                    ContentSection::new()
                        .class_list("pos[relative]")
                        .mixin(|builder| apply_methods!(builder, {
                            .tip!(settings.item_slots.single.slot.signal_ref(|slot| slot.max.is_some() && slot.current.is_some()) => {
                                .text("Poziom ulepszenia")
                                .child(html!("br"))
                                .child(html!("b", {
                                    .text_signal(settings.item_slots.single.slot.signal_ref(|slot| {
                                        let jd = || {
                                            let max = slot.max?;
                                            let current = slot.current?;
                                            let percentage = ((current as f64 / max as f64) * 100.0).clamp(0.0, 100.0);

                                            Some(format!("{current} / {max} ({percentage:.2}%)"))
                                        };

                                        jd().unwrap_or_else(String::new)
                                    }))
                                }))
                            })
                        }))
                        .section(
                            ContentSection::new()
                                .class_list("enhance-progress-bg pos[relative] z-i[-1]"),
                        )
                        .section(
                            ContentSection::new()
                                .class_list("enhance-progress-current z-i[-2]")
                                .custom_property_signal("width", settings.item_slots.single.slot.signal_ref(|slot| {
                                    let max = slot.max? as f64;
                                    let current = slot.current? as f64;
                                    let percentage = ((current / max) * 100.0).clamp(0.0, 100.0);

                                    Some(format!("{percentage:.2}%"))
                                })),
                        ),
                )
                .section(settings.get_single_receive()),
        )
        .section(
            ContentSection::new()
                .visible_signal(settings.mode.signal_ref(|mode| {
                    matches!(*mode, UpgradingMode::Group | UpgradingMode::Hybrid)
                }))
                .class_list("d[flex] j-c[center] g[20] p-top[12]")
                .input(settings.get_input(SlotType::Armor))
                .input(settings.get_input(SlotType::Jewelry))
                .input(settings.get_input(SlotType::Weapons)),
        )
    }
}

#[cfg(feature = "ni")]
fn click_listener_factory(settings: &'static Settings) -> Function {
    closure!(move |event: Event| {
        let event = Click::unchecked_from_event(event);
        let Some(item_container) = event.dyn_target::<HtmlDivElement>() else {
            SELECTING_ITEM.with(SelectingItem::deselect);
            event.stop_propagation();
            return;
        };
        let class_list = item_container.class_list();

        if class_list.contains("bag") {
            return;
        }
        
        event.stop_propagation();
        let Some((slot_type, rarity)) = SELECTING_ITEM.with(SelectingItem::deselect) else {
            return;
        };

        if !class_list.contains("item") || !class_list.contains("inventory-item") {
            return;
        }

        let Some(item_id) = Array::from(&class_list.values()).iter().find_map(|class| {
            let class = JsString::from(class);
            let search_string = "item-id-";
            if !class.starts_with(search_string, 0) {
                return None;
            }

            class
                .substring(search_string.len() as u32, class.length())
                .as_string()?
                .parse()
                .ok()
        }) else {
            return;
        };

        wasm_bindgen_futures::spawn_local(async move {
            debug_log!("SETTING ITEM IN SLOT:", item_id);
            // TODO: Schedule addon deactivation.
            if let Err(err_code) = settings.set_item_in_slot(slot_type, item_id, rarity).await {
                console_error!(err_code)
            }            
        });
    })
}

#[cfg(not(feature = "ni"))]
fn click_listener_factory(settings: &'static Settings) -> Function {
    closure!(move |event: Event| {
        let event = Click::unchecked_from_event(event);
        let Some(item_container) = event.dyn_target::<web_sys::HtmlElement>().and_then(|target| target.parent_element()) else {
            SELECTING_ITEM.with(SelectingItem::deselect);
            event.stop_propagation();
            return;
        };
        let class_list = item_container.class_list();
        let container_id = item_container.id();

        if item_container.has_attribute("bag") {
            return;
        }
        
        event.stop_propagation();
        let Some((slot_type, rarity)) = SELECTING_ITEM.with(SelectingItem::deselect) else {
            return;
        };

        if !class_list.contains("item") || !container_id.starts_with("item") {
            return;
        }

        let search_string = "item";
        let Ok(item_id) = container_id[search_string.len()..].parse() else {
            console_error!();
            return;
        };

        wasm_bindgen_futures::spawn_local(async move {
            // TODO: Schedule addon deactivation.
            if let Err(err_code) = settings.set_item_in_slot(slot_type, item_id, rarity).await {
                console_error!(err_code)
            }            
        });
    })
}

fn mouse_move_listener_factory(settings: &'static Settings) -> Function {
    return closure!(move |event: Event| {
        let event = MouseMove::unchecked_from_event(event);

        SELECTING_ITEM.with(|selecting_item| {
            let Some((item_id, item_canvas)) = get_item_data(event) else {
                // debug_log!("NO ITEM DATA");
                selecting_item.hovering_over.clear_canvas(settings);
                selecting_item.hovering_over.item_id.set_neq(None);
                return;
            };

            if selecting_item.hovering_over.item_id.get().is_some_and(|old_id| old_id == item_id) {
                return;
            }

            selecting_item.hovering_over.clear_canvas(settings);

            let Some(item_rarity) = Items::get()
                .lock_ref()
                .get(&item_id)
                .and_then(|item| item.parse_stats().map(|stats| stats.rarity))
            else {
                selecting_item.hovering_over.clear_canvas(settings);
                selecting_item.hovering_over.item_id.set_neq(None);
                return;
            };

            selecting_item.hovering_over.draw_canvas(settings, &item_canvas);
            selecting_item.hovering_over.rarity.set(Some(item_rarity));
            selecting_item.hovering_over.item_id.set(Some(item_id));

            settings.item_slots.init_animation(selecting_item.hovering_over.slot_type.get().unwrap_js(), item_id);
        });
    });

    #[cfg(feature = "ni")]
    fn get_item_data(event: MouseMove) -> Option<(Id, HtmlCanvasElement)> {
        let target = event.dyn_target::<HtmlDivElement>()?;
        let class_list = target.class_list();

        if !class_list.contains("item")
            || !class_list.contains("inventory-item")
            || class_list.contains("bag")
            || class_list.contains("disable-item-mark")
        {
            return None;
        }

        let item_canvas = target.query_selector(".icon.canvas-icon").ok()??.unchecked_into();
        let item_id = Array::from(&class_list.values()).iter().find_map(|class| {
            let class = JsString::from(class);
            let search_string = "item-id-";
            if !class.starts_with(search_string, 0) {
                return None;
            }

            class
                .substring(search_string.len() as u32, class.length())
                .as_string()?
                .parse::<Id>()
                .ok()
        })?;

        Some((item_id, item_canvas))
    }

    #[cfg(not(feature = "ni"))]
    fn get_item_data(event: MouseMove) -> Option<(Id, HtmlCanvasElement)> {
        let target = event.dyn_target::<web_sys::HtmlElement>()?;
        // debug_log!("ITEM_CANVAS:", &item_canvas);
        let target = target.parent_element().and_then(|parent| parent.dyn_into::<HtmlDivElement>().ok())?;
        let item_canvas = target.query_selector("img").ok()??;
        let class_list = target.class_list();
        let target_id = target.id();

        // debug_log!(!class_list.contains("item") , !target_id.starts_with("item")
            // , class_list.contains("disable-item-mark"));
        if !class_list.contains("item") || !target_id.starts_with("item")
            || class_list.contains("disable-item-mark") || target.has_attribute("bag")
        {
        // debug_log!("FROM CLASS LIST");
            return None;
        }

        let search_string = "item";
        let item_id = target_id[search_string.len()..].parse().ok()?;

        Some((item_id, item_canvas.unchecked_into()))
    }
}

impl Settings {
    fn mode_button(&'static self, mode: UpgradingMode) -> Button {
        Button::builder()
            .no_hover()
            .text(mode.into())
            .selected_signal(self.mode.signal_ref(move |current_mode| *current_mode == mode))
            .on_click(move |_| self.mode.set(mode))
    }

    fn receive_overlay_signal(&'static self, slot_type: SlotType) -> impl Signal<Item = Option<Dom>> {
        map_ref! {
            let slotted_id_opt = self.item_slots[slot_type].signal_ref(|upgrade_slot| upgrade_slot.item_id),
            let hovered_id_opt = SELECTING_ITEM.with(|selecting_item| selecting_item.hovering_over.item_id.signal()),
            let selecting_slot_type = SELECTING_ITEM.with(|selecting_item| selecting_item.hovering_over.slot_type.signal()) => {
                let item_id_opt = match selecting_slot_type.is_some_and(|selecting_slot| selecting_slot == slot_type) {
                    false => slotted_id_opt,
                    true => hovered_id_opt,
                };

                item_id_opt.and_then(|id| ItemContainer::find(id).ok().flatten())
            }
        }.map(move |container_opt| {
            let item_container = container_opt?;
            Some(dominator::html!("div", {
                .class!(overlay w[32] h[32] pos[absolute] z-i[1] p-e[none] u-s[none])
                .style_signal("background-image", ITEM_FRAME.with(|frame| frame.overlay_image_url_signal()))
                .style_signal(
                    "background-position-y", 
                    dominator_helpers::DomMutationSignal::new(&*item_container).map(move |_mutations| {
                        item_container.get_data_upgrade().map(|upgrade_lvl| format!("{}px", (upgrade_lvl + 2) as i32 * -32)).or_else(|| Some(String::from("-32px")))
                    })
                )
                .attr_signal("data-rarity", self.overlay_data_rarity_signal(slot_type))
            }))
        })
    }

    fn overlay_signal(&'static self, slot_type: SlotType) -> impl Signal<Item = Option<Dom>> {
        map_ref! {
            let slotted_id_opt = self.item_slots[slot_type].signal_ref(|upgrade_slot| upgrade_slot.item_id),
            let hovered_id_opt = SELECTING_ITEM.with(|selecting_item| selecting_item.hovering_over.item_id.signal()),
            let selecting_slot_type = SELECTING_ITEM.with(|selecting_item| selecting_item.hovering_over.slot_type.signal()) => {
                let item_id_opt = match selecting_slot_type.is_some_and(|selecting_slot| selecting_slot == slot_type) {
                    false => slotted_id_opt,
                    true => hovered_id_opt,
                };

                item_id_opt.and_then(|id| ItemContainer::find(id).ok().flatten())
            }
        }.map(move |container_opt| {
            let item_container = container_opt?;

            Some(dominator::html!("div", {
                .class!(overlay w[32] h[32] pos[absolute] z-i[1] p-e[none] u-s[none])
                .style_signal("background-image", ITEM_FRAME.with(|frame| frame.overlay_image_url_signal()))
                .style_signal(
                    "background-position-y", 
                    dominator_helpers::DomMutationSignal::new(&*item_container).map(move |_mutations| {
                        item_container.get_data_upgrade().map(|upgrade_lvl| format!("{}px", (upgrade_lvl + 1) as i32 * -32))
                    })
                )
                .attr_signal("data-rarity", self.overlay_data_rarity_signal(slot_type))
            }))
        })
    }

    //fn overlay_visible_signal(&'static self, slot_type: SlotType) -> impl Signal<Item = bool> {
    //    SELECTING_ITEM.with(|selecting_item| {
    //        map_ref! {
    //            let slotted_rarity_opt = self.rarity_signal(slot_type),
    //            let selected_rarity_opt = selecting_item.hovering_over.rarity.signal(),
    //            let selecting_slot = selecting_item.hovering_over.slot_type.signal() => {
    //                let rarity_opt = match selecting_slot.is_some_and(|selecting_slot| selecting_slot == slot_type) {
    //                    true => selected_rarity_opt,
    //                    false => slotted_rarity_opt,
    //                };
    //
    //                rarity_opt.is_some()
    //            }
    //        }
    //    })
    //}

    fn overlay_data_rarity_signal(&'static self, slot_type: SlotType) -> impl Signal<Item = Option<&'static str>> {
        SELECTING_ITEM.with(|selecting_item| {
            map_ref! {
                let slotted_rarity_opt = self.rarity_signal(slot_type),
                let selected_rarity_opt = selecting_item.hovering_over.rarity.signal(),
                let selecting_slot = selecting_item.hovering_over.slot_type.signal() => {
                    let rarity_opt = match selecting_slot.is_some_and(|selecting_slot| selecting_slot == slot_type) {
                        true => selected_rarity_opt,
                        false => slotted_rarity_opt,
                    };

                    rarity_opt.map(Into::into)
                }
            }
        })
    }
    //    SELECTING_ITEM.with(|selecting_item| {
    //        map_ref! {
    //            let slotted_rarity_opt = self.rarity_signal(slot_type),
    //            let selected_rarity_opt = selecting_item.hovering_over.rarity.signal(),
    //            let _ = DomMutationSignal::new(item_container.as_ref()),
    //            let overlay_value = ITEM_FRAME.with(|frame| frame.overlay_image_url_signal()),
    //            let selecting_slot = selecting_item.hovering_over.slot_type.signal() => {
    //                let rarity_opt = match selecting_slot.is_some_and(|selecting_slot| selecting_slot == slot_type) {
    //                    true => selected_rarity_opt,
    //                    false => slotted_rarity_opt,
    //                };
    //                let item_container_opt = self.item_slots[slot_type].lock_ref().item_id.and_then(|id| ItemContainer::find(id).ok().flatten());
    //                match (selecting_slot.is_some_and(|selecting_slot| selecting_slot == slot_type), item_container_opt) {
    //                    (true, Some(item_container)) => overlay_value.as_ref().map(|url| dominator::html!("div", {
    //                        .class!(overlay w[32] h[32] pos[absolute] z-i[1] p-e[none] u-s[none])
    //                        .style("background", url)
    //                        .apply(|builder| {
    //                            if let Some(upgrade_lvl) = item_container.get_data_upgrade() {
    //                                builder.style("background-position-y", format!("{}px", (upgrade_lvl + 1) as i32 * -32))
    //                            } else {
    //                                builder
    //                            }
    //                        })
    //                        .apply_if(rarity_opt.is_some(), |b| b.attr("data-rarity", rarity_opt.unwrap_js().into()))
    //                    })),
    //                    _ => None,
    //                }
    //            }
    //        }
    //    })
    //}

    fn highlight_signal(&self, slot_type: SlotType) -> impl Signal<Item = Option<Dom>> {
        SELECTING_ITEM.with(|selecting_item| {
            map_ref! {
                let slotted_rarity_opt = self.rarity_signal(slot_type),
                let selected_rarity_opt = selecting_item.hovering_over.rarity.signal(),
                let selecting_slot = selecting_item.hovering_over.slot_type.signal() => {
                    let rarity_opt = match selecting_slot.is_some_and(|selecting_slot| selecting_slot == slot_type) {
                        true => selected_rarity_opt,
                        false => slotted_rarity_opt,
                    };

                    rarity_opt.map(|rarity| html!("div", {
                        .class!(highlight z-i[1]) 
                        .attr("data-rarity", <Rarity as Into<&'static str>>::into(rarity))
                    }))
                }
            }
        })
    }

    fn get_single_receive(&'static self) -> ContentSection {
        ContentSection::new()
            .class_list("z-i[1] item-input default-cursor")
            .section(
                ContentSection::new()
                    .class_list("p-e[none] u-s[none] pos[absolute] t[1] l[1] w[32] h[32]")
                    .section(
                        ContentSection::new()
                            .class_list("t[-9] l[-6] game-item-decor pos[absolute]"),
                    )
                    .mixin(|builder| {
                        builder
                            .child_signal(self.highlight_signal(SlotType::Single))
                            .child(html!("canvas" => HtmlCanvasElement, {
                                .attr("width", "32")
                                .attr("height", "32")
                                .class!(z-i[1] p-e[none] u-s[none] pos[absolute] w[32] h[32])
                                .with_node!(canvas => {
                                    .apply(|builder| {
                                        *self.item_slots.single.preview_canvas.borrow_mut() = Some(canvas);
                                        builder
                                    })
                                })
                            }))
                            .child_signal(self.receive_overlay_signal(SlotType::Single))
                    }),
            )
    }

    fn get_slot_canvas(&'static self, slot_type: SlotType) -> Dom {
        html!("canvas" => HtmlCanvasElement, {
            .attr("width", "32")
            .attr("height", "32")
            .class!(z-i[1] p-e[none] u-s[none] pos[absolute] w[32] h[32])
            .with_node!(canvas => {
                .apply(|builder| {
                    #[cfg(feature = "ni")]
                    if self.item_slots.occupied(slot_type) {
                        let context = canvas
                            .get_context("2d")
                            .unwrap_js()
                            .unwrap_js()
                            .unchecked_into::<web_sys::CanvasRenderingContext2d>();
                        let placeholder_image = get_engine()
                            .items_manager()
                            .unwrap_js()
                            .get_placeholder_item()
                            .unwrap_js()
                            .dyn_into::<web_sys::HtmlImageElement>()
                            .unwrap_js();
                        context
                            .draw_image_with_html_image_element(&placeholder_image, 0.0, 0.0)
                            .unwrap_js();
                    }

                    self.item_slots[slot_type].lock_mut().canvas = Some(canvas);
                    builder
                })
            })
        })    
    }

    fn get_input(&'static self, slot_type: SlotType) -> Input {
        let input_builder= Input::builder()
            .class_list("z-i[1]")
            .input_type(InputType::item(html!("div", {
                .class!(p-e[none] u-s[none] pos[absolute] t[1] l[1] w[32] h[32])
                .child(html!("div", {
                    .class!(t[-9] l[-6] game-item-decor pos[absolute])
                }))
                .child_signal(self.highlight_signal(slot_type))
                .child(self.get_slot_canvas(slot_type))
                .child_signal(self.overlay_signal(slot_type))
            })));

        self.prepare_input(input_builder, slot_type)
    }

    fn prepare_input(&'static self, input_builder: Input, slot_type: SlotType) -> Input {
        input_builder
            .on_click(move |_, _| {
                if self.item_slots.occupied(slot_type) {
                    return;
                }
                SELECTING_ITEM.with(|selecting_item| {
                    selecting_item.active.set_neq(true);
                    selecting_item
                        .hovering_over
                        .init(slot_type, self);
                });
            })
            .mixin(|builder, _| {
                apply_methods!(builder, {
                    .apply_if(slot_type != SlotType::Single, |builder| builder.attr("data-type", slot_type.get_attribute()))
                    .class_signal("o[0]", self.slot_occupied_signal(slot_type))
                    .class_signal(
                        "selecting-item",
                        SELECTING_ITEM.with(|selecting_item| map_ref! {
                            let selecting = selecting_item.active.signal(),
                            let selecting_slot = selecting_item.hovering_over.slot_type.signal() => {
                                selecting_slot.is_some_and(|selecting_slot| selecting_slot == slot_type) && *selecting
                            }
                        })
                    )
                    .tip!({
                        .child_signal(map_ref! {
                            let not_found = self.item_not_found_signal(slot_type),
                            let not_selecting = SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal_ref(|active| !active)) => {
                                match *not_selecting && *not_found {
                                    true => Some(html!("div", {
                                        .style("color", "red")
                                        .text("Nie wykryto przedmiotu w ekwipunku!")
                                    })),
                                    false => None,
                                }
                            }
                        })
                        .child_signal(map_ref! {
                            let not_found = self.item_not_found_signal(slot_type),
                            let not_selecting = SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal_ref(|active| !active)) => {
                                match *not_selecting && *not_found {
                                    true => Some(html!("br", {})),
                                    false => None,
                                }
                            }
                        })
                        .text_signal(self.slot_occupied_signal(slot_type).map(|occupied| match occupied {
                            true => "PPM aby usunąć przedmiot z listy ulepszanych",
                            false => "LPM aby rozpocząć wybór",
                        }))
                        .child_signal(self.item_slots[slot_type].signal_ref(|slot| (slot.max.is_some() && slot.current.is_some()).then(|| html!("br", {}))))
                        .child_signal(self.item_slots[slot_type].signal_ref(|slot| (slot.max.is_some() && slot.current.is_some()).then(|| html!("br", {}))))
                        .text_signal(
                            self.item_slots[slot_type]
                                .signal_ref(|slot| {
                                    if slot.max.is_some() && slot.current.is_some() {
                                        return String::from("Poziom ulepszenia")
                                    }
                                    String::new()
                        }))
                        .child_signal(self.item_slots[slot_type].signal_ref(|slot| (slot.max.is_some() && slot.current.is_some()).then(|| html!("br", {}))))
                        .child_signal(self.item_slots[slot_type].signal_ref(|slot| (slot.max.is_some() && slot.current.is_some()).then(move || html!("b", {
                            .text(&{
                                let jd = || {
                                    let max = slot.max?;
                                    let current = slot.current?;
                                    let percentage = ((current as f64 / max as f64) * 100.0).clamp(0.0, 100.0);

                                    Some(format!("{current} / {max} ({percentage:.2}%)"))
                                };

                                jd().unwrap_or_else(String::new)
                            })
                        }))))
                        .children_signal_vec(self.tip_details_signal(slot_type))
                    })
                    .event(move |_: MouseEnter| {
                        if SELECTING_ITEM.with(|selecting_item| selecting_item.active.get()) {
                            return;
                        }
                        let Some(item_id) = self.item_slots[slot_type].lock_ref().item_id else {
                            return;
                        };
                        ANIMATION_ACTIVE.with(|animation_active| {
                            let selector = match cfg!(feature = "ni") {
                                true => document().query_selector(&format!(".inventory-item.item-id-{item_id} .icon.canvas-icon")),
                                false => document().query_selector(&format!("#item{item_id} > img")),
                            };
                            match selector {
                                Ok(None) | Err(_) => animation_active.set_neq(ItemState::not_found(item_id)),
                                Ok(Some(_)) => animation_active.set_neq(ItemState::start_animation(item_id)),
                            }
                        })
                    })
                    .event(|_: MouseLeave| ANIMATION_ACTIVE.with(|animation_active| animation_active.set_neq(ItemState::None)))
                    .event_with_options(&EventOptions::preventable(), move |event: ContextMenu| {
                        if event.button() != MouseButton::Right {
                            return;
                        }

                        self.item_slots.clear_slot(slot_type);
                        ANIMATION_ACTIVE
                            .with(|animation_active| animation_active.set_neq(ItemState::None));
                        event.dyn_target::<HtmlInputElement>().unwrap_js().blur().unwrap_js();
                    })
                })
            })
    }

    fn tip_details_signal(&'static self, slot_type: SlotType) -> impl SignalVec<Item = Dom> {
        self.item_not_found_signal(slot_type)
            .dedupe_map(move |not_found| {
                if *not_found {
                    return vec![];
                }

                match self.item_slots.occupied(slot_type) {
                    true => self.occupied_slot_tip(slot_type),
                    false => self.vacant_slot_tip(slot_type),
                }
            })
            .to_signal_vec()
    }
        
    fn not_upgrading_tip() -> Vec<Dom> {
        vec![
            html!("br", {}),
            html!("br", {}),
            html!("span", {
                .text("Ten przedmiot nie podlega ulepszaniu według aktualnych kryteriów.")
            })
        ]
    }

    fn occupied_slot_tip(&self, slot_type: SlotType) -> Vec<Dom> {
        let cl_items_dom = self.get_cl_items_dom(slot_type);
        if cl_items_dom.is_empty() {
            return Self::not_upgrading_tip();
        }

        let mut tip_children = vec![
            html!("br", {}),
            html!("br", {}),
            html!("div", {
                .text("Ulepszany przez:")
            }),
            html!("br", {}),
        ];
        tip_children.extend(cl_items_dom);
        tip_children
    }

    fn vacant_slot_tip(&self, slot_type: SlotType) -> Vec<Dom> {
        let cl_items_dom = self.get_cl_items_dom(slot_type);
        if cl_items_dom.is_empty() {
            return vec![];
        }

        let mut tip_children = vec![
            html!("div", {
                .text("przedmiotu ulepszanego przez:")
            }),
            html!("br", {}),
        ];
        tip_children.extend(cl_items_dom);
        tip_children

    }

    fn get_cl_items_dom(&self, slot_type: SlotType) -> Vec<Dom> {
        let item_groups = match self.mode.get() {
            UpgradingMode::Single => vec![EquipmentItemGroup::Armor, EquipmentItemGroup::Jewelry, EquipmentItemGroup::Weapons],
            mode => match slot_type {
                SlotType::Armor => vec![EquipmentItemGroup::Armor],
                SlotType::Jewelry => vec![EquipmentItemGroup::Jewelry],
                SlotType::Weapons => vec![EquipmentItemGroup::Weapons],
                SlotType::Single => {
                    if mode == UpgradingMode::Group {
                        throw_err_code!("Requested SlotType::Single tip for group mode. How?")
                    }

                    let mut groups = vec![];
                    if !self.item_slots.occupied(SlotType::Armor) || self.item_not_found(SlotType::Armor) {
                        groups.push(EquipmentItemGroup::Armor);
                    }
                    if !self.item_slots.occupied(SlotType::Jewelry) || self.item_not_found(SlotType::Jewelry) {
                        groups.push(EquipmentItemGroup::Jewelry);
                    }
                    if !self.item_slots.occupied(SlotType::Weapons) || self.item_not_found(SlotType::Weapons) {
                        groups.push(EquipmentItemGroup::Weapons);
                    }
                    groups
                }
            },
        };

        self.item_types.lock_ref().iter()
            .filter_map(|(&item_class, active)| {
                if !active {
                    return None;
                }
                if !item_groups.iter().any(|&group| item_class.is_in_group(group))
                    && (slot_type != SlotType::Single || item_class != ItemClass::Upgrade) {
                       return None;
                }

                Some(html!("span", {
                    .attr("data-cl", &(item_class as u8).to_string())
                    .class!(cl-icon d[inline-block])
                }))
            })
            .collect()
    }

    fn render(&'static self, active_settings: &'static ActiveSettings) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_right(decors::CloseButton::new())
            .build();
        let header = WindowHeader::new(decor);

        let content = WindowContent::builder()
            .class_list("f-d[column]")
            .mode_setting(self)
            .upgrade_button_settings(self)
            .item_types_setting(self)
            .buffer_setting(active_settings, self)
            .excluded_item_names_setting(self, active_settings);

        SettingsWindow::builder(ADDON_NAME)
            .header(header)
            .content(content)
            .build()
    }

    fn item_types_checkbox<'a>(&'static self, dummy: &'a Mutable<bool>, item_class: ItemClass) -> Checkbox<bool> {
        let dummy_clone = dummy.clone();
        Checkbox::builder(dummy.clone())
            .label_mixin(|builder| {
                builder.class("p-left[22]")
                    .child(html!("span", {
                        .attr("data-cl", &(item_class as u8).to_string())
                        .class!(cl-icon pos[absolute] t[2] d[inline-block])
                    }))
                    .child(html!("span", {
                        .class("p-left[26]")
                        .text(item_class.to_str_pretty())
                    }))
                    .apply_if(ItemClass::Upgrade == item_class, |b| apply_methods!(b, {
                        .tip!({
                            .text("Łup z potworów o randze ")
                            .child(html!("i", {
                                .text("elita III")
                            }))
                        })
                    }))
            })
            .on_click(move |_| {
                let mut types_lock = self.item_types.lock_mut();
                let new_value = !types_lock.get(&item_class).unwrap_js();
                types_lock.insert(item_class, new_value);
                dummy_clone.set(new_value);
            })
    }
}

impl WindowContent {
    fn excluded_item_names_setting(
        self,
        settings: &'static Settings,
        active_settings: &'static ActiveSettings,
    ) -> Self {
        let excluded_items = &settings.excluded_items;
        let exclusion_list_heading = Heading::builder()
            .text("Ignorowane przedmioty")
            .info_bubble(
                InfoBubble::builder()
                    .text("Wielkość liter nie ma znaczenia.\nW niedalekiej przyszłości pojawi się tutaj lista przedmiotów jak w dodatku Znacznik :)")
                    .build(),
            );
        let mut original_on_click = ItemNameInput::on_click_factory(excluded_items);
        let exclusion_list_input = Input::builder()
            .class_list("m-left[10]")
            .placeholder("Nazwa Przedmiotu...")
            .maxlength("60")
            .text_align(TextAlign::Left)
            .size(InputSize::Big)
            .store_root(&excluded_items.root)
            .with_tooltip(&excluded_items.custom_validity)
            .on_input(ItemNameInput::on_input_factory(excluded_items))
            .confirm_button(
                InputButton::builder()
                    .with_tooltip(&excluded_items.custom_validity)
                    .on_click(move |event, input_elem| {
                        original_on_click(event, input_elem);
                        active_settings.update_buffer(settings);
                    }),
            );
        let excluded_items_signal = excluded_items.item_names.signal_cloned().map(
            move |excluded_items_vec| {
                excluded_items_vec
                    .into_iter()
                    .map(|excluded_nick| {
                        Input::builder()
                            .disabled()
                            .class_list("m-left[10] m-bottom[6]")
                            .value(&excluded_nick)
                            .size(InputSize::Big)
                            .confirm_button(
                                InputButton::builder()
                                    .button_type(InputButtonType::Remove)
                                    .on_click(move |_, _| {
                                        excluded_items
                                            .lock_mut()
                                            .retain(|nick| *nick != excluded_nick)
                                    })
                                    .class_list("m-bottom[6]"),
                            )
                    })
                    .collect()
            })
            .to_signal_vec();

        self.heading(exclusion_list_heading)
            .input_signal_vec(excluded_items_signal)
            .input(exclusion_list_input)
    }

    fn buffer_setting(self, active_settings: &'static ActiveSettings, settings: &'static Settings) -> Self {
        let buffer_heading = Heading::builder()
            .text("Schowek na przedmioty")
            .info_bubble(
                InfoBubble::builder()
                .text_signal(|| map_ref! {
                    let current_size = active_settings.buffer_queue.signal_vec_cloned().len(),
                    let buffer_limit = settings.buffer_limit_signal(active_settings) => {
                        format!(
                            "Łup z potworów spełniający aktualne kryteria zostaje przechowany w schowku.\nPrzedmioty z łupu zostaną wykorzystane do ulepszania po jego zapełnieniu.\n\n Stan schowka: {current_size} / {buffer_limit}"
                        )
                    }
                })
                .build(),
            );
        let buffer_mode_checkbox = Checkbox::builder(settings.buffer_mode.clone()).class_list("w-s[pre-line] l-h[16] p-top[12]")
            .text_signal(settings.buffer_size.signal().map(|slots| {
                format!(
                    "Opróżnij schowek przy  ≤ {slots}\nwolny{} miejsc{} w ekwipunku",
                    if slots == 1 { "m" } else { "ch" },
                    if slots == 1 { "u" } else { "ach" },
                )
            }));
        let buffer_input = Input::builder()
            .class_list("w[200]")
            .input_type(InputType::slider(1.0, 25.0))
            .mixin(|b, _| {
                apply_methods!(b, {
                    .style_signal(
                        "background",
                        settings.buffer_size
                            .signal()
                            .map(|size| {
                                let percentage = (size as f64 / 25.0) * 100.0;
                                format!("linear-gradient(to right, rgb(57, 107, 41) {percentage:.2}%, rgb(12, 13, 13) {percentage:.2}%)")
                            })
                    )
                    .tip!({
                        .text_signal(settings.buffer_size.signal().map(|size| format!("Rozmiar schowka: {size}")))
                    })
                })
            })
            .value(settings.buffer_size.get().to_string())
            .on_input(|_event, input| {
                let value = input.value_as_number();
                settings.buffer_size.set((value as u8).clamp(1, 25));
            });
        let buffer_slider_section = ContentSection::new()
            .class_list("d[flex] f-d[row] a-i[center] j-c[space-between]")
            .section(ContentSection::new().text("Rozmiar"))
            .input(buffer_input);

        self.heading(buffer_heading)
            .section(
                ContentSection::new()
                    .class_list("d[flex] f-d[column]")
                    .section(buffer_slider_section)
                    .checkbox(buffer_mode_checkbox)
                    .checkbox(Checkbox::builder(settings.common.clone()).text("Pakuj przedmioty pospolite"))
                    .checkbox(Checkbox::builder(settings.unique.clone()).text("Pakuj przedmioty unikatowe"))
            )
    }

    fn upgrade_button_settings(self, settings: &'static Settings) -> Self {
        let heading = Heading::builder()
            .text("Przycisk Ulepsz")
            .info_bubble(InfoBubble::builder()
                .mixin(|builder| {
                    apply_methods!(builder, {
                        .tip!({
                            .text("Przycisk ")
                            .child(html!("b", {
                                .text("Ulepsz")
                            }))
                            .text(", umożliwia ulepszanie przedmiotami o rzadkości wyższej niż pospolita.")
                            .child(html!("br", {}))
                            .text("Podczas ulepszania uwzględniane są tylko typy przedmiotów wymienione na liście z zakładki ")
                            .child(html!("b", {
                                .text("Typy przedmiotów")
                            }))
                            .text(".")
                        })
                    })
                })
                .build()
            );
        let active_checkbox = Checkbox::builder(settings.upgrade_button.active.clone())
            .label_mixin(|builder| {
                builder.text("Wyświetlaj przycisk ")
                    .child(html!("b", {
                        .text("Ulepsz")
                    }))
            });
        let unique_checkbox = Checkbox::builder(settings.upgrade_button.unique.clone())
            .text("Ulepszaj przedmiotami unikatowymi");
        let heroic_checkbox = Checkbox::builder(settings.upgrade_button.heroic.clone())
            .text("Ulepszaj przedmiotami heroicznymi");
        let from_event_checkbox = Checkbox::builder(settings.upgrade_button.from_event.clone())
            .text("Ulepszaj przedmiotami z eventów");

        self.heading(heading)
            .section(
                ContentSection::new()
                    .checkbox(active_checkbox)
                    .checkbox(unique_checkbox)
                    .checkbox(heroic_checkbox)
                    .checkbox(from_event_checkbox)
                )
    }

    fn item_types_setting(self, settings: &'static Settings) -> Self {
        let mut first_column = ContentSection::new().class_list("d[flex] f-d[column]");
        let mut second_column = ContentSection::new().class_list("d[flex] f-d[column]");
        let item_types_lock = settings.item_types.lock_ref();

        for (idx, &item_class) in item_types_lock.keys().enumerate() {
            let dummy = Mutable::new(*item_types_lock.get(&item_class).unwrap_js());
            let checkbox= settings.item_types_checkbox(&dummy, item_class);
            
            match idx < item_types_lock.len() / 2 {
                true => first_column = first_column.checkbox(checkbox),
                false => second_column = second_column.checkbox(checkbox),
            };
        }

        let content = ContentSection::new()
            .class_list("d[flex] f-d[row] g[20]")
            .section(first_column)
            .section(second_column);

        self.heading(
                Heading::builder()
                .text("Typy przedmiotów")
                .info_bubble(
                    InfoBubble::builder()
                    .text("Rodzaje przedmiotów spalanych podczas ulepszania.")
                    .build(),
                ),
            )
            .section(content)
    }

    fn mode_setting(self, settings: &'static Settings) -> Self {
        self.heading(Heading::builder().text("Tryb ulepszania").class_list("m-top[0]"))
            .section(
                ContentSection::new()
                    .class_list("d[flex] f-d[row] j-c[space-around] a-i[center]")
                    .button(settings.mode_button(UpgradingMode::Single))
                    .button(settings.mode_button(UpgradingMode::Group))
                    .button(settings.mode_button(UpgradingMode::Hybrid)),
            )
    }
}


pub(super) fn init(
    settings: &'static Settings,
    active_settings: &'static ActiveSettings,
) -> JsResult<()> {
    let _active_settings_window_handle = WINDOWS_ROOT
        .try_append_dom(active_settings.render(settings)?)
        .ok_or_else(|| err_code!())?;
    let _settings_window_handle = WINDOWS_ROOT
        .try_append_dom(settings.render(active_settings)?)
        .ok_or_else(|| err_code!())?;
    Ok(())
}

pub(super) fn init_animation(
    item_container: &ItemContainer,
    item_id: Id,
) -> JsResult<()> {
    let animation = MutableAnimation::new(500.0);
    let glow_signal = map_ref! {
        let animation = animation.signal(),
        let active = animation_signal(item_id) => {
            match *active {
                true => Some(format!("rgba(62, 209, 222, {}) 0px 0px 20px {}px", animation.range_inclusive(0.0, 1.0), animation.range_inclusive(0.0, 5.0))),
                false => None,
            }
        }
    };
    let skip_jump = Rc::new(Cell::new(false));
    let item_container_dom = apply_methods!(item_container.clone().into_builder(), {
            .future(animation_signal(item_id).for_each(clone!(animation, skip_jump => move |active| {
                clone!(animation, skip_jump => async move {
                    if !active {
                        return;
                    }

                    if animation.current_percentage() != Percentage::START {
                        animation.jump_to(Percentage::START);
                        skip_jump.set(true);
                    }

                    delay(200).await;
                    if ANIMATE.with_borrow(|animate_opt| *animate_opt != Some(item_id)) {
                        return;
                    }

                    animation.jump_to(Percentage::START);
                    animation.animate_to(Percentage::END);
                    let Some(bag_slot) = Items::get().lock_ref().get(&item_id).unwrap_js().get_bag_slot() else {
                        return;
                    };
                    let hero_equipment = get_engine().hero_equipment().unwrap_js();
                    if bag_slot != hero_equipment.get_active_bag().unwrap_js() {
                        hero_equipment.show_bag(bag_slot as u8).unwrap_js();
                    }
                })
            })))
            .future(animation.signal().for_each(move |t| {
                if !skip_jump.get() {
                    if t == Percentage::START {
                        animation.animate_to(Percentage::END);
                    } else if t == Percentage::END {
                        animation.animate_to(Percentage::START);
                    }
                } else {
                    skip_jump.set(false);
                }

                async {}
            }))
            .style_signal("z-index", animation_signal(item_id).map(|active| match active {
                true => Some("1"),
                false => None,
            }))
            .style_signal("border-radius", animation_signal(item_id).map(|active| match active {
                true => Some("10px"),
                false => None,
            }))
            .style_signal("box-shadow", glow_signal)
        }).into_dom();
    let container_parent = item_container.parent_node().ok_or_else(|| err_code!())?;
    dominator::replace_dom(&container_parent, item_container.as_ref(), item_container_dom);

    Ok(())
}
