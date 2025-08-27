use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dominator::animation::{MutableAnimation, Percentage};
use dominator::events::{
    Click, ContextMenu,  MouseButton, MouseEnter, MouseLeave, MouseMove, Scroll
};
use dominator::traits::StaticEvent;
use dominator::{
    apply_methods, clone, dom_builder, html, shadow_root, with_node, Dom, DomBuilder, DomHandle, EventOptions,
};
use futures::StreamExt;
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_map::Entry;
use futures_signals::signal_vec::SignalVecExt;
use js_sys::Function;
use wasm_bindgen::{intern, JsCast};
use web_sys::{
    AddEventListenerOptions, Event, EventListenerOptions, HtmlCanvasElement, HtmlDivElement, HtmlElement, HtmlImageElement, Node
};
use discard::Discard;

use crate::addon_window::prelude::*;
use crate::interface::tips_parser::tip;
use crate::interface::{get_windows_stylesheet, ThreadLocalShadowRoot, WINDOWS_ROOT};
use crate::prelude::*;

use super::{
     ActiveSettings, DefaultDescriptors, Descriptor, ItemContainer, OwnDescriptors, SelectingItem, OwnDescriptor, OwnItemHandle, ADDON_NAME
};

const TOP_FADE_MAX: f64 = 0.15;
const BOTTOM_FADE_MIN: f64 = 1.0 - TOP_FADE_MAX;
const CONTAINER_MAX_HEIGHT: f64 = 200.0;
const THRESHOLD: f64 = CONTAINER_MAX_HEIGHT * TOP_FADE_MAX;

thread_local! {
    static ANIMATION_ACTIVE: Mutable<ItemState> = {
        let animation = Mutable::default();
        let future = animation.signal().for_each(|state| {
            let to_animate = match state {
                ItemState::Animation { item_id } => Some(item_id),
                _ => None,
            };
            //debug_log!(@f "to_animate: {to_animate:?}");
            ANIMATE.with_borrow_mut(|animate| *animate = to_animate);

            async {}
        });
        wasm_bindgen_futures::spawn_local(future);

        animation
    };
    pub(super) static SELECTING_ITEM: SelectingItem = SelectingItem::default();
    static ANIMATE: RefCell<Option<Id>> = const { RefCell::new(None) };
}

#[derive(Debug, Eq, PartialEq, Default, Clone, Copy)]
enum ItemState {
    Animation {
        item_id: Id,
    },
    Tip {
        item_id: Id,
    },
    #[default]
    None,
}

impl ItemState {
    fn not_found(item_id: Id) -> Self {
        Self::Tip { item_id }
    }

    fn start_animation(item_id: Id) -> Self {
        Self::Animation { item_id }
    }
}

fn animation_signal(current_item_id: Id) -> impl Signal<Item = bool> {
    ANIMATION_ACTIVE.with(|animation_active| animation_active.signal_ref(move |state| {
        matches!(state, ItemState::Animation { item_id } if *item_id == current_item_id && SELECTING_ITEM.with(|selecting_item| !selecting_item.active.get()))
    }))
}

fn item_not_found_signal(current_item_id: Id) -> impl Signal<Item = bool> {
    ANIMATION_ACTIVE.with(|animation_active| {
        animation_active.signal_ref(
            move |state| matches!(state, ItemState::Tip { item_id } if *item_id == current_item_id),
        )
    })
}

impl ActiveSettings {
    pub(super) fn render_dmg_type(
        &'static self,
        item_id: Id,
        dmg_type_class: &str,
        item_container: ItemContainer,
        disabled: &Mutable<bool>,
    ) -> JsResult<DomHandle> {
        let addon_data =
            Addons::get_addon(ADDON_NAME).ok_or_else(|| err_code!())?;

        let dmg_type_dom = html!("div", {
            .class(["buff", dmg_type_class])
        });
        let dmg_type_container = DomBuilder::<HtmlDivElement>::new_html(s!("div"));
        let display_dmg_type = &self.default_descriptors.display_dmg_type;
        let dmg_type_wrapper = html!("div", {
            .style_signal("display", map_ref! {
                let addon_active = addon_data.active.signal(),
                let display_dmg_type = display_dmg_type.active.signal() => {
                    match !*addon_active || !*display_dmg_type {
                        true => Some("none"),
                        false => Some("flex"),
                    }
                }
            })
            .style_signal("border", display_dmg_type.with_border.signal_ref(|active| match active {
                true => Some("1px solid rgba(255, 255, 255, 0.5)"),
                false => None,
            }))
            .style_signal("border-radius", display_dmg_type.with_border.signal_ref(|active| match active {
                true => Some("3px"),
                false => None,
            }))
            .style_signal("background", display_dmg_type.with_border.signal_ref(|active| match active {
                true => Some("rgba(0, 0, 0, 0.5)"),
                false => None,
            }))
            .style_signal("justify-content", display_dmg_type.with_border.signal_ref(|active| match active {
                true => "center",
                false => "left",
            }))
            .style_signal("align-items", display_dmg_type.with_border.signal_ref(|active| match active {
                true => "center",
                false => "end",
            }))
            .style("z-index", "2")
            .style("height", "12px")
            .style("width", "11px")
            .style("position", "absolute")
            .style("bottom", "0")
            .style("left", "0")
            .style("user-select", "none")
            .style("pointer-events", "none")
            .class_signal("o[40%]", map_ref! {
                let disabled = disabled.signal(), 
                let selecing_item = SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal()) => {
                    *disabled || ( *selecing_item && self.character_descriptors.values.lock_ref().contains_key(&item_id))
                }
            })
            .child(dmg_type_dom)
            .child(get_windows_stylesheet())
        });
        let shadow = dmg_type_container
            .__internal_shadow_root(web_sys::ShadowRootMode::Closed)
            .child(dmg_type_wrapper);
        let dmg_type_container = dmg_type_container
            .__internal_transfer_callbacks(shadow)
            .into_dom();

        let handle = dominator::append_dom(item_container.as_ref(), dmg_type_container);

        Ok(handle)
    }
}

