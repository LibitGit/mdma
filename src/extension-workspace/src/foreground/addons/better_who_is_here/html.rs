use std::ops::Deref;

use common::{err_code, map_err};
use dominator::animation::{MutableAnimation, Percentage};
use dominator::events::{ContextMenu, DoubleClick, MouseButton, MouseDown, MouseEnter, MouseLeave, Scroll};
use dominator::{apply_methods, html, with_node, Dom, DomBuilder, EventOptions };
use futures::FutureExt;
use futures_signals::map_ref;
use futures_signals::signal::{self, Mutable, Signal, SignalExt};
use futures_signals::signal_vec::{SignalVec, SignalVecExt};
use itertools::Itertools;
use web_sys::{HtmlDivElement, HtmlImageElement};

use crate::addon_window::{prelude::*, MdmaAddonWindow};
use crate::bindings::engine::iframe_window::PlayerProfileOptions;
use crate::interface::tips_parser::tip;
use crate::color_mark::{Color, ColorMark};
use crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT};
use crate::prelude::*;

use super::{get_emotion_source, ActiveSettings, LevelDisplay, Ordering, Settings, SortBy, ADDON_NAME, SRC};

const TOP_FADE_MAX: f64 = 0.10;
const BOTTOM_FADE_MIN: f64 = 1.0 - TOP_FADE_MAX;
const HIGHLIGHT_COLOR: Color = Color::Lime;

impl Settings {
    fn render(&'static self) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_right(decors::CloseButton::new())
            .build();
        let settings_window_header = WindowHeader::new(decor);
        let settings_window_content = WindowContent::builder()
            .class_list("f-d[column]")
            .heading(Heading::builder().class_list("first-heading").text("Ustawienia ogólne"))
            .section(self.common_settings())
            .heading(Heading::builder().text("Lista graczy"))
            .section(self.player_list_settings());

        SettingsWindow::builder(ADDON_NAME)
            .header(settings_window_header)
            .content(settings_window_content)
            .build()
    }

    fn common_settings(&self) -> ContentSection {
        ContentSection::new()
            .class_list("d[flex] f-d[column]")
            .apply_if(Premium::active(), |b| b.checkbox(
                Checkbox::builder(self.clear_target.clone())
                    .text("Automatyczne przerywanie dobijania")
                    .info_bubble(
                        InfoBubble::builder()
                            .apply(|b| {
                                b.class(["w[240]", "max-w[none]"])
                                    .text("Dobijanie zostanie przerwane, jeśli:")
                                    .child(html!("div", {
                                        .class!(p-top[6] t-a[left])
                                        .text("- zmienimy trasę przed dojściem do celu,")
                                        .text("\n- cel wyjdzie spoza zasięgu ataku.")
                                    }))
                            })
                            .build()
                    )
            ))
            .checkbox(
                Checkbox::builder(self.replace_widget.clone())
                    .text("Otwieraj za pomocą widgetu ")
                    .label_mixin(|b| b.child(html!("span", {
                        .class("c[darkorange]")
                        .text("Gracze na mapie")
                    })))
            )
    }

    fn player_list_settings(&'static self) -> ContentSection {
        ContentSection::new()
            .class_list("d[flex] f-d[column] g[6]")
            .section(self.sort_by_setting())
            .section(self.sort_order_setting())
            .section(self.level_display_setting())
        
    }

    fn level_display_setting(&'static self) -> ContentSection {
        // TODO: Figure out a better solution.
        let wrapper_state: &'static _ = Box::leak(Box::new(Mutable::default()));

        ContentSection::new()
            .class_list("d[flex] f-d[row] g[10] j-c[space-between]")
            .section(ContentSection::new().class_list("a-c[center]").text("Wyświetlanie poziomów"))
            .button(
                Button::builder()
                    .class_list("w[208] t-a[left]")
                    .no_hover()
                    .text_signal(self.level_display_signal())
                    .on_click(|_| wrapper_state.set_neq(true))
                    .on_mousedown(|event| event.stop_propagation())
                    .mixin(|builder| {
                        builder.child(html!("div", {
                            .class!(pos[absolute] r[8] align-center menu-arrow)
                        }))
                    })
                    .scroll_wrapper(
                        ScrollWrapper::builder(|| {
                                || {
                                    wrapper_state.set_neq(false);
                                }
                            })
                            .class_list("w[200] l[1]")
                            .visible_signal(wrapper_state.signal())
                            .option(self.level_display_option(LevelDisplay::First))
                            .option(self.level_display_option(LevelDisplay::Last))
                            .option(self.level_display_option(LevelDisplay::None))
                            .option(self.level_display_option(LevelDisplay::Only))
                            .build()
                    ),
            )
    }

