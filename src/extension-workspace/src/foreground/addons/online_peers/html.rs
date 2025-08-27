use std::{cell::Cell, future::Future, ops::Deref, pin::Pin};

use common::{closure, err_code};
use dominator::{
    animation::{MutableAnimation, Percentage},
    apply_methods, clone,
    events::{
        Click, ContextMenu, DoubleClick, MouseButton, MouseDown, MouseEnter, MouseLeave, Scroll,
        Wheel,
    },
    html, with_node, Dom, EventOptions,
};
use futures_signals::{
    map_ref,
    signal::{ Mutable, SignalExt},
    signal_vec::SignalVecExt,
};
use itertools::Itertools;
use js_sys::Function;
use web_sys::HtmlDivElement;

use crate::{
    addon_window::prelude::*,
    color_mark::{Color, ColorMark},
    bindings::engine::{iframe_window::PlayerProfileOptions, show_eq::ShowEqPlayerData},
    interface::{tips_parser::tip, ThreadLocalShadowRoot, WINDOWS_ROOT},
};
use crate::{addons::better_who_is_here::LevelDisplay, prelude::*};

use super::{ActiveSettings, DisplayTab, Ordering, Settings, SortBy, ADDON_NAME};

const TOP_FADE_MAX: f64 = 0.10;
const BOTTOM_FADE_MIN: f64 = 1.0 - TOP_FADE_MAX;
const MAX_OVERSCROLL: f64 = 50.0;
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
            .heading(
                Heading::builder()
                    .text("Lista rówieśników")
                    .class_list("first-heading"),
            )
            .section(self.player_list_settings());

        SettingsWindow::builder(ADDON_NAME)
            .header(settings_window_header)
            .content(settings_window_content)
            .build()
    }

    fn player_list_settings(&'static self) -> ContentSection {
        ContentSection::new()
            .class_list("d[flex] f-d[column] g[6]")
            .checkbox(
                Checkbox::builder(self.always_show_tip.clone())
                    .text("Zawsze wyświetlaj tip")
                    .info_bubble(
                        InfoBubble::builder()
                            .text("Gdy ta opcja jest wyłączona tip rówieśnika wyświetli się po najechaniu na jego komórkę myszką, tylko jeżeli jego opis się w niej nie mieści.")
                            .build()
                    )
            )
            .checkbox(Checkbox::builder(self.show_location.clone()).text("Wyświetlaj lokację rówieśnika"))
            .checkbox(
                Checkbox::builder(self.show_alias.clone())
                    .class_list("w-s[pre-line] l-h[16]")
                    .text("Wyświetlaj informację o kolosie\nlub tytanie w lokacji rówieśnika")
                    .info_bubble(
                        InfoBubble::builder()
                            .text("Jeśli rówieśnik znajduje się w przedsionku lub lokacji z kolosem/tytanem, wyświetli się o tym informacja w jego komórce. Np. K-114 lub T-285")
                            .build()
                    )
            )
            .section(self.sort_by_setting())
            .section(self.sort_order_setting())
            .section(self.level_display_setting())
    }

    fn level_display_setting(&'static self) -> ContentSection {
        // TODO: Figure out a better solution.
        let wrapper_state: &'static _ = Box::leak(Box::new(Mutable::default()));

        ContentSection::new()
            .class_list("d[flex] f-d[row] g[10] j-c[space-between]")
            .section(
                ContentSection::new()
                    .class_list("a-c[center]")
                    .text("Wyświetlanie poziomów"),
            )
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
                        .build(),
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
        let wrapper_state: &'static _ = Box::leak(Box::new(Mutable::default()));

        ContentSection::new()
            .class_list("d[flex] f-d[row] j-c[space-between] g[10]")
            .section(
                ContentSection::new()
                    .class_list("a-c[center]")
                    .text("Sortowanie względem"),
            )
            .button(
                Button::builder()
                    .class_list("w[98] t-a[left]")
                    .no_hover()
                    .text_signal(self.sort_by_signal())
                    .on_click(|_| wrapper_state.set_neq(true))
                    .on_mousedown(|event| event.stop_propagation())
                    .mixin(move |builder| {
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
                        .build(),
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
        let wrapper_state: &'static _ = Box::leak(Box::new(Mutable::default()));

        ContentSection::new()
            .class_list("d[flex] f-d[row] j-c[space-between]")
            .section(
                ContentSection::new()
                    .class_list("a-c[center]")
                    .text("Kolejność"),
            )
            .button(
                Button::builder()
                    .class_list("w[98] t-a[left]")
                    .no_hover()
                    .text_signal(self.sort_ordering_signal())
                    .on_click(|_| wrapper_state.set_neq(true))
                    .on_mousedown(|event| event.stop_propagation())
                    .mixin(move |builder| {
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
                        .option(self.sort_ordering_option(Ordering::Descending))
                        .option(self.sort_ordering_option(Ordering::Ascending))
                        .build(),
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
            .push_right(decors::SizeToggle::new(3))
            .push_right(decors::CollapseButton::new())
            .push_right(decors::CounterBubble::builder().class_list("p-right[5]").mixin(|b| {
                apply_methods!(b, {
                    .text_signal(Peers::get().online_len_signal().map(|online_count| format!("({online_count})")))
                    .tip!({
                        .text_signal(Peers::get().online_entries_signal().map(|entries| {
                            let clan_members_count = entries
                                .iter()
                                .filter(|(_, peer_data)| {
                                    peer_data
                                        .relation.get()
                                         == Relation::Clan
                                })
                                .count();
                            let friends_count = entries
                                .iter()
                                .filter(|(_, peer_data)| {
                                    peer_data
                                        .relation.get()
                                         == Relation::Friend
                                })
                                .count();
                            format!(
                                "Klanowicze online: {}\n\n Przyjaciele online: {} / 50",
                                clan_members_count, friends_count
                            )
                        }).dedupe_cloned())
                    })
                })
            }).build())
            .build();
        let header = WindowHeader::new(decor);

        #[cfg(not(debug_assertions))]
        let scroll_wrapper_signal = self
            .scroll_target_signal()
            .map(move |visible| visible.map(|peer_id| self.build_scroll_wrapper(peer_id)));
        #[cfg(debug_assertions)]
        let scroll_wrapper_signal = self
            .scroll_target_signal()
            .map(move |visible| visible.map(|peer_id| self.build_scroll_wrapper(peer_id)));
        let content = WindowContent::builder()
            .class_list("f-d[column] p[1-4-4-4]")
            .section_signal(scroll_wrapper_signal)
            .section(
                ContentSection::new()
                    .class_list("d[flex] f-d[row] m[0--2]")
                    .section(
                        ContentSection::new()
                            .class_list("card-header")
                            .text("Klanowicze")
                            .class_signal(
                                "active",
                                self.current_tab
                                    .signal_ref(|tab| *tab == DisplayTab::ClanMembers),
                            )
                            .event(|_: Click| {
                                self.current_tab.set(!self.current_tab.get());
                            }),
                    )
                    .section(
                        ContentSection::new()
                            .class_list("card-header")
                            .text("Przyjaciele")
                            .class_signal(
                                "active",
                                self.current_tab
                                    .signal_ref(|tab| *tab == DisplayTab::Friends),
                            )
                            .event(|_: Click| {
                                self.current_tab.set(!self.current_tab.get());
                            }),
                    ),
            )
            .section(self.render_list(settings)?);

        AddonWindow::builder(ADDON_NAME)
            .header(header)
            .content(content)
            .build()
    }

    fn build_scroll_wrapper(
        &'static self,
        peer_id: Id,
        //settings: &'static Settings,
    ) -> ScrollWrapper {
        ScrollWrapper::builder(|| {
            || {
                let Some(old_scroll_target_id) = self.scroll_visible.replace(None) else {
                    return;
                };

                //debug_log!("old_target", old_target);
                get_engine()
                    .targets()
                    .unwrap_js()
                    .delete_arrow(&format!("Other-{old_scroll_target_id}"))
                    .unwrap_js();
                ColorMark::remove(HIGHLIGHT_COLOR, ADDON_NAME, old_scroll_target_id);
                //globals.others.remove_color_mark(old_target);
            }
        })
        .class_list("w[130] t-a[center] l[100%+4] align-center")
        .option(
            ScrollWrapperOption::builder()
                .text("Wyślij wiadomość")
                .on_click(move |_| {
                    get_engine()
                        .chat_controller()
                        .unwrap_js()
                        .get_chat_input_wrapper()
                        .unwrap_js()
                        .set_private_message_procedure(
                            Peers::get()
                                .lock_ref()
                                .get(&peer_id)
                                .unwrap_js()
                                .nick
                                .lock_ref()
                                .deref(),
                        )
                        .unwrap_js();
                })
                .build(),
        )
        .option(
            ScrollWrapperOption::builder()
                .text("Pokaż ekwipunek")
                .on_click(move |_| {
                    let Some(player_data) = 
                        Peers::get()
                            .lock_ref()
                            .get(&peer_id)
                            .and_then(|peer| ShowEqPlayerData::new_with_peer(peer))
                            .or_else(|| Others::get().lock_ref().get(&peer_id).and_then(|other| ShowEqPlayerData::new_with_other(other)))
                     else {
                        let _ = message("[MDMA::RS] Nie udało się wczytać ekwipunku gracza.");
                        return;
                    };
                    get_engine()
                        .show_eq_manager()
                        .unwrap_js()
                        .update(&player_data)
                        .unwrap_js()
                })
                .build(),
        )
        .option(
            ScrollWrapperOption::builder()
                .text("Zaproś do przyjaciół")
                .on_click(move |_| {
                    send_task(&format!(
                        "friends&a=finvite&nick={}",
                        Peers::get()
                            .lock_ref()
                            .get(&peer_id)
                            .unwrap_js()
                            .nick
                            .lock_ref()
                            .deref()
                            .trim()
                            .split_ascii_whitespace()
                            .join("_")
                    ))
                    .unwrap_js()
                })
                .build(),
        )
        .option(
            ScrollWrapperOption::builder()
                .text("Dodaj do wrogów")
                .on_click(move |_| {
                    send_task(&format!(
                        "friends&a=eadd&nick={}",
                        Peers::get()
                            .lock_ref()
                            .get(&peer_id)
                            .unwrap_js()
                            .nick
                            .lock_ref()
                            .deref()
                            .trim()
                            .split_ascii_whitespace()
                            .join("_")
                    ))
                    .unwrap_js()
                })
                .build(),
        )
        .option(
            ScrollWrapperOption::builder()
                .text("Zaproś do drużyny")
                .on_click(move |_| {
                    send_task(&format!("party&a=inv&id={peer_id}")).unwrap_js();
                })
                .build(),
        )
        .option(
            ScrollWrapperOption::builder()
                .text("Pokaż profil")
                .on_click(move |_| {
                    let Some((account_id, character_id)) = Peers::get()
                        .lock_ref()
                        .get(&peer_id)
                        .and_then(|peer_data| Some((peer_data.account.get()?, peer_data.char_id)))
                        .or_else(|| {
                            PlayersOnline::get()
                                .lock_ref()
                                .get(&peer_id)
                                .map(|player_data| (player_data.account, player_data.char_id))
                        })
                        .or_else(|| {
                            Others::get()
                                .lock_ref()
                                .get(&peer_id)
                                .and_then(|other_data| {
                                    Some((other_data.account, other_data.char_id))
                                })
                        })
                    else {
                        let _ = message("[MDMA::RS] Nie udało się wczytać profilu gracza.");
                        return;
                    };
                    let options = PlayerProfileOptions::new(account_id, character_id);

                    get_engine()
                        .inline_frame_window_manager()
                        .unwrap_js()
                        .new_player_profile(&options)
                        .unwrap_js();
                })
                .build(),
        )
        .build()
    }

    fn snap_back(
        new_stop_refresh: bool,
        stop_refresh: &Cell<bool>,
        before_wheel_stop: &Cell<bool>,
        start_snap_back_timeout: &Cell<Option<i32>>,
        start_snap_back: &Function,
        clear_snap_back_timeout: &Cell<Option<i32>>,
        clear_snap_back: &Function,
    ) {
        stop_refresh.set(new_stop_refresh);
        before_wheel_stop.set(true);

        let timeout = match new_stop_refresh {
            true => 1000,
            false => 50,
        };
        start_snap_back_timeout.set(Some(
            window()
                .set_timeout_with_callback_and_timeout_and_arguments_0(start_snap_back, timeout)
                .unwrap_js(),
        ));
        let timeout = match new_stop_refresh {
            true => 1200,
            false => 250,
        };
        clear_snap_back_timeout.set(Some(
            window()
                .set_timeout_with_callback_and_timeout_and_arguments_0(clear_snap_back, timeout)
                .unwrap_js(),
        ));
    }

    fn render_list(
        &'static self,
        settings: &'static Settings,
    ) -> JsResult<ContentSection> {
        let stop_refresh: &'static _ = Box::leak(Box::new(Cell::new(false)));
        let before_wheel_stop: &'static _ = Box::leak(Box::new(Cell::new(false)));
        let end_wheel_spin = closure!(move || {
            before_wheel_stop.set(false);
        });
        let current_overscroll: &'static _ = Box::leak(Box::new(Cell::new(0.0)));
        let delta: &'static _ = Box::leak(Box::new(Cell::new(0.0)));
        let timeout: &'static _ = Box::leak(Box::new(Cell::new(None)));
        let snap_back_timeout: &'static _ = Box::leak(Box::new(Cell::new(None)));
        let start_snap_back_timeout: &'static _ = Box::leak(Box::new(Cell::new(None)));
        let clear_snap_back_timeout: &'static _ = Box::leak(Box::new(Cell::new(None)));
        let transform: &'static _ = Box::leak(Box::new(Mutable::new(None)));
        let transform_scale: &'static _ = Box::leak(Box::new(Mutable::new(1)));

        // TODO: Is it possible that the scale stays at 0 until the next full refresh ?
        let start_snap_back = closure!(move || {
            // TODO: Change this to MAX_OVERSCROLL - epsilon instead of 35.
            if transform.get().is_some_and(|diff| diff == 35.0) {
                common::debug_log!("SETTING SCALE TO 0");
                transform_scale.set(0);
            } else {
                transform.set_neq(None);
            }
        });
        let clear_snap_back = closure!(move || {
            if transform.get().is_some() {
                wasm_bindgen_futures::spawn_local(async move {
                    transform.set_neq(None);
                    current_overscroll.set(0.0);
                    delay(200).await;
                    stop_refresh.set(false);
                    transform_scale.set(1);
                });
                return;
            }

            current_overscroll.set(0.0);
            stop_refresh.set(false);
            transform_scale.set(1);
        });

        let snap_back_from_timeout = closure!(
            {
                let start_snap_back = start_snap_back.clone(),
                let clear_snap_back = clear_snap_back.clone(),
            },
            move || {
                Self::snap_back(
                    false,
                    stop_refresh,
                    before_wheel_stop,
                    start_snap_back_timeout,
                    &start_snap_back,
                    clear_snap_back_timeout,
                    &clear_snap_back,
                );
            },
        );
        let top_shadow_animation: &'static _ = Box::leak(Box::new(MutableAnimation::new(0.0)));
        let bottom_shadow_animation: &'static _ = Box::leak(Box::new(
            MutableAnimation::new_with_initial(0.0, Percentage::END),
        ));
        //#[cfg(not(debug_assertions))]
        //let entries_signal = globals.peers.online_entries_signal();
        //#[cfg(debug_assertions)]
        //let entries_signal = globals.others.entries_cloned().to_signal_cloned();
        // TODO: Are the signals correct ?
        //let list_len_signal = map_ref! {
        //    let entries = entries_signal,
        //    let current_tab = self.current_tab.signal() => {
        //        entries.iter().filter(|(_, peer_data)| {
        //            let relation = peer_data.relation.as_ref().unwrap_js().get();
        //            let Some(_nick) = peer_data.nick.as_ref() else {
        //                return false;
        //            };
        //            let Some(_lvl) = peer_data.lvl.as_ref() else {
        //                return false;
        //            };
        //            let Some(_prof) = peer_data.prof.as_ref() else {
        //                return false;
        //            };
        //
        //            if cfg!(debug_assertions) {
        //                return true;
        //            }
        //            match current_tab {
        //                DisplayTab::ClanMembers => Relation::Clan == relation,
        //                DisplayTab::Friends => Relation::Friend == relation,
        //            }
        //        }).count()
        //
        //    }
        //};
        let window_size = Addons::get_window_size(ADDON_NAME, Self::WINDOW_TYPE)
            .ok_or_else(|| err_code!())?;

        Ok(ContentSection::new()
            .class_list("scroll-y min-h[50] fade-top-bottom d[flex] f-d[column] pos[relative]")
            .dynamic_class_signal(window_size.signal().map(Self::current_max_height_px))
            .event_with_elem(move |event: Wheel, elem| {
                let scroll_top = elem.scroll_top();
                let delta_y = event.delta_y();

                // TODO: delta >= or > ?
                if scroll_top != 0 || delta_y >= 0.0 {
                    return;
                }

                event.prevent_default();

                if stop_refresh.get() {
                    //common::debug_log!("stop_refresh");
                    return;
                }

                let responsiveness = Self::calculate_responsiveness(current_overscroll.get());
                let new_delta_y = delta_y * responsiveness;
                let old_delta = delta.replace(new_delta_y);

                if before_wheel_stop.get() {
                    let epsilon = 10.0 * responsiveness;
                    let new_timeout = window()
                        .set_timeout_with_callback_and_timeout_and_arguments_0(&end_wheel_spin, 50)
                        .unwrap_js();
                    if let Some(old_timeout_id) = timeout.replace(Some(new_timeout)) {
                        window().clear_timeout_with_handle(old_timeout_id);
                    }

                    if old_delta - new_delta_y <= epsilon {
                        //common::debug_log!("before_wheel_stop");
                        return;
                    }

                    before_wheel_stop.set(false);
                }

                if let Some(old_timeout_id) = start_snap_back_timeout.take() {
                    window().clear_timeout_with_handle(old_timeout_id);
                }
                if let Some(old_timeout_id) = clear_snap_back_timeout.take() {
                    window().clear_timeout_with_handle(old_timeout_id);
                }
                let new_timeout = window()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        &snap_back_from_timeout,
                        50,
                    )
                    .unwrap_js();
                if let Some(old_timeout_id) = snap_back_timeout.replace(Some(new_timeout)) {
                    window().clear_timeout_with_handle(old_timeout_id);
                }

                let new_overscroll = (current_overscroll.get() - new_delta_y).min(MAX_OVERSCROLL);
                //common::debug_log!(new_overscroll, "new_overscroll");
                current_overscroll.set(new_overscroll);
                transform.set_neq(Some(new_overscroll));

                // If within epsilon px from MAX_OVERSCROLL count scroll as a refresh.
                let epsilon = 15.0;
                if new_overscroll + epsilon < MAX_OVERSCROLL {
                    //common::debug_log!("epsilon");
                    return; // Return from timeout in before_wheel_stop.
                }

                //common::debug_log!("SUCCESSFULL REFRESH");
                if let Some(old_timeout_id) = start_snap_back_timeout.take() {
                    window().clear_timeout_with_handle(old_timeout_id);
                }
                if let Some(old_timeout_id) = clear_snap_back_timeout.take() {
                    window().clear_timeout_with_handle(old_timeout_id);
                }
                if let Some(old_timeout_id) = snap_back_timeout.take() {
                    window().clear_timeout_with_handle(old_timeout_id);
                }
                if let Some(old_timeout_id) = timeout.take() {
                    window().clear_timeout_with_handle(old_timeout_id);
                }
                Self::snap_back(
                    true,
                    stop_refresh,
                    before_wheel_stop,
                    start_snap_back_timeout,
                    &start_snap_back,
                    clear_snap_back_timeout,
                    &clear_snap_back,
                );
                wasm_bindgen_futures::spawn_local(async move {
                    delay(200).await;
                    transform.set(Some(MAX_OVERSCROLL - epsilon));
                });
                self.refresh();
            })
            .custom_property_signal(
                "--fade-start",
                top_shadow_animation
                    .signal()
                    .map(|t| format!("{:.2}%", t.range_inclusive(0.0, TOP_FADE_MAX * 100.0))),
            )
            .custom_property_signal(
                "--fade-end",
                bottom_shadow_animation.signal().map(|t| {
                    format!(
                        "{:.2}%",
                        t.invert().range_inclusive(BOTTOM_FADE_MIN * 100.0, 100.0)
                    )
                }),
            )
            .event_with_elem(move |_event: Scroll, container: &HtmlDivElement| {
                let current_max_height = Self::current_max_height(window_size.get());
                let threshold = current_max_height * TOP_FADE_MAX;
                Self::update_shadow_animations(
                    threshold,
                    top_shadow_animation,
                    bottom_shadow_animation,
                    container,
                )
            })
            .mixin(|b| {
                b.after_inserted(move |container| {
                    let element = container.clone();
                    let future = dominator_helpers::DomRectSignal::new(&element)
                        .switch(move |_| {
                            window_size.signal_ref(move |size| {
                                Self::current_max_height(*size) * TOP_FADE_MAX
                            })
                        })
                        .for_each(move |threshold| {
                            Self::update_shadow_animations(
                                threshold,
                                top_shadow_animation,
                                bottom_shadow_animation,
                                &container,
                            );

                            async {}
                        });
                    wasm_bindgen_futures::spawn_local(future);
                })
            })
            .section(
                ContentSection::new()
                    .class_list("refresh-icon-background pos[absolute] t[-27] l[50%]")
                    .custom_property_signal(
                        "transform",
                        map_ref! {
                            let offset_opt = transform.signal(),
                            let scale = transform_scale.signal() => {
                                format!(
                                    "translate(-50%, {}px) scale({})",
                                    offset_opt.unwrap_or(0f64),
                                    *scale,
                                )
                            }

                        },
                    ),
            )
            .section(
                ContentSection::new()
                    .class_list("refresh-icon pos[absolute] t[-20] l[50%]")
                    .custom_property_signal(
                        "transform",
                        map_ref! {
                            let offset_opt = transform.signal(),
                            let scale = transform_scale.signal() => {
                                format!(
                                    "translate(-50%, {}px) rotate({}deg) scale({})",
                                    offset_opt.unwrap_or(0f64),
                                    offset_opt.unwrap_or(0f64) / MAX_OVERSCROLL * 360.0 - 126.0,
                                    *scale,
                                )
                            }

                        },
                    )
                    .custom_property_signal(
                        "filter",
                        map_ref! {
                            let offset_opt = transform.signal(),
                            let _scale = transform_scale.signal() => {
                                format!(
                                    "brightness({})",
                                    offset_opt.unwrap_or(0.0).min(30.0) / (30.0) * 1.5
                                )
                            }

                        },
                    ),
            )
            .section_signal_vec(
                settings
                    .sort_signal()
                    .switch(|_| Others::get().len())
                    .switch_signal_vec(move |_| {
                        Peers::get()
                            .online_from_keys_signal()
                            .map(|mut entries| {
                                entries.sort_by(|(_, a), (_, b)| {
                                    settings.compare(a, b)
                                });
                                entries
                            })
                            .to_signal_vec()
                            .filter_map(move |(peer_id, peer_data)| {
                                self.render_one_peer(peer_id, peer_data, settings)
                            })
                    }),
            ))
        //.mixin(|builder| {
        //    apply_methods!(builder, {
        //        .with_cfg!(debug_assertions, {
        //            .children_signal_vec(
        //                settings
        //                    .sort_signal()
        //                    .switch_signal_vec(move |_| {
        //                        globals
        //                            .others
        //                            .signal_vec_keys()
        //                            .filter_map(|other_id| {
        //                                let other_data = globals.others.lock_ref().get(&other_id).cloned()?;
        //                                other_data.nick.is_some().then_some((other_id, other_data))
        //                            })
        //                            .to_signal_cloned()
        //                            .map(|mut entries| {
        //                                entries.sort_by(|(_, a),(_,b)| settings.compare(a, b, &globals.others));
        //                                entries
        //                            })
        //                            .to_signal_vec()
        //                            .filter_map(move |(peer_id, peer_data)| self.render_one_peer(peer_id, peer_data, settings, globals).map(ContentSection::render))
        //                    })
        //            )
        //        })
        //    })
        //}))
    }

    fn calculate_responsiveness(current_overscroll: f64) -> f64 {
        let progress = current_overscroll / MAX_OVERSCROLL;
        let factor = 1.0 - progress;

        (0.3 * factor).max(0.1)
    }

    fn refresh(&self) {
        let current_tab = self.current_tab.get();
        if !Hero::is_in_clan() && current_tab == DisplayTab::ClanMembers {
            return;
        }

        let emitter_event = match current_tab {
            DisplayTab::ClanMembers => EmitterEvent::Members,
            DisplayTab::Friends => EmitterEvent::Friends,
        };
        let task = match current_tab {
            DisplayTab::ClanMembers => clan::get_members,
            DisplayTab::Friends => friends::get_friends,
        };

        fn callback<'a>(
            socket_response: &'a mut Response,
        ) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>> {
            Box::pin(async move {
                Peers::extract_from_socket_response(socket_response);
                Ok(())
            })
        }
        
        if let Err(err_code) = Emitter::intercept_once(emitter_event, callback) {
            return console_error!(err_code)
        }
        if let Err(_err) = send_task(task) {
            common::debug_log!(_err);
        }
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

    fn render_one_peer(
        &'static self,
        peer_id: PeerId,
        peer_data: Peer,
        settings: &'static Settings,
    ) -> Option<ContentSection> {
        let relation = peer_data.relation.get();
        let nick = peer_data.nick;
        let lvl = peer_data.lvl;
        let oplvl = peer_data.operational_lvl;
        let prof = peer_data.prof;
        let map_name = peer_data.map_name;
        let x_mutable = peer_data.x;
        let y_mutable = peer_data.y;

        #[cfg(debug_assertions)]
        let is_scroll_target_signal =
            self.is_scroll_target_signal(peer_id);
        #[cfg(not(debug_assertions))]
        let is_scroll_target_signal = self.is_scroll_target_signal(peer_id);
        let peer = ContentSection::new()
            .class_list("one-other f[1] g[3] overflow[hidden]")
            .class_signal("selected", is_scroll_target_signal)
            .section(
                ContentSection::new()
                    .class_list("other-nick")
                    .text_signal(nick.signal_cloned()),
            )
            .section(
                ContentSection::new()
                    .class_list("other-lvl m-right[auto]")
                    .text_signal(map_ref! {
                        let level_display = settings.level_display.signal(),
                        let lvl = lvl.signal(),
                        let oplvl = oplvl.signal(),
                        let prof = prof.signal() => {
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
            .section_signal(settings.show_alias
                .signal()
                .switch(clone!(map_name => move |show_alias| {
                    map_name.signal_ref(move |map_name_opt| {
                        let map_alias = ActiveSettings::to_map_alias(map_name_opt.as_ref().map(|s| s.as_str()).unwrap_or_default());

                        if map_alias.is_empty() || !show_alias {
                            return None;
                        }

                        let map_alias_tip = ActiveSettings::to_map_alias_tip(map_name_opt.as_ref().map(|s| s.as_str()).unwrap_or_default());

                        Some(ContentSection::new()
                            .class_list("other-map-alias p-left[3] f[none]")
                            .text(map_alias)
                            .mixin(move |b| apply_methods!(b, {
                                .tip!({ .text(map_alias_tip) })
                            }))
                        )
                    })
                }))
            )
            .section_signal(map_name.signal_ref(move |map_name_opt| {
                map_name_opt.as_ref().map(|map_name| {
                    ContentSection::new()
                        .class_list("other-map p-left[3]")
                        .visible_signal(settings.show_location.signal())
                        .text(map_name)
                })
                })
            )
            .section_signal(Town::get()
                .signal_ref(move |town| town.name.clone())
                .switch(clone!(map_name => move |town_name_opt| {
                    map_name.signal_ref(move |peer_map_name_opt| town_name_opt == *peer_map_name_opt)
                    
                }))
                .switch(|same_map| {
                    settings.show_location.signal().map(move |show_location| same_map && show_location)
                })
                .dedupe()
                .switch(move |show_coords| {
                    map_ref! {
                        let x_opt = x_mutable.signal(),
                        let y_opt = y_mutable.signal() => {
                            match (show_coords, x_opt, y_opt) {
                                (true, Some(x), Some(y)) => Some(ContentSection::new()
                                    .class_list("other-coords p-left[3] f[none]")
                                    .text(&format!("({},{})", x, y))),
                                _ => None,
                            }
                        }
                    }
                })
            )
            .event(move |event: DoubleClick| {
                if event.button() != MouseButton::Left {
                    return;
                }
                if !Others::get().lock_ref().contains_key(&peer_id) {
                    return;
                }

                wasm_bindgen_futures::spawn_local(async move {
                    get_engine()
                        .hero()
                        .unwrap_js()
                        .auto_go_to_other(peer_id)
                        .await
                        .unwrap_js();
                });
            })
            .event(move |event: ContextMenu| {
                if event.button() != MouseButton::Right {
                    return;
                }

                self.scroll_visible.set(Some(peer_id));
            })
            .event_with_options(&EventOptions::preventable(), move |_: MouseDown| {
                if !Others::get().lock_ref().contains_key(&peer_id) {
                    return;
                }
                if ColorMark::has_mark(HIGHLIGHT_COLOR, ADDON_NAME, &peer_id)
                    && !self.is_scroll_target(peer_id)
                {
                    return;
                }

                let other = get_engine()
                    .others()
                    .unwrap_js()
                    .get_by_id(peer_id)
                    .unwrap_js();
                //debug_log!("other:", &other);
                get_engine()
                    .targets()
                    .unwrap_js()
                    .add_arrow(false, &other.d().nick(), &other, "Other", "navigate")
                    .unwrap_js();
                if let Err(err_code) =
                    Others::init_color_mark(HIGHLIGHT_COLOR, ADDON_NAME, peer_id)
                {
                    console_error!(err_code);
                }
            })
            .event(move |_: MouseEnter| {
                if !Others::get().lock_ref().contains_key(&peer_id) {
                    return;
                }
                if self.is_scroll_active() {
                    return;
                }

                let other = get_engine()
                    .others()
                    .unwrap_js()
                    .get_by_id(peer_id)
                    .unwrap_js();
                get_engine()
                    .targets()
                    .unwrap_js()
                    .add_arrow(false, &other.d().nick(), &other, "Other", "navigate")
                    .unwrap_js();
                if let Err(err_code) =
                    Others::init_color_mark(HIGHLIGHT_COLOR, ADDON_NAME, peer_id)
                {
                    console_error!(err_code);
                }
            })
            .event(move |_: MouseLeave| {
                if self.is_scroll_target(peer_id) {
                    return;
                }
                if !ColorMark::has_mark(HIGHLIGHT_COLOR, ADDON_NAME, &peer_id) {
                    return;
                }

                get_engine()
                    .targets()
                    .unwrap_js()
                    .delete_arrow(&format!("Other-{peer_id}"))
                    .unwrap_js();
                ColorMark::remove(HIGHLIGHT_COLOR, ADDON_NAME, peer_id);
            })
            .after_inserted(move |_| {
                if !self.is_scroll_target(peer_id) {
                    return;
                }

                let Some(other) = get_engine().others().unwrap_js().get_by_id(peer_id) else {
                    return;
                };
                get_engine()
                    .targets()
                    .unwrap_js()
                    .add_arrow(false, &other.d().nick(), &other, "Other", "navigate")
                    .unwrap_js();
                if let Err(err_code) =
                    Others::init_color_mark(HIGHLIGHT_COLOR, ADDON_NAME, peer_id)
                {
                    console_error!(err_code);
                }
            })
            .after_removed(move |_| {
                if !Others::get().lock_ref().contains_key(&peer_id) {
                    return;
                }
                if !ColorMark::has_mark(HIGHLIGHT_COLOR, ADDON_NAME, &peer_id) {
                    return;
                }

                get_engine()
                    .targets()
                    .unwrap_js()
                    .delete_arrow(&format!("Other-{peer_id}"))
                    .unwrap_js();
                ColorMark::remove(HIGHLIGHT_COLOR, ADDON_NAME, peer_id);
            })
            .mixin(move |b| {
                let overflow_signal_options = dominator_helpers::OverflowSignalOptions::builder()
                    .with_subtree(1)
                    .build();
                let tip_active_signal = dominator_helpers::OverflowSignal::new_with_options(
                    &b.__internal_element(),
                    overflow_signal_options,
                )
                .switch(move |from_overflow| {
                    settings
                        .always_show_tip
                        .signal()
                        .map(move |from_settings| from_settings || from_overflow)
                });
                apply_methods!(b, {
                    .tip!(tip_active_signal => {
                        .class("max-w[250]")
                        // TODO: Add this using game wanted list ?
                        //.apply_if(is_wanted, |b| b.child(html!("div", {
                        //    .class("wanted")
                        //})))
                        .child(html!("div", {
                            .class("nick")
                            .text_signal(map_ref!{
                                let level_display = settings.level_display.signal(),
                                let nick = nick.signal_cloned(),
                                let lvl = lvl.signal(),
                                let oplvl = oplvl.signal(),
                                let prof = prof.signal() => {
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
                        // TODO: Add this.
                        //.apply_if(other_data.clan.is_some(), |b| b.child(html!("div", {
                        //    .class("clan-in-tip")
                        //    .text(other_data.clan.as_ref().unwrap_js().name.as_str())
                        //})))
                        .child_signal(
                            map_name.signal_ref(|map_name_opt| map_name_opt.as_ref().map(|map_name| {
                                html!("div", {
                                    .class("map-name")
                                    .text(map_name)
                                })
                            }))
                        )
                    })
                })
            });

        let inv_button = ContentSection::new()
            .class_list("one-other w[25] j-c[center]")
            .section(
                ContentSection::new()
                    .text("✕")
                    .mixin(|b| b.style("transform", "rotate(45deg)")),
            )
            .mixin(|b| {
                apply_methods!(b, {
                    .tip!({
                        .text("Zaproś do grupy")
                    })
                })
            })
            .event(move |_: Click| {
                communication::send_task(&communication::party::invite(peer_id)).unwrap_js();
            });

        Some(
            ContentSection::new()
                .class_list("d[flex] f-d[row] w[100%] g[1]")
                .visible_signal(self.current_tab.signal().map(move |current_tab| {
                    if cfg!(debug_assertions) {
                        return true;
                    }

                    match current_tab {
                        DisplayTab::ClanMembers => Relation::Clan == relation,
                        DisplayTab::Friends => Relation::Friend == relation,
                    }
                }))
                .section(peer)
                .section(inv_button),
        )
    }

    fn current_max_height(size: u8) -> f64 {
        match size {
            0 => 265.0,
            _ => window().inner_height().unwrap_js().unchecked_into_f64() / 2.0,
        }
    }

    fn current_max_height_px(size: u8) -> &'static str {
        match size {
            0 => "max-h[265]",
            _ => "max-h[50vh]",
        }
    }
}

pub(super) fn init(
    active_settings: &'static ActiveSettings,
    settings: &'static Settings,
) -> JsResult<()> {
    let _settings_window_handle = WINDOWS_ROOT
        .try_append_dom(settings.render()?)
        .ok_or_else(|| err_code!())?;
    let _addon_window_handle = WINDOWS_ROOT
        .try_append_dom(active_settings.render(settings)?)
        .ok_or_else(|| err_code!())?;

    Ok(())
}