impl Descriptor {
    pub(super) fn render(
        &self,
        only_img: &'static Mutable<bool>,
        only_text: &'static Mutable<bool>,
        disabled: &Mutable<bool>,
        item_container: ItemContainer,
    ) -> JsResult<DomHandle> {
        let signed_custom_teleports_data =
            Addons::get_addon(ADDON_NAME).ok_or_else(|| err_code!())?;
        let addon_active = &signed_custom_teleports_data.active;

        self.init_item_container_dom(only_text, &item_container, addon_active)?;
        let img: HtmlImageElement = document().create_element(intern("img")).unwrap_js().unchecked_into();
        let style = img.style();

        style.set_property(intern("width"), intern("32px")).unwrap_js();
        style.set_property(intern("height"), intern("32px")).unwrap_js();
        style.set_property(intern("object-fit"), intern("contain")).unwrap_js();
        style.set_property(intern("left"), intern("0")).unwrap_js();

        let img_opt = self.img.value.signal_cloned().map(move |src| {
            let src = src?;
            // let on_load_options = AddEventListenerOptions::new();
            // on_load_options.set_once(true);

            // let listener = &closure!(
            //     @once
            //     { let img = img.clone() },
            //     move || {
            //         let mut width = img.width() as f64;
            //         let mut height = img.height() as f64;
            //         if height > 32.0 {
            //             width *= 32.0 / height;
            //             height = 32.0;
            //         }
            //         if width > 32.0 {
            //             height *= 32.0 / width;
            //             width = 32.0;
            //         }
            //         let offset = (32.0 - width) / 2.0;
            //         img.set_width(width as u32);
            //         img.set_height(height as u32);
            //         let style = img.style();
            //         style.set_property(intern("left"), &format!("{}px", offset as i32)).unwrap_js();
            //         style.set_property(intern("display"), intern("block")).unwrap_js();
            //     }
            // );
            // img.add_event_listener_with_callback_and_add_event_listener_options(Load::EVENT_TYPE, listener, &on_load_options).unwrap_js();
            img.set_src(src.as_str());

            Some(img.clone())
        });

        let descriptor = html!("div", {
            .shadow_root!(web_sys::ShadowRootMode::Closed => {
                .child(get_windows_stylesheet())
                .child(html!(intern(s!("div")), {
                    .visible_signal(addon_active.signal())
                    .style_signal("font-size", self.text.value.signal_ref(|text_opt| text_opt.as_ref().map(|text| if text.len() < 5 { "8px" } else { "7px" })))
                    .class!(pos[absolute] t[0] l[0] w[32] h[100%] p-e[none] u-s[none] t-a[center] l-h[12] c[white] overflow[hidden])
                    .style(s!("text-shadow"), s!("0 0 2px #000"))
                    .style(s!("font-family"), s!("'Arial Bold', 'Arial Black', Gadget, sans-serif"))
                    .child_signal(map_ref! {
                        let disabled = disabled.signal(),
                        img_opt,
                        let img_active = self.img.active.signal(),
                        let selecing_item = SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal()),
                        let only_text = only_text.signal() => {
                            match *img_active && !only_text {
                                false => None,
                                true => img_opt.clone().map(|img| {
                                    if img.class_list().remove_1("o[40%]").is_err() {
                                        console_error!();
                                    }

                                    dom_builder!(img, {
                                        .class!(pos[absolute])
                                        .apply_if(*disabled || *selecing_item, |builder| builder.class("o[40%]"))
                                    })
                                })
                            }
                        }
                    })
                    .child_signal(map_ref! {
                        let disabled = disabled.signal(),
                        let selecing_item = SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal()),
                        let text_opt = self.text.value.signal_cloned(),
                        let text_active_signal = self.text.active.signal(),
                        let only_img = only_img.signal() => {
                            match *text_active_signal && !only_img {
                                true => text_opt.as_ref().map(|text| {
                                    html!(intern(s!("b")), {
                                        .text(text)
                                        .apply_if(*disabled || *selecing_item, |builder| builder.class("o[40%]"))
                                        .class!(pos[absolute] w[100%] l[0] t[0] z-i[1])
                                    })
                                }),
                                false => None,
                            }
                        }
                    })
                    .child_signal(SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal()).map(|selecting| match selecting {
                        false => None,
                        true => Some(html!("div", {
                            .class!(disable-game-item-icon z-i[1] w[100%] h[100%] bg-r[no-repeat] bg-p[center] pos[absolute] l[0] t[0] p-e[all] u-s[all])
                        })),
                    }))
                }))
            })
        });

        Ok(dominator::append_dom(item_container.as_ref(), descriptor))
    }

    pub(super) fn init_item_container_dom(
        &self,
        only_text: &'static Mutable<bool>,
        item_container: &ItemContainer,
        addon_active: &Mutable<bool>,
    ) -> JsResult<()> {
        let get_shrink_signal = move |value: &'static str| {
            map_ref! {
                let img_src_signal = self.img.value.signal_ref(|opt| opt.is_some()),
                let addon_active = addon_active.signal(),
                let img_active = self.img.active.signal(),
                let only_text = only_text.signal() => {
                    match *img_active && *img_src_signal && *addon_active && !only_text {
                        true => Some(value),
                        false => None,
                    }
                }
            }
        };

        if let Some(item_highlight) = item_container.item_highlight()? {
            let item_highlight_dom = dom_builder!(item_highlight.clone(), {
                .style_important_signal("opacity", SELECTING_ITEM.with(|selecing_item| selecing_item.active.signal()).map(|shrink| if shrink { Some("0.4") } else { None }))
            });
            dominator::replace_dom(item_container.as_ref(), &item_highlight, item_highlight_dom);
        }

        let item_canvas = item_container
            .item_canvas()
            .map_err(map_err!())?;
        let item_canvas_dom = dom_builder!(item_canvas.clone(), {
            .style_important_signal("z-index", get_shrink_signal("1"))
            .style_important_signal("bottom", get_shrink_signal("0"))
            .style_important_signal("top", get_shrink_signal("unset"))
            .style_important_signal("left", get_shrink_signal("0"))
            .style_important_signal("width", get_shrink_signal("20px"))
            .style_important_signal("height", get_shrink_signal("20px"))
            .style_important_signal("opacity", SELECTING_ITEM.with(|selecing_item| selecing_item.active.signal()).map(|shrink| if shrink { Some("0.4") } else { None }))
        });
        dominator::replace_dom(
            item_container.as_ref(),
            &item_canvas,
            item_canvas_dom,
        );

        #[cfg(feature = "ni")] 
        {
            let item_notice = item_container
                .item_notice()?;
            let item_notice_dom = dom_builder!(item_notice.clone(), {
                .style_important_signal("z-index", get_shrink_signal("1"))
                .style_important_signal("opacity", SELECTING_ITEM.with(|selecing_item| selecing_item.active.signal()).map(|shrink| if shrink { Some("0.4") } else { None }))
            });
            dominator::replace_dom(
                item_container.as_ref(),
                item_notice.as_ref(),
                item_notice_dom,
            );            

            if let Some(last_item_cooldown) = item_container.last_item_cooldown()? {
                let last_item_cooldown_dom = dom_builder!(last_item_cooldown.clone(), {
                    .style_important_signal("z-index", get_shrink_signal("1"))
                    .style_important_signal("opacity", SELECTING_ITEM.with(|selecing_item| selecing_item.active.signal()).map(|shrink| if shrink { Some("0.4") } else { None }))
                });
                dominator::replace_dom(
                    item_container.as_ref(),
                    last_item_cooldown.as_ref(),
                    last_item_cooldown_dom,
                );
            }
        }

        if let Some(item_amount) = item_container.item_amount()? {
            let item_amount_dom = dom_builder!(item_amount.clone(), {
                .style_important_signal("z-index", get_shrink_signal("1"))
                .style_important_signal("opacity", SELECTING_ITEM.with(|selecing_item| selecing_item.active.signal()).map(|shrink| if shrink { Some("0.4") } else { None }))
            });
            dominator::replace_dom(
                item_container.as_ref(),
                item_amount.as_ref(),
                item_amount_dom,
            );
        }

        Ok(())
    }
}