    fn level_display_option(&'static self, level_display: LevelDisplay) -> ScrollWrapperOption {
        ScrollWrapperOption::builder()
            .text(level_display.as_str())
            .on_click(move |_| self.level_display.set_neq(level_display))
            .build()
    }

    fn sort_by_setting(&'static self) -> ContentSection {
        // TODO: Figure out a better solution.
        let wrapper_state: &'static _ = Box::leak(Box::new(Mutable::default()));

        ContentSection::new()
            .class_list("d[flex] f-d[row] j-c[space-between]")
            .section(ContentSection::new().class_list("a-c[center]").text("Sortowanie względem"))
            .button(
                Button::builder()
                    .class_list("w[98] t-a[left]")
                    .no_hover()
                    .text_signal(self.sort_by_signal())
                    .on_click(|_| wrapper_state.set_neq(true))
                    .on_mousedown(|event| event.stop_propagation())
                    .mixin(|builder| {
                        builder.child(html!("div", {
                            .class!(pos[absolute] r[8] align-center menu-arrow)
                        }))
                    })
                    .scroll_wrapper(
                        ScrollWrapper::builder(|| {
                                || {
                                    wrapper_state.set_neq(false);
                                }
                            })
                            .class_list("w[90] l[1]")
                            .visible_signal(wrapper_state.signal())
                            .option(self.sort_by_option(SortBy::Lvl))
                            .option(self.sort_by_option(SortBy::Nick))
                            .option(self.sort_by_option(SortBy::Prof))
                            .build()
                    ),
            )
    }

    fn sort_by_option(&'static self, sort_by: SortBy) -> ScrollWrapperOption {
        ScrollWrapperOption::builder()
            .text(sort_by.as_str())
            .on_click(move |_| self.sort_by.set_neq(sort_by))
            .build()
    }

    fn sort_order_setting(&'static self) -> ContentSection {
        // TODO: Figure out a better solution.
        let wrapper_state: &'static _ = Box::leak(Box::new(Mutable::default()));

        ContentSection::new()
            .class_list("d[flex] f-d[row] j-c[space-between]")
            .section(ContentSection::new().class_list("a-c[center]").text("Kolejność"))
            .button(
                Button::builder()
                    .class_list("w[98] t-a[left]")
                    .no_hover()
                    .text_signal(self.sort_ordering_signal())
                    .on_click(|_| wrapper_state.set_neq(true))
                    .on_mousedown(|event| event.stop_propagation())
                    .mixin(move |builder| {
                        builder
                            .child(html!("div", {
                                .class!(pos[absolute] r[8] align-center menu-arrow)
                            }))
                    })
                    .scroll_wrapper(
                    ScrollWrapper::builder(|| {
                                || {
                                    wrapper_state.set_neq(false);
                                }
                            })
                            .class_list("w[90] l[1]")
                            .visible_signal(wrapper_state.signal())
                            .option(self.sort_ordering_option(Ordering::Descending))
                            .option(self.sort_ordering_option(Ordering::Ascending))
                            .build()
                    ),
            )
    }

    fn sort_ordering_option(&'static self, ordering: Ordering) -> ScrollWrapperOption {
        ScrollWrapperOption::builder()
            .text(ordering.as_str())
            .on_click(move |_| self.ordering.set_neq(ordering))
            .build()
    }

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
            .push_right(decors::SizeToggle::new(2))
            .push_right(decors::CollapseButton::new())
            .push_right(Self::player_counter_bubble())
            .build();
        let addon_window_header = WindowHeader::new(decor);