impl DefaultDescriptors {
    fn render(&'static self) -> ContentSection {
        let heading = Heading::builder()
            .text("Znaczniki podstawowe")
            .class_list("m[0]");
        let list_signal = self.search_text.signal_ref(
            move |search_text_opt|{
                let descriptors_lock = self.values.lock_ref();
                descriptors_lock
                    .iter()
                    .filter_map(move |(map_id, descriptor)| {
                        self.filter_descriptor(*map_id, descriptor, search_text_opt)
                    })
                    .collect()
            },
        );
        let list_len_signal = self.search_text.signal_ref(move |search_text_opt| {
            let descriptors_lock = self.values.lock_ref();
            descriptors_lock
                .iter()
                .filter(|(map_id, _)| {
                    let alias = self.alias_list.get(map_id).unwrap_js();
                    !matches!(search_text_opt.as_ref(), Some(search_text) if !alias.0.to_lowercase().contains(search_text))
                })
                .count()
        });
        let top_shadow_animation = MutableAnimation::new(0.0);
        let bottom_shadow_animation = MutableAnimation::new_with_initial(0.0, Percentage::END);
        let list_signal = list_signal.to_signal_vec();

        let default_descriptors_list = ContentSection::new()
            .class_list("g[1] d[flex] f-d[column] fade-top-bottom max-h[150] scroll-y w[250] m-top[6] b-f[glassy-blur]")
            .section_signal_vec(list_signal)
            .custom_property_signal(
                "--fade-start",
                top_shadow_animation
                    .signal()
                    .map(|t| format!("{}%", t.range_inclusive(0.0, TOP_FADE_MAX * 100.0)))
            )
            .custom_property_signal(
                "--fade-end",
                bottom_shadow_animation.signal().map(|t| {
                    format!(
                        "{}%",
                        t.invert().range_inclusive(BOTTOM_FADE_MIN * 100.0, 100.0)
                    )
                })
            )
            .mixin(|builder| {
                apply_methods!(builder, {
                    .with_node!(container => {
                        .future(clone!(top_shadow_animation, bottom_shadow_animation => async move {
                            list_len_signal.to_stream().skip(1).for_each(
                                clone!(container, top_shadow_animation, bottom_shadow_animation => move |len| {
                                    if len <= 1 {
                                        top_shadow_animation.animate_to(Percentage::START);
                                        bottom_shadow_animation.animate_to(Percentage::START);
                                    } else {
                                        update_shadow_animations(&top_shadow_animation, &bottom_shadow_animation, &container);
                                    }

                                    async {}
                                })).await
                        }))
                    })
                })
            })
            .event_with_elem(move |_event: Scroll, container: &HtmlDivElement| update_shadow_animations(&top_shadow_animation, &bottom_shadow_animation, container));
        let search_text = &self.search_text;
        let map_search_bar = Input::builder()
            .placeholder("Szukaj mapy...")
            .maxlength("100")
            .size(InputSize::Big)
            .text_align(TextAlign::Left)
            .on_input(clone!(search_text => move |_event, input_elem| {
                let value = input_elem.value();
                //debug_log!("NEW SEARCH TEXT:", &value);
                match value.is_empty() {
                    true => search_text.set_neq(None),
                    false => search_text.set_neq(Some(value)),
                }
            }))
            .class_list("m-top[6]");
        let with_border_mutable = &self.display_dmg_type.with_border;

        ContentSection::new()
            .heading(heading)
            .checkbox(
                Checkbox::builder(self.display_dmg_type.active.clone())
                    .text("Zaznaczaj typy obrażeń broni"),
            )
            .section_signal(self.display_dmg_type.active.signal_ref(
                clone!(with_border_mutable => move |active| match active {
                    false => None,
                    true => {
                        Some(ContentSection::new().checkbox(Checkbox::builder(with_border_mutable.clone()).text("Ikona typu obrażeń wewnątrz ramki")))
                    }
                }),
            ))
            .section(
                ContentSection::new()
                .class_list("p-top[6]")
                .checkbox_pair(
                    Checkbox::builder(self.only_text.clone()).text("Tylko podpisy"),
                    Checkbox::builder(self.only_img.clone()).text("Tylko ikony"),
                )
            )
            .section(default_descriptors_list)
            .input(map_search_bar)
            .class_list("d[flex] f-d[column]")
    }

    fn filter_descriptor(
        &'static self,
        map_id: Id,
        descriptor: &Descriptor,
        search_text_opt: &Option<String>,
    ) -> Option<ContentSection> {
        let alias = self.alias_list.get(&map_id).unwrap_js();
        if search_text_opt
            .as_ref()
            .is_some_and(|search_text| !alias.0.to_lowercase().contains(&search_text.to_lowercase()))
        {
            return None;
        }

        let descriptor_heading = Heading::builder().text(&alias.0).class_list("m[0]");
        let default_descriptors_map = &self.values;

        let text_active = descriptor.text.active.clone();
        let text_checkbox = Checkbox::builder(descriptor.text.active.clone())
            .text("Wyświetlaj podpis")
            .on_click(move |_event| {
                text_active.set(!text_active.get());
                let mut descriptors_lock = default_descriptors_map.lock_mut();
                let Entry::Occupied(mut entry) = descriptors_lock.entry(map_id) else {
                    console_error!();
                    return;
                };
                entry.modify_cloned(|_| {});
                //let descriptor = descriptors_lock.get(&map_id).unwrap_js().clone();
                ////Trigger MapDiff::Update "by hand"
                //descriptors_lock.insert_cloned(alias.clone(), descriptor);
            });
        let text_input = Input::builder()
            .maxlength("5")
            .placeholder("Podpis")
            .size(InputSize::Custom("w[80]"))
            .value_signal(descriptor.text.value.signal_cloned())
            .on_input(move |_event, input_elem| {
                let value = input_elem.value();
                //let descriptor = descriptors.lock_ref().get(&alias).cloned().unwrap_js();
                let mut descriptors_lock = default_descriptors_map.lock_mut();
                let Entry::Occupied(mut entry) = descriptors_lock.entry(map_id) else {
                    console_error!();
                    return;
                };
                entry.modify_cloned(|descriptor| {
                    descriptor.text.value.set(match value.is_empty() {
                        true => None,
                        false => Some(value),
                    });
                });
                //descriptors
                //    .lock_mut()
                //    .insert_cloned(alias.clone(), descriptor);
            });
        let text_setting = ContentSection::new()
            .class_list("d[flex] g[5]")
            .checkbox(text_checkbox)
            .input(text_input);

        let img_active = descriptor.img.active.clone();
        let img_checkbox = Checkbox::builder(descriptor.img.active.clone())
            .text("Wyświetlaj ikonę przedmiotu")
            .on_click(move |_event| {
                img_active.set(!img_active.get());
                let mut descriptors_lock = default_descriptors_map.lock_mut();
                let Entry::Occupied(mut entry) = descriptors_lock.entry(map_id) else {
                    console_error!();
                    return;
                };
                entry.modify_cloned(|_| {});
                //let descriptor = descriptors_lock.get(&alias).unwrap_js().clone();
                //Trigger MapDiff::Update "by hand"
                //descriptors_lock.insert_cloned(alias.clone(), descriptor);
            });
        let img_input = Input::builder()
            .class_list("p-left[5]")
            .size(InputSize::Big)
            .placeholder("Link do ikony")
            .value_signal(descriptor.img.value.signal_cloned())
            .confirm_button(
                InputButton::builder()
                    .on_click(move |_event, input_elem| {
                        let value = input_elem.value();
                        //let descriptor =
                        //    descriptors.lock_ref().get(&alias).cloned().unwrap_js();
                        let mut descriptors_lock = default_descriptors_map.lock_mut();
                        let Entry::Occupied(mut entry) = descriptors_lock.entry(map_id) else {
                            console_error!();
                            return;
                        };
                        entry.modify_cloned(|descriptor| {
                            descriptor.img.value.set(match value.is_empty() {
                                true => None,
                                false => Some(value),
                            });
                        });
                        //descriptors
                        //    .lock_mut()
                        //    .insert_cloned(alias.clone(), descriptor);
                    })
                    .tip("Zapisz link"),
            );
        let img_setting = ContentSection::new()
            .class_list("d[flex] g[5] f-d[column]")
            .checkbox(img_checkbox)
            .input(img_input);

        Some(
            ContentSection::new()
                .class_list("p[5] g[5] f-d[column] d[flex] b[1] b-r[5] bg[glassy]")
                .heading(descriptor_heading)
                .section(text_setting)
                .section(img_setting),
        )
    }
}

pub(crate) struct Listeners {
    click_listener: Function,
    mouse_move_listener: Function,
    click_options: EventListenerOptions,
}

impl Listeners {
    pub(crate) fn new(click_listener: Function, mouse_move_listener: Function) -> Self {
        let click_options = AddEventListenerOptions::new();
        click_options.set_capture(true);
        //let click_listener = Listeners::click_listener_factory(own_descriptors);
        window()
            .add_event_listener_with_callback_and_add_event_listener_options(
                Click::EVENT_TYPE,
                &click_listener,
                &click_options,
            )
            .unwrap_js();

        //let mouse_move_listener = Listeners::mouse_move_listener_factory(items);
        window()
            .add_event_listener_with_callback(MouseMove::EVENT_TYPE, &mouse_move_listener)
            .unwrap_js();

        Self {
            click_listener,
            mouse_move_listener,
            click_options: click_options.unchecked_into(),
        }
    }