        let top_shadow_animation: &'static _ = Box::leak(Box::new(MutableAnimation::new(0.0)));
        let bottom_shadow_animation: &'static _ = Box::leak(Box::new(
            MutableAnimation::new_with_initial(0.0, Percentage::END),
        ));
        let window_size = Addons::get_window_size(ADDON_NAME, Self::WINDOW_TYPE).ok_or_else(|| err_code!())?;
        let addon_window_content = WindowContent::builder()
            .class_list("p[1-4-4-4] f-d[column]")
            .section_signal(self.scroll_target_signal().map(move |visible| {
                visible.map(|other_id| self.build_scroll_wrapper(other_id, settings))
            }))
            .section_signal(self.target_cell_signal(Premium::active(), settings))
            .section_signal(self.target_line_signal())
            .section(
                ContentSection::new()
                    //.class_list("player-list max-h[265] w[100%] scroll-y fade-top-bottom")
                    .class_list("player-list w[100%] scroll-y fade-top-bottom")
                    .dynamic_class_signal(window_size.signal().map(Self::current_max_height_px))
                    .section(
                        ContentSection::new()
                            .class_list("l-h[24] t-a[center]")
                            .visible_signal(Others::get().signal_vec_keys().is_empty().dedupe())
                            .text("----"),
                    )
                    .mixin(|b| {
                        let sorted_signal = settings
                            .sort_signal()
                            .switch_signal_vec(move |_| {
                                self.sorted_list_signal_vec(settings)
                            });

                        b.children_signal_vec(sorted_signal)
                    })
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
                    .event_with_elem(move |_event: Scroll, container: &HtmlDivElement| {
                        let current_max_height = Self::current_max_height(window_size.get());
                        let threshold = current_max_height * TOP_FADE_MAX;
                        Self::update_shadow_animations(threshold, top_shadow_animation, bottom_shadow_animation, container)
                    })
                    .mixin(|b| {
                        b.after_inserted(move |container| {
                            let element = container.clone();
                            let future = dominator_helpers::DomRectSignal::new(&element)
                                .switch(move |_| {
                                    window_size.signal_ref(move |size| Self::current_max_height(*size) * TOP_FADE_MAX)
                                })
                                .for_each(move |threshold| {
                                    Self::update_shadow_animations(threshold, top_shadow_animation, bottom_shadow_animation, &container);

                                    async {}
                                });
                            wasm_bindgen_futures::spawn_local(future);
                        })
                    }),
            )
            .input(
                Input::builder()
                    .class_list("b-s[none] b[none] bg[none] cursor[auto]")
                    .size(InputSize::Big)
                    .placeholder("Szukaj gracza...")
                    .text_align(TextAlign::Left)
                    .on_input(|_, input_elem| {
                        let mut value = input_elem.value();
                        value.make_ascii_lowercase();
                        self.search_text.set(value);
                    })
            );

        AddonWindow::builder(ADDON_NAME)
            .header(addon_window_header)
            .content(addon_window_content)
            .build()
    }

    fn current_max_height(size: u8) -> f64 {
        match size {
            0 => 265.0,
            _ => window().inner_height().unwrap_js().unchecked_into_f64() / 2.0
        }
    }

    fn current_max_height_px(size: u8) -> &'static str {
        match size {
            0 => "max-h[265]",
            _ => "max-h[50vh]",
        }
    }

    fn prepare_list_signal_vec(&'static self) -> impl SignalVec<Item = Other> + 'static {
        Others::get().signal_vec_keys()
            .filter_signal_cloned(|id| signal::not(self.is_target_signal(*id)))
            .filter_map(|other_id| {
                Others::get().lock_ref().get(&other_id).cloned()
            })
            .filter_signal_cloned(|other_data| {
                other_data.nick
                    .signal_cloned()
                    .switch(|mut nick| {
                        nick.make_ascii_lowercase();
                        self.search_text.signal_ref(move |search_text| nick.contains(search_text))
                    })
            })
    }

    fn sorted_list_signal_vec(&'static self, settings: &'static Settings) -> impl SignalVec<Item = Dom> + 'static {
        self.prepare_list_signal_vec()
            .to_signal_cloned()
            .map(|mut entries| {
                entries.sort_by(|a,b| settings.compare(a, b));
                entries
            })
            .to_signal_vec()
            .map(move |other_data| {
                build_one_other(self, other_data.char_id, other_data, settings)
            })
    }

    fn player_counter_bubble() -> decors::CounterBubble {
        decors::CounterBubble::builder()
            .counter_signal(Others::get().signal_vec_keys().len())
            .mixin(|b| {
                apply_methods!(b, {
                    .tip!({
                        .class!(d[flex] f-d[column] g[2])
                        .child(player_count_tip_text("Zwykli gracze", Relation::None))
                        .child(player_count_tip_text("Przyjaciele", Relation::Friend))
                        .child(player_count_tip_text("Wrogowie", Relation::Enemy))
                        .child(player_count_tip_text("Klanowicze", Relation::Clan))
                        .child(player_count_tip_text("Sojusznicy klanowi", Relation::ClanAlly))
                        .child(player_count_tip_text("Wrogowie klanowi", Relation::ClanEnemy))
                        .apply_if(WorldConfig::has_fractions(), |builder| {
                            builder
                                .child(player_count_tip_text("Sojusznicy frakcji", Relation::FractionAlly))
                                .child(player_count_tip_text("Wrogowie frakcji", Relation::FractionEnemy))
                        })
                        .child(html!("div", {
                            .class!(d[flex] f-d[row] j-c[space-between] group)
                            .child(html!("span", { .text("Grupowicze") }))
                            .child(html!("span", { 
                                .text_signal(Others::get().entries_cloned().to_signal_cloned().map(|entries| {
                                    let party_lock = Party::get().lock_ref();
                                    // Count of players in the same location that are also in
                                    // hero's party
                                    let party_count = entries.into_iter().filter(|(other_id, _)| party_lock.contains_key(other_id)).count();

                                    format!("{party_count}")
                                })) 
                            }))
                        }))
                    })
                })
            })
            .build()
    }

    fn target_line_signal(&self) -> impl Signal<Item = Option<ContentSection>> {
        self.target
            .active_signal()
            .dedupe_map(|active| {
                active.then(|| ContentSection::new().class_list("line m-top[6] m-bottom[6]"))
            })
    }

    fn target_cell_signal(&'static self, has_premium: bool, settings: &'static Settings) -> impl Signal<Item = Option<ContentSection>> {
        self.target.signal_cloned().dedupe_map(move |target_opt| {
            let target = target_opt.clone()?;
            let nick_mutable = target.nick.clone();
            let lvl_mutable = target.lvl.clone();
            let oplvl_mutable = target.operational_lvl.clone();
            let prof_mutable = target.prof.clone();
            let nick_cell = nick_mutable.signal_ref(|nick| {
                Some(ContentSection::new().class_list("other-nick").text(nick.as_str()))
            });

            let clan = target.clan.clone();
            let other_cell = ContentSection::new()
                .class_list("one-other target")
                .mixin(|b| {
                    apply_methods!(b, {
                        .dynamic_class_signal(target.relation.signal().map(|relation| relation.to_str()))
                        .tip!({
                            .child(html!("div", {
                                .class("nick")
                                .text_signal(map_ref!{
                                    let level_display = settings.level_display.signal(),
                                    let lvl = lvl_mutable.signal(),
                                    let oplvl = oplvl_mutable.signal(),
                                    let prof = prof_mutable.signal() => {
                                        match level_display {
                                            _ if *lvl <= 300 => format!("({}{})", *lvl, *prof),
                                            LevelDisplay::First => format!("({}{}|{}{})", *lvl, *prof, *oplvl, *prof),
                                            LevelDisplay::Last => format!("({}{}|{}{})", *oplvl, *prof, *lvl, *prof),
                                            LevelDisplay::None => format!("({}{})", *oplvl, *prof),
                                            LevelDisplay::Only => format!("({}{})",*lvl, *prof),
                                        }
                                    }
                                })
                            }))
                            .child_signal(clan.signal_ref(|clan_option| clan_option.as_ref().map(|clan| {
                                html!("div", {
                                    .class("clan-in-tip")
                                    .text(clan.name.as_str())
                                })
                            })))
                            .child(html!("div", {
                                .style("margin-top", "3px")
                                .text(match has_premium {
                                    true => "PPM aby przerwać dobijanie gracza.",
                                    false => "PPM aby przerwać atak na gracza.",
                                })
                            }))
                        })
                    })
                })
                .section_signal(nick_cell)
                .section(
                    ContentSection::new()
                        .class_list("other-emotions-wrapper")
                        .mixin(|b| b.children_signal_vec(
                            target.emo
                                .signal_vec()
                                .map_signal(|emotion| signal::always(emotion).map_future(|emotion| {
                                    get_emotion_source(emotion)
                                        .map(|emotion_img_src_res| emotion_img_src_res.and_then(|emotion_img_src| {
                                            let img = HtmlImageElement::new()?;
                                            SRC.with(|src| js_sys::Reflect::set(&img, src, &emotion_img_src).map_err(map_err!()))?;

                                            Ok(html!("div", {
                                                .class("other-emotion-wrapper")
                                                .child(DomBuilder::new(img).class("other-emotion").into_dom())
                                            }))
                                        }))
                                }))
                                .filter_map(move |emo_img_opt| {
                                    let opt = emo_img_opt.transpose();
                                    match opt {
                                        Ok(data) => data,
                                        Err(err_code) => {
                                            console_error!(err_code);
                                            None
                                        }
                                    }
                                })
                        ))
                )
                .section(
                    ContentSection::new()
                        .class_list("other-lvl")
                        .text_signal(map_ref!{
                            let level_display = settings.level_display.signal(),
                            let lvl = target.lvl.signal(),
                            let oplvl = target.operational_lvl.signal(),
                            let prof = target.prof.signal() => {
                                match level_display {
                                    _ if *lvl <= 300 => format!("({}{})", *lvl, *prof),
                                    LevelDisplay::First => format!("({}{}|{}{})", *lvl, *prof, *oplvl, *prof),
                                    LevelDisplay::Last => format!("({}{}|{}{})", *oplvl, *prof, *lvl, *prof),
                                    LevelDisplay::None => format!("({}{})", *oplvl, *prof),
                                    LevelDisplay::Only => format!("({}{})",*lvl, *prof),
                                }
                            }
                        }),
                )
                .event(move |_: MouseEnter| {
                    #[cfg(feature = "ni")]
                    {
                        let other = get_engine().others().unwrap_js().get_by_id(target.char_id).unwrap_js();
                        get_engine()
                            .targets()
                            .unwrap_js()
                            .add_arrow(false, &other.d().nick(), &other, "Other", "attack")
                            .unwrap_js();
                    }
                })
                .event(move |event: ContextMenu| {
                    if event.button() == MouseButton::Right {
                        self.clear_target();
                    }
                });

            Some(other_cell)
        })
    }

    fn build_scroll_wrapper(&'static self, other_id: Id, settings: &'static Settings) -> ScrollWrapper {
        ScrollWrapper::builder(|| {
            || {
                let Some(old_scroll_target_id) = self.scroll_visible.replace(None) else {
                    return;
                };

                //common::debug_log!("old_targget", old_scroll_target_id);
                #[cfg(feature = "ni")] 
                if !self.is_target(old_scroll_target_id) {
                    get_engine()
                        .targets()
                        .unwrap_js()
                        .delete_arrow(&format!("Other-{old_scroll_target_id}"))
                        .unwrap_js();
                }

                ColorMark::remove(HIGHLIGHT_COLOR, ADDON_NAME, old_scroll_target_id);
                //globals.others.remove_color_mark(old_target);
            }
        })
            .class_list("w[130] t-a[center] l[100%+4] align-center")
            .option_if(!Premium::active(), || ScrollWrapperOption::builder()
                .text("Atakuj")
                .on_click(move |_| {
                    self.target.set(Others::get().lock_ref().get(&other_id).unwrap_js().clone(), ADDON_NAME);
                })
                .build()
            )
            .option_if(Premium::active(), || ScrollWrapperOption::builder()
                .text("Dobijaj")
                .on_click(move |_| {
                    self.target.set(Others::get().lock_ref().get(&other_id).unwrap_js().clone(), ADDON_NAME);
                    match cfg!(feature = "antyduch") {
                        true => settings.clear_target.set_neq(false), // never clear target automatically
                        false => wasm_bindgen_futures::spawn_local(async move {
                            let success = get_engine().hero().unwrap_js().auto_go_to_other(other_id).await.unwrap_js();
                            if !self.is_target(other_id) {
                                return;
                            }
                            if !success && settings.clear_target.get() { 
                                self.clear_target();
                                return;
                            }
                            self.after_follow.set(true);
                        }),
                    };
                })
                .build()
            )
            .option(
                ScrollWrapperOption::builder()
                    .text("Handluj")
                    .on_click(move |_| {
                        wasm_bindgen_futures::spawn_local(async move {
                            let success = get_engine().hero().unwrap_js().auto_go_to_other(other_id).await.unwrap_js();
                            if !success {
                                return;
                            }
                            let can_trade = Others::get()
                                .lock_ref()
                                .get(&other_id)
                                .map(|other_data| Hero::is_non_peer_in_invite_range(other_data))
                                .unwrap_or(false);

                            if can_trade {
                                send_task(&format!("trade&a=ask&id={}", other_id)).unwrap_js();
                            }
                        });
                    })
                    .build()
            )
            .option_if(Hero::get().lvl.get() > 29,
                || ScrollWrapperOption::builder()
                    .text("Pocałuj")
                    .on_click(move |_| {
                        wasm_bindgen_futures::spawn_local(async move {
                            let success = get_engine().hero().unwrap_js().auto_go_to_other(other_id).await.unwrap_js();
                            if !success {
                                return;
                            }
                            let can_kiss = Others::get()
                                .lock_ref()
                                .get(&other_id)
                                .map(|other_data| Hero::is_non_peer_in_invite_range(other_data))
                                .unwrap_or(false);

                            if can_kiss {
                                send_task(&format!("emo&a=kiss&id={}", other_id)).unwrap_js();
                            }
                        });
                    })
                    .build()
            )
            .option_if(Hero::get().vip.get(),
                || ScrollWrapperOption::builder()
                    .text("Karmazynowe błogosławieństwo")
                    .on_click(move |_| {
                        wasm_bindgen_futures::spawn_local(async move {
                            let success = get_engine().hero().unwrap_js().auto_go_to_other(other_id).await.unwrap_js();
                            if !success {
                                return;
                            }
                            let can_bless = Others::get()
                                .lock_ref()
                                .get(&other_id)
                                .map(|other_data| Hero::is_non_peer_in_invite_range(other_data))
                                .unwrap_or(false);

                            if can_bless {
                                send_task(&format!("emo&a=bless&id={}", other_id)).unwrap_js();
                            }
                        });
                    })
                    .build()
            )
            .option_if(
                Others::get().lock_ref()
                    .get(&other_id)
                    .is_some_and(|other_data| other_data.relation.get() != Relation::Enemy),
                || ScrollWrapperOption::builder()
                    .text("Wyślij wiadomość")
                    .on_click(move |_| {
                        get_engine()
                            .chat_controller()
                            .unwrap_js()
                            .get_chat_input_wrapper()
                            .unwrap_js()
                            .set_private_message_procedure(
                                Others::get()
                                    .lock_ref()
                                    .get(&other_id)
                                    .unwrap_js()
                                    .nick
                                    .lock_ref()
                                    .deref()
                            )
                            .unwrap_js();
                    })
                    .build()
            )
            .option(
                ScrollWrapperOption::builder()
                    .text("Pokaż ekwipunek")
                    .on_click(move |_| {
                        get_engine().others().unwrap_js().get_by_id(other_id).unwrap_js().show_eq().unwrap_js();
                    })
                    .build()
            )
            .option(
                ScrollWrapperOption::builder()
                    .text("Zaproś do przyjaciół")
                    .on_click(move |_| {
                        send_task(&format!(
                            "friends&a=finvite&nick={}",
                                Others::get()
                                    .lock_ref()
                                    .get(&other_id)
                                    .unwrap_js()
                                    .nick
                                    .lock_ref()
                                    .deref()
                                    .trim()
                                    .split_ascii_whitespace()
                                    .join("_")
                        )).unwrap_js()
                    })
                    .build()
            )
            .option(
                ScrollWrapperOption::builder()
                    .text("Zaproś do drużyny")
                    .on_click(move |_| {
                        send_task(&format!("party&a=inv&id={other_id}")).unwrap_js();
                    })
                    .build()
            )
            .option(
                ScrollWrapperOption::builder()
                    .text("Pokaż profil")
                    .on_click(move |_| {
                        let others_lock = Others::get().lock_ref();
                        let other_data = others_lock.get(&other_id).unwrap_js();
                        let options = PlayerProfileOptions::new(other_data.account, other_data.char_id);

                        get_engine()
                            .inline_frame_window_manager()
                            .unwrap_js()
                            .new_player_profile(&options)
                            .unwrap_js();
                    })
                    .build()
            )
            .build()
    }

    fn update_shadow_animations(
        threshold: f64,
        top_shadow_animation: &MutableAnimation,
        bottom_shadow_animation: &MutableAnimation,
        container: &HtmlDivElement,
    ) {
        let scroll_top = container.scroll_top() as f64;
        let scroll_dist = (container.scroll_height() - container.client_height()) as f64;
        //TODO: Verify it's always offset by 1px.
        //debug_log!(scroll_dist, scroll_top);
        let scroll_bottom = scroll_dist - scroll_top;
    
        match scroll_top < threshold {
            true => {
                //debug_log!("%from top:", scroll_top / threshold);
                top_shadow_animation.animate_to(Percentage::new(scroll_top / threshold));
            }
            false => top_shadow_animation.animate_to(Percentage::END),
        }
        match scroll_bottom < threshold {
            true => {
                //debug_log!("%from bottom:", ((scroll_bottom - 1.0) / threshold).max(0.0));
                bottom_shadow_animation.animate_to(Percentage::new(
                    ((scroll_bottom - 1.0) / threshold).max(0.0),
                ));
            }
            false => bottom_shadow_animation.animate_to(Percentage::END),
        }
    }
}