    #[cfg(feature = "ni")]
    fn click_listener_factory(own_descriptors: &'static OwnDescriptors) -> Function {
        closure!(move |event: Event| {
            let event = Click::unchecked_from_event(event);
            //common::debug_log!(event.target().unwrap_js());
            event.stop_propagation();

            SELECTING_ITEM.with(|selecting_item| selecting_item.active.set(false));

            let Some(item_container) = event.dyn_target::<HtmlDivElement>() else {
                return;
            };
            let class_list = item_container.class_list();

            if !class_list.contains("item") || !class_list.contains("inventory-item") {
                return;
            }

            let Some(item_id) = js_sys::Array::from(&class_list.values()).iter().find_map(|class| {
                let class = js_sys::JsString::from(class);
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

            own_descriptors
                .values
                .lock_mut()
                .insert_cloned(item_id, OwnDescriptor::default());
        })
    }

    #[cfg(not(feature = "ni"))]
    fn click_listener_factory(own_descriptors: &'static OwnDescriptors) -> Function {
        closure!(move |event: Event| {
            let event = Click::unchecked_from_event(event);
            
            event.stop_propagation();
            SELECTING_ITEM.with(|selecting_item| selecting_item.active.set(false));

            let Some(item_container) = event.dyn_target::<web_sys::HtmlElement>().and_then(|target| target.parent_element()) else {
                return;
            };
            let class_list = item_container.class_list();
            let container_id = item_container.id();

            if !class_list.contains("item") || !container_id.starts_with("item") {
                return;
            }

            let search_string = "item";
            let Ok(item_id) = container_id[search_string.len()..].parse() else {
                console_error!();
                return;
            };

            own_descriptors
                .values
                .lock_mut()
                .insert_cloned(item_id, OwnDescriptor::default());
        })
    }

    fn mouse_move_listener_factory() -> Function {
        return closure!(move |event: Event| {
            let event = MouseMove::unchecked_from_event(event);

            SELECTING_ITEM.with(|selecting_item| {
                selecting_item.hovering_over.clear_canvas();
                
                let Some((item_id, item_canvas)) = get_item_data(event) else {
                    // debug_log!("NO ITEM DATA");
                    return;
                };

                let Some(item_rarity) = Items::get()
                    .lock_ref()
                    .get(&item_id)
                    .and_then(|item| item.parse_stats().map(|stats| stats.rarity))
                else {
                    return;
                };

                selecting_item.hovering_over.draw_canvas(&item_canvas);
                selecting_item.hovering_over.update_rarity(item_rarity);
            });

            // SELECTING_ITEM
            //     .with(|selecting_item| selecting_item.hovering_over.clear_canvas());

            // let event = MouseMove::unchecked_from_event(event);
            // let Some(target) = event.dyn_target::<HtmlDivElement>() else {
            //     return;
            // };
            // let class_list = target.class_list();

            // if !class_list.contains("item") || !class_list.contains("inventory-item") {
            //     return;
            // }

            // let Ok(Some(item_canvas)) = target.query_selector(".icon.canvas-icon") else {
            //     return;
            // };

            // let Some(item_id) =
            //     Array::from(&class_list.values()).iter().find_map(|class| {
            //         let class = JsString::from(class);
            //         let search_string = "item-id-";
            //         if !class.starts_with(search_string, 0) {
            //             return None;
            //         }

            //         class
            //             .substring(search_string.len() as u32, class.length())
            //             .as_string()?
            //             .parse::<Id>()
            //             .ok()
            //     })
            // else {
            //     //debug_log!("ITERATOR MISHAP");
            //     return;
            // };
            // let Some(item_rarity) = Items::get()
            //     .lock_ref()
            //     .get(&item_id)
            //     .and_then(|item| item.parse_stats().map(|stats| stats.rarity))
            // else {
            //     //debug_log!("Could not get item rarity");
            //     return;
            // };

            // SELECTING_ITEM.with(|selecting_item| {
            //     selecting_item
            //         .hovering_over
            //         .draw_canvas(item_canvas.unchecked_ref());
            //     selecting_item.hovering_over.update_rarity(item_rarity);
            // });
        });

        #[cfg(feature = "ni")]
        fn get_item_data(event: MouseMove) -> Option<(Id, HtmlCanvasElement)> {
            let target = event.dyn_target::<HtmlDivElement>()?;
            let class_list = target.class_list();

            if !class_list.contains("item")
                || !class_list.contains("inventory-item")
                || class_list.contains("disable-item-mark")
            {
                return None;
            }

            let item_canvas = target.query_selector(".icon.canvas-icon").ok()??.unchecked_into();
            let item_id = js_sys::Array::from(&class_list.values()).iter().find_map(|class| {
                let class = js_sys::JsString::from(class);
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
                || class_list.contains("disable-item-mark")
            {
            // debug_log!("FROM CLASS LIST");
                return None;
            }

            let search_string = "item";
            let item_id = target_id[search_string.len()..].parse().ok()?;

            Some((item_id, item_canvas.unchecked_into()))
        }
    }
}

impl Discard for Listeners {
    fn discard(self) {
        window()
            .remove_event_listener_with_callback(
                MouseMove::EVENT_TYPE,
                &self.mouse_move_listener,
            )
            .unwrap_js();
        window()
            .remove_event_listener_with_callback_and_event_listener_options(
                Click::EVENT_TYPE,
                &self.click_listener,
                &self.click_options,
            )
            .unwrap_js();
    } 
}

impl OwnDescriptors {
    fn render_user_aliases(&'static self) -> ContentSection {
        let events: Mutable<Option<Listeners>> = Mutable::new(None);
        let future = SELECTING_ITEM
            .with(|selecting_item| selecting_item.active.signal())
            .for_each(move |active| {
                if let Some(listeners) = events.lock_mut().take() {
                    listeners.discard();
                }
                match active {
                    false => SELECTING_ITEM.with(|selecting_item| selecting_item.hovering_over.clear_canvas()),
                    true => events.set(Some(Listeners::new(
                                Listeners::click_listener_factory(self),
                                Listeners::mouse_move_listener_factory()))
                        ),
                }

                async {}
            });
        wasm_bindgen_futures::spawn_local(future);

        ContentSection::new()
            .heading(Heading::builder().text("Twoje znaczniki"))
            .section(
                ContentSection::new()
                    .class_list("d[flex] f-d[row] j-c[center] flex-w[wrap] w[250]")
                    .input(
                        Input::builder()
                            .input_type(InputType::item(html!("div", {
                                .class!(p-e[none] u-s[none] t[1] l[1] w[32] h[32] pos[absolute])
                                .child_signal(SELECTING_ITEM.with(|selecting_item| {
                                    selecting_item.hovering_over.rarity.signal_ref(|rarity_opt| -> Option<Dom> {
                                        let rarity = rarity_opt.map(<Rarity as Into<&'static str>>::into)?;

                                        Some(html!("div", {
                                            .class!(highlight)
                                            .attr("data-rarity", rarity)
                                        }))
                                    })
                                }))
                                .child(html!("canvas" => HtmlCanvasElement, {
                                    .class!(p-e[none] u-s[none] pos[absolute] w[32] h[32])
                                    .attr("width", "32")
                                    .attr("height", "32")
                                    .with_node!(canvas => {
                                        .apply(move |builder| {
                                            SELECTING_ITEM.with(|selecting_item| selecting_item.hovering_over.init(canvas));
                                            builder
                                        })
                                    })
                                }))
                            })))
                            .mixin(|builder, _| {
                                apply_methods!(builder, {
                                    .event(|_: Click| {
                                        SELECTING_ITEM.with(|selecting_item| selecting_item.active.set_neq(true));
                                        //common::debug_log!("AFTER START SELECTING_ITEM");
                                    })
                                    .class_signal(
                                        "selecting-item", 
                                        SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal())
                                    )
                                    .tip!({
                                        .text_signal(SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal().map(|active| match active {
                                            true => "LPM na przedmiot w ekwipunku, aby dodać go do listy znaczników",
                                            false => "LPM aby rozpocząć wybór przedmiotu",
                                        })))
                                    })
                                })
                            }),
                    )
                    .section_signal_vec(self.values.entries_cloned().map(move |(item_id, own_descriptor)| {
                        //debug_log!(@f "item handles: {} {}", self.item_dom_handles.borrow().len(), self.item_dom_handles.borrow().contains_key(&item_id));
                        ContentSection::new().class_list("item-input").section(
                            self.item_dom_handles
                                .borrow_mut()
                                .entry(item_id)
                                .or_insert_with(|| OwnItemHandle::new(item_id, &own_descriptor))
                                .get_item_view(own_descriptor,self),
                        )
                    }))
            )
                .section_signal(self.editing.signal_ref(move |opt| {
                    let (id, descriptor) = opt.as_ref()?;
                    let id = *id;

                    let descriptor_heading = Heading::builder().text(Items::get().lock_ref().get(&id)?.name.as_ref()?.as_str()).class_list("m[0]");

                    let text_active = descriptor.text.active.clone();
                    let text_checkbox = Checkbox::builder(descriptor.text.active.clone())
                        .text("Wyświetlaj podpis")
                        .on_click(move |_event| {
                            text_active.set(!text_active.get());
                            let mut descriptors_lock = self.values.lock_mut();
                            let descriptor = descriptors_lock.get(&id).cloned().unwrap_js();
                            //Trigger MapDiff::Update "by hand"
                            descriptors_lock.insert_cloned(id, descriptor);
                        });
                    let text_input = Input::builder()
                        .maxlength("5")
                        .placeholder("Podpis")
                        .size(InputSize::Custom("w[80]"))
                        .value_signal(descriptor.text.value.signal_cloned())
                        .on_input(move |_event, input_elem| {
                            let value = input_elem.value();
                            let mut descriptors_lock = self.values.lock_mut();
                            let descriptor = descriptors_lock.get(&id).cloned().unwrap_js();
                            descriptor.text.value.set(match value.is_empty() {
                                true => None,
                                false => Some(value),
                            });
                            descriptors_lock.insert_cloned(id, descriptor);
                        });
                    let text_setting = ContentSection::new()
                        .class_list("d[flex] g[5]")
                        .checkbox(text_checkbox)
                        .input(text_input);
                    
                    let rarity_setting = descriptor.rarity.clone();
                    let rarity_checkbox = Checkbox::builder(descriptor.rarity.active.clone())
                        .text("Zmień rzadkość")
                        .on_click(move |_| {
                            rarity_setting.active.set(!rarity_setting.active.get());
                            let mut descriptors_lock = self.values.lock_mut();
                            let descriptor = descriptors_lock.get(&id).unwrap_js().clone();
                            //Trigger MapDiff::Update "by hand"
                            descriptors_lock.insert_cloned(id, descriptor);
                        });
                    let shown = Mutable::new(false);
                    let shown_clone = shown.clone();
                    let visible_signal = shown.signal();
                    let rarity_value = descriptor.rarity.value.clone();
                    let rarity_button = Button::builder()
                        .class_list("w[108] t-a[left]")
                        .no_hover()
                        .text("Rzadkość")
                        .mixin(move |builder| {
                            builder
                                .child(html!("div", {
                                    .class!(pos[absolute] r[8] align-center menu-arrow)
                                }))
                                .child_signal(visible_signal.map(move |visible| {
                                    match visible {
                                        false => None,
                                        true => Some(html!("div", {
                                            .class!(w[100] t-a[center] l[1] scroll-wrapper)
                                            .child(html!("div", {
                                                .class!(scroll-option)
                                                .text("Brak")
                                                .event_with_options(&EventOptions::bubbles(), clone!(rarity_value => move |_: Click| {
                                                    rarity_value.set(None);
                                                }))
                                            }))
                                            .child(html!("div", {
                                                .class!(scroll-option)
                                                .text("Pospolity")
                                                .event_with_options(&EventOptions::bubbles(), clone!(rarity_value => move |_: Click| {
                                                    rarity_value.set(Some(Rarity::Common));
                                                }))
                                            }))
                                            .child(html!("div", {
                                                .class!(scroll-option)
                                                .text("Unikatowy")
                                                .event_with_options(&EventOptions::bubbles(), clone!(rarity_value => move |_: Click| {
                                                    rarity_value.set(Some(Rarity::Unique));
                                                }))
                                            }))
                                            .child(html!("div", {
                                                .class!(scroll-option)
                                                .text("Heroiczny")
                                                .event_with_options(&EventOptions::bubbles(), clone!(rarity_value => move |_: Click| {
                                                    rarity_value.set(Some(Rarity::Heroic));
                                                }))
                                            }))
                                            .child(html!("div", {
                                                .class!(scroll-option)
                                                .text("Ulepszony")
                                                .event_with_options(&EventOptions::bubbles(), clone!(rarity_value => move |_: Click| {
                                                    rarity_value.set(Some(Rarity::Upgraded));
                                                }))
                                            }))
                                            .child(html!("div", {
                                                .class!(scroll-option)
                                                .text("Legendarny")
                                                .event_with_options(&EventOptions::bubbles(), clone!(rarity_value => move |_: Click| {
                                                    rarity_value.set(Some(Rarity::Legendary));
                                                }))
                                            }))
                                            .child(html!("div", {
                                                .class!(scroll-option)
                                                .text("Artefakt")
                                                .event_with_options(&EventOptions::bubbles(), clone!(rarity_value => move |_: Click| {
                                                    rarity_value.set(Some(Rarity::Artifact));
                                                }))
                                            }))
                                        }))
                                    }
                                }))
                                .event_with_options(&EventOptions::bubbles(), move |_: Click| {
                                    shown_clone.set(!shown_clone.get());
                                    //TODO: Move from here to list elements.
                                    let mut descriptors_lock = self.values.lock_mut();
                                    let descriptor = descriptors_lock.get(&id).unwrap_js().clone();
                                    //Trigger MapDiff::Update "by hand"
                                    descriptors_lock.insert_cloned(id, descriptor);
                                    //debug_log!("shown", shown_clone.get());
                                })
                        });
                    let rarity_setting = ContentSection::new()
                        .class_list("d[flex] g[5] f-d[row]")
                        .checkbox(rarity_checkbox)
                        .button(rarity_button);

                    let img_setting = descriptor.img.clone();
                    let img_checkbox = Checkbox::builder(descriptor.img.active.clone())
                        .text("Zmień ikonę")
                        .on_click(move |event| {
                            if img_setting.value.lock_ref().is_none() && !img_setting.active.get() {
                                event.prevent_default();
                                if message("Ustaw ikonę do podmiany.").is_err() {
                                    console_error!()
                                }
                                return;
                            }

                            img_setting.active.set(!img_setting.active.get());
                            let mut descriptors_lock = self.values.lock_mut();
                            let descriptor = descriptors_lock.get(&id).unwrap_js().clone();
                            //Trigger MapDiff::Update "by hand"
                            descriptors_lock.insert_cloned(id, descriptor);
                        });
                    let img_input = Input::builder()
                        .class_list("p-left[5]")
                        .size(InputSize::Big)
                        .placeholder("Link do ikony")
                        .value_signal(descriptor.img.value.signal_cloned())
                        .confirm_button(
                            InputButton::builder()
                                .on_click(move |_event, input_elem| {
                                    let value = input_elem.value();
                                    let mut descriptors_lock = self.values.lock_mut();
                                    let descriptor =
                                        descriptors_lock.get(&id).cloned().unwrap_js();
                                    descriptor.img.value.set(match value.is_empty() {
                                        true => None,
                                        false => Some(value),
                                    });
                                    descriptors_lock
                                        .insert_cloned(id, descriptor);
                                })
                                .tip("Zapisz link"),
                        );
                    let img_setting = ContentSection::new()
                        .class_list("d[flex] f-d[column]")
                        .checkbox(img_checkbox)
                        .input(img_input);
                    
                    Some(
                        ContentSection::new()
                            .class_list("p[5] g[5] f-d[column] d[flex] b[1] b-r[5] bg[glassy] b-f[glassy-blur]")
                            .heading(descriptor_heading)
                            .section(text_setting)
                            .section(rarity_setting)
                            .section(img_setting),
                    )
                }))
            .class_list("d[flex] f-d[column] g[5]")
    }

    pub(super) fn prepare_canvas(
        &'static self,
        builder: DomBuilder<HtmlCanvasElement>,
        item_id: Id,
    ) -> DomBuilder<HtmlCanvasElement> {
        apply_methods!(builder, {
            .attr("width", "32")
            .attr("height", "32")
            .style("width", "32px")
            .style("height", "32px")
            .class!(pos[absolute])
            .event(move |_: MouseEnter| {
                if SELECTING_ITEM.with(|selecting_item| selecting_item.active.get()) {
                    return;
                }
                let selectors = match cfg!(feature = "ni") {
                    true => format!(".inventory-item.item-id-{item_id} .icon.canvas-icon"),
                    false => format!("#item{item_id} > img")
                };
                ANIMATION_ACTIVE.with(|animation_active| match document().query_selector(&selectors) {
                    Ok(None) | Err(_) => animation_active.set(ItemState::not_found(item_id)),
                    Ok(Some(_)) => animation_active.set(ItemState::start_animation(item_id)),
                })
            })
            .event(|_: MouseLeave| ANIMATION_ACTIVE.with(|animation_active| animation_active.set_neq(ItemState::None)))
            .event(move |event: ContextMenu| {
                if event.button() != MouseButton::Right {
                    return;
                }

                //debug_log!("REMOVING OWN ITEM DESCRIPTOR", item_id);
                self.remove_one(&item_id);
                ANIMATION_ACTIVE.with(|animation_active| animation_active.set_neq(ItemState::None));
            })
            .tip!(SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal_ref(|active| !active)) => {
                .child_signal(item_not_found_signal(item_id).map(|active| match active {
                    true => Some(html!("div", {
                        .style("color", "red")
                        .text("Nie wykryto przedmiotu w ekwipunku!")
                    })),
                    false => None,
                }))
                .child_signal(item_not_found_signal(item_id).map(|active| match active {
                    true => Some(html!("br", {})),
                    false => None,
                }))
                .text("LPM aby edytować znacznik przedmiotu")
                .child(html!("br", {}))
                .text("PPM aby usunąć przedmiot z listy znaczników")
            })
        })
    }
}

static HIGHLIHGT_CLASS: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| dominator::class! {
    .dominator::pseudo!("::after", {
        .style_important("z-index", "2")
    })
});

impl OwnItemHandle {
    pub(super) fn init_item_container_dom(&self, own_descriptor: &OwnDescriptor, item_container: &ItemContainer, addon_active: &Mutable<bool>) -> JsResult<()> {
        if let Some(item_highlight) = item_container.item_highlight()? {
            let elem_dom = dom_builder!(item_highlight.clone(), {
                .style_important_signal("opacity", self.disable_signal("0.4"))
                .class(&*HIGHLIHGT_CLASS)
                .style_important_signal("display", own_descriptor.highlight_display_signal(addon_active))
            });
            dominator::replace_dom(item_container.as_ref(), item_highlight.as_ref(), elem_dom);
        }

        let item_canvas = item_container.item_canvas()?;
        let elem_dom = dom_builder!(item_canvas.clone(), {
            .style_important_signal("opacity", self.disable_signal("0.4"))
            .style_important("z-index", "1")
            .style_important_signal("display", own_descriptor.canvas_display_signal(addon_active))
        });
        dominator::replace_dom(item_container.as_ref(), item_canvas.as_ref(), elem_dom);

        #[cfg(feature = "ni")]
        {
            let item_notice = item_container.item_notice()?;
            self.init_dom(item_notice, item_container);
            
            if let Some(last_item_cooldown) = item_container.last_item_cooldown()? {
                self.init_dom(last_item_cooldown, item_container);
            }
        }

        if let Some(item_amount) = item_container.item_amount()? {
            self.init_dom(item_amount, item_container);
        }

        Ok(())
    }