fn related_players_count_signal(
    relation: Relation,
) -> impl Signal<Item = String> {
    Others::get()
        .entries_cloned()
        .to_signal_cloned()
        .map(move |entries| {
            entries
                .into_iter()
                .filter(|(_, other_data)| {
                    other_data.relation.get() == relation
                })
                .count()
                .to_string()
        })
}

fn player_count_tip_text(
    description: &'static str,
    relation: Relation,
) -> Dom {
    html!("div", {
        .class!(d[flex] f-d[row] j-c[space-between])
        .class(relation.to_str())
        .child(html!("span", { .text(description) }))
        .child(html!("span", { .text_signal(related_players_count_signal(relation)) }))
    })
}

fn build_one_other(
    state: &'static ActiveSettings,
    other_id: OtherId,
    other_data: Other,
    settings: &'static Settings,
) -> Dom {
    let is_wanted = other_data.is_wanted();
    let nick_mutable = other_data.nick;
    let lvl_mutable = other_data.lvl.clone();
    let oplvl_mutable = other_data.operational_lvl.clone();
    let prof_mutable = other_data.prof.clone();
    let nick_cell = nick_mutable.signal_ref(|nick| {
        Some(html!("div", {
            .class("other-nick")
            .text(&nick)
        }))
    });

    html!("div", {
        .class("one-other")
        .class("a-i[center]")
        //.class(.unwrap_or(Relation::None).to_str())
        // TODO: Instead of `signal_vec_keys` create a signal that responds with the last added id ?
        .dynamic_class_signal(
            other_data.relation
                .signal()
                .switch(move |relation| {
                    Party::get()
                        .signal_vec_keys()
                        .to_signal_cloned()
                        .map(move |party_members| {
                            party_members.contains(&other_id).then_some("group").unwrap_or_else(|| relation.to_str())
                        })
                })
        )
        .class_signal("selected", state.is_scroll_target_signal(other_id))
        .attr("data-id", &other_id.to_string())
        .tip!({
            .class("max-w[250]")
            .apply_if(is_wanted, |b| b.child(html!("div", {
                .class("wanted")
            })))
            .child(html!("div", {
                .class("nick")
                .text_signal(map_ref!{
                    let level_display = settings.level_display.signal(),
                    let nick = nick_mutable.signal_cloned(),
                    let lvl = lvl_mutable.signal(),
                    let oplvl = oplvl_mutable.signal(),
                    let prof = prof_mutable.signal() => {
                        match level_display {
                            _ if *lvl <= 300 => format!("{nick} ({}{})", *lvl, *prof),
                            LevelDisplay::First => format!("{nick} ({}{}|{}{})", *lvl, *prof, *oplvl, *prof),
                            LevelDisplay::Last => format!("{nick} ({}{}|{}{})", *oplvl, *prof, *lvl, *prof),
                            LevelDisplay::None => format!("{nick} ({}{})", *oplvl, *prof),
                            LevelDisplay::Only => format!("{nick} ({}{})",*lvl, *prof),
                        }
                    }
                })
            }))
            .child_signal(other_data.clan.signal_ref(|clan_option| clan_option.as_ref().map(|clan| {
                html!("div", {
                    .class("clan-in-tip")
                    .text(clan.name.as_str())
                })
            })))
        })
        .child_signal(nick_cell)
        .child(html!("div", {
            .class("other-emotions-wrapper")
            .children_signal_vec(
                other_data.emo
                    .signal_vec()
                    .map_signal(|emotion| signal::always(emotion).map_future(|emotion| {
                        get_emotion_source(emotion)
                            .map(|emotion_img_src_res| emotion_img_src_res.and_then(|emotion_img_src| {
                                let img = HtmlImageElement::new()?;
                                SRC.with(|src| js_sys::Reflect::set(&img, src, &emotion_img_src).map_err(map_err!()))?;

                                Ok(html!("div", {
                                    .class("other-emotion-wrapper")
                                    .child(DomBuilder::new(img).class("other-emotion").into_dom())
                                }))
                            }))
                    }))
                    .filter_map(move |emo_img_opt| {
                        let opt = emo_img_opt.transpose();
                        match opt {
                            Ok(data) => data,
                            Err(err_code) => {
                                console_error!(err_code);
                                None
                            }
                        }
                    })
            )
        }))
        .apply_if(is_wanted, |b| b.child(html!("div", {
            .class("skull")
        })))
        .child_signal(map_ref!{
            let level_display = settings.level_display.signal(),
            let lvl = other_data.lvl.signal(),
            let oplvl = other_data.operational_lvl.signal(),
            let prof = other_data.prof.signal() => {
                let txt = match level_display {
                    _ if *lvl <= 300 => format!("({}{})", *lvl, *prof),
                    LevelDisplay::First => format!("({}{}|{}{})", *lvl, *prof, *oplvl, *prof),
                    LevelDisplay::Last => format!("({}{}|{}{})", *oplvl, *prof, *lvl, *prof),
                    LevelDisplay::None => format!("({}{})", *oplvl, *prof),
                    LevelDisplay::Only => format!("({}{})",*lvl, *prof),
                };

                Some(html!("div", {
                    .class("other-lvl")
                    .text(&txt)
                }))
            }
        })
        .event(move |event: DoubleClick| {
            if event.button() != MouseButton::Left {
                return;
            }

            wasm_bindgen_futures::spawn_local(async move {
                get_engine().hero().unwrap_js().auto_go_to_other(other_id).await.unwrap_js();
            });
        })
        .event(move |event: ContextMenu| {
            if event.button() != MouseButton::Right {
                return;
            }

            state.scroll_visible.set(Some(other_id));
        })
        .event_with_options(&EventOptions::preventable(), move |_: MouseDown| {
            //if !globals.others.has_color_mark(other_id) {
            //common::debug_log!("HAS COLOR MARK:", ColorMark::has_mark(HIGHLIGHT_COLOR, ADDON_NAME, &other_id));
            if ColorMark::has_mark(HIGHLIGHT_COLOR, ADDON_NAME, &other_id) && !state.is_scroll_target(other_id) {
                return
            }

            #[cfg(feature = "ni")] 
            {
                let other = get_engine().others().unwrap_js().get_by_id(other_id).unwrap_js();
                //debug_log!("other:", &other);
                get_engine().targets().unwrap_js().add_arrow(false, &other.d().nick(), &other, "Other", "navigate").unwrap_js();
            }
            
            if let Err(err_code) = Others::init_color_mark(HIGHLIGHT_COLOR, ADDON_NAME, other_id) {
                console_error!(err_code);
            }
        })
        .event(move |_: MouseEnter| {
            if state.is_scroll_active() {
                return;
            }

            #[cfg(feature = "ni")] 
            {
                let other = get_engine().others().unwrap_js().get_by_id(other_id).unwrap_js();
                get_engine().targets().unwrap_js().add_arrow(false, &other.d().nick(), &other, "Other", "navigate").unwrap_js();
            }
            
            if let Err(err_code) = Others::init_color_mark(HIGHLIGHT_COLOR, ADDON_NAME, other_id) {
                console_error!(err_code);
            }
        })
        .event(move |_: MouseLeave| {
            if state.is_scroll_target(other_id) {
                return;
            }

            #[cfg(feature = "ni")] 
            {
                get_engine().targets().unwrap_js().delete_arrow(&format!("Other-{other_id}")).unwrap_js();
            }
            ColorMark::remove(HIGHLIGHT_COLOR, ADDON_NAME, other_id);
            //globals.others.remove_color_mark(other_id);
        })
        .after_inserted(move |_| {
            if !state.is_scroll_target(other_id) {
                return
            }

            common::debug_log!("ADDING MARK TO SCROLL TARGET");
            #[cfg(feature = "ni")] 
            {
                let other = get_engine().others().unwrap_js().get_by_id(other_id).unwrap_js();
                get_engine().targets().unwrap_js().add_arrow(false, &other.d().nick(), &other, "Other", "navigate").unwrap_js();
            }
            
            if let Err(err_code) = Others::init_color_mark(HIGHLIGHT_COLOR, ADDON_NAME, other_id) {
                console_error!(err_code);
            }
        })
        // TODO: Any other solutions not involving after_removed ?
        .after_removed(move |_| {
            if state.is_target(other_id) {
                //common::debug_log!("is target...");
                return;
            }
            if !ColorMark::has_mark(HIGHLIGHT_COLOR, ADDON_NAME, &other_id) {
                return;
            }

            #[cfg(feature = "ni")] 
            {
                get_engine().targets().unwrap_js().delete_arrow(&format!("Other-{other_id}")).unwrap_js();
            }
            ColorMark::remove(HIGHLIGHT_COLOR, ADDON_NAME, other_id);
        })
    })
}

pub(super) fn init(
    settings_window: &'static Settings,
    addon_window: &'static ActiveSettings,
) -> JsResult<()> {
    let _settings_window_handle = WINDOWS_ROOT
        .try_append_dom(settings_window.render()?)
        .ok_or_else(|| err_code!())?;
    let _addon_window_handle = WINDOWS_ROOT
        .try_append_dom(addon_window.render(
            settings_window,
        )?)
        .ok_or_else(|| err_code!())?;

    Ok(())
}