    fn init_dom<B: AsRef<HtmlElement> + Clone + Into<Node>>(&self, elem: B, item_container: &ItemContainer) {
        let elem_dom = dom_builder!(elem.clone(), {
            .style_important_signal("opacity", self.disable_signal("0.4"))
            .style_important("z-index", "2")
        });
        dominator::replace_dom(item_container.as_ref(), elem.as_ref(), elem_dom);
    }

    pub(super) fn init_own_descriptor(
        &self, 
        item_container: &ItemContainer, 
        own_descriptor: &OwnDescriptor, 
        disabled: &Mutable<bool>, 
    ) -> JsResult<()> {
        let addon_data = Addons::get_addon(ADDON_NAME).ok_or_else(|| err_code!())?;
        let addon_active = &addon_data.active;
        
        self.init_item_container_dom(own_descriptor, item_container, addon_active)?;
        item_container.init_animation(self.item_id)?;

        let disable_icon_dom = html!("div", {
            .style_signal("display", addon_active.signal().map(|active| match active {
                true => None,
                false => Some("none")
            }))
            .shadow_root!(web_sys::ShadowRootMode::Closed => {
                .child(get_windows_stylesheet())
                .child_signal(own_descriptor.highlight_signal(disabled.clone()))
                .child_signal(own_descriptor.canvas_signal(disabled.clone()))
                .child_signal(own_descriptor.overlay_signal(disabled.clone(), item_container.clone()))
                .child_signal(own_descriptor.text_signal(disabled.clone()))
                .child_signal(SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal()).map(|selecting| match selecting {
                    false => None,
                    true => Some(html!("div", {
                        .class!(disable-game-item-icon z-i[1] w[100%] h[100%] bg-r[no-repeat] bg-p[center] pos[absolute] l[0] t[0] p-e[all] u-s[all])
                    })),
                }))
            })
        });
        self.shadow_tree_handle.set(Some(dominator::append_dom(
            item_container.as_ref(),
            disable_icon_dom,
        )));

        Ok(())
    }

}
impl ItemContainer {
    fn init_animation(&self, item_id: Id) -> JsResult<()> {
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
        let item_container_dom = apply_methods!(self.clone().into_builder(), {
            //.event(|_: Click| common::debug_log!("Jd"))
            .future(animation_signal(item_id).for_each(clone!(animation, skip_jump => move |active| {
                //debug_log!(@f "active state change: {active:?} {item_id}");
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
                    //debug_log!("STARTING ANIMATION FOR", item_id);

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
        let container_parent = self.parent_node().ok_or_else(|| err_code!())?;
        dominator::replace_dom(&container_parent, self.as_ref(), item_container_dom);


        Ok(())
    }
}

impl ActiveSettings {
    fn render(
        &'static self,
    ) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_right(decors::CloseButton::new())
            .push_right(decors::CollapseButton::new())
            .build();
        let header = WindowHeader::new(decor);

        let content = WindowContent::builder()
            .section(self.default_descriptors.render())
            .section(self.character_descriptors.render_user_aliases())
            .class_list("f-d[column]");

        AddonWindow::builder(AddonName::Znacznik)
            .has_item_slots(true)
            .header(header)
            .content(content)
            .build()
    }
}

fn update_shadow_animations(
    top_shadow_animation: &MutableAnimation,
    bottom_shadow_animation: &MutableAnimation,
    container: &HtmlDivElement,
) {
    let scroll_top = container.scroll_top() as f64;
    let scroll_dist = (container.scroll_height() - container.client_height()) as f64;
    //TODO: Verify it's always offset by 1px.
    let scroll_bottom = scroll_dist - scroll_top;

    match scroll_top < THRESHOLD {
        true => {
            //debug_log!("%from top:", scroll_top / THRESHOLD);
            top_shadow_animation.animate_to(Percentage::new(scroll_top / THRESHOLD));
        }
        false => top_shadow_animation.animate_to(Percentage::END),
    }
    match scroll_bottom < THRESHOLD {
        true => {
            //debug_log!("%from bottom:", scroll_bottom / THRESHOLD);
            bottom_shadow_animation.animate_to(Percentage::new(
                ((scroll_bottom - 1.0) / THRESHOLD).max(0.0),
            ));
        }
        false => bottom_shadow_animation.animate_to(Percentage::END),
    }
}

pub(super) fn init(
    active_settings: &'static ActiveSettings,
) -> JsResult<()> {
    let _active_settings_window_handle = WINDOWS_ROOT
        .try_append_dom(active_settings.render()?)
        .ok_or_else(|| err_code!())?;
    Ok(())
}
