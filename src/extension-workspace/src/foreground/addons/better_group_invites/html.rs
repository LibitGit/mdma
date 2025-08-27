use dominator::{Dom, apply_methods, clone, html};
use futures_signals::map_ref;

use crate::addon_window::prelude::*;
use crate::interface::tips_parser::{info_bubble, tip};
use crate::interface::{ThreadLocalShadowRoot, WINDOWS_ROOT};

use super::*;

//struct ArrowProps {
//    width: f64,
//    height: f64,
//    color: String,
//}

impl WindowContent {
    //    fn priority_setting(self) -> Self {
    //        use dominator::svg;
    //
    //let props = ArrowProps{
    //            width: 20.0,
    //            height: 68.0,
    //            color: String::from("beige"),
    //        };
    //    let base_width = 20.0;
    //    let base_height = 68.0;
    //    let arrow_width = props.width * 0.6;
    //    let curve_control1_x = props.width * 0.2;
    //    let curve_control1_y = props.height * 0.3;
    //    let curve_control2_x = props.width * 0.8;
    //    let curve_control2_y = props.height * 0.6;
    //    let dot_spacing = 3.0;
    //    let dot_radius = 1.0;
    //
    //    let mid_x = props.width / 2.0;
    //
    //        self.heading(Heading::builder().text("Priorytet zapraszania"))
    //            .child(html!("div", {
    //            .style("border-radius", "7px")
    //            .style("box-shadow", "0 0 1px #010101, 0 0 0 1px #ccc, 0 0 0 2px #0c0d0d, 1px 1px 2px 2px #0c0d0d66")
    //            .class!(m[0-10] p[5] d[flex] f-d[row])
    //            .child(html!("div", {
    //                .class!(d[flex] f-d[column] a-i[center])
    //                .child(html!("div", {
    //                    .class("trophy")
    //                }))
    //                .child(svg!("svg", {
    //                .attr("viewBox", &format!("0 0 {} {}", base_width, base_height))
    //                .attr("preserveAspectRatio", "xMidYMid meet")
    //                .children([
    //                    // Arrow head
    //                    svg!("path", {
    //                        .attr("d", &format!("M {} 0 L {} {} M {} 0 L {} {}",
    //                            mid_x,
    //                            mid_x - arrow_width / 5.0, arrow_width / 3.0,
    //                            mid_x,
    //                            mid_x + arrow_width / 5.0, arrow_width / 3.0
    //                        ))
    //                        .attr("fill", "none")
    //                        .attr("stroke", &props.color)
    //                        .attr("stroke-width", "1")
    //                    }),
    //                    // Curved line
    //                    svg!("path", {
    //                        .attr("d", &format!("M {} {} C {} {}, {} {}, {} {} C {} {}, {} {}, {} {}",
    //                            mid_x, arrow_width / 4.0,
    //                            curve_control1_x, curve_control1_y,
    //                            curve_control2_x, curve_control2_y,
    //                            mid_x, props.height - 5.0 * dot_spacing + 1.5,
    //                            curve_control1_x, curve_control1_y + props.height - 18.0,
    //                            curve_control2_x, curve_control2_y + props.height,
    //                            mid_x, props.height * 2.0 - 5.0 * dot_spacing + 1.5
    //                        ))
    //                        .attr("fill", "none")
    //                        .attr("stroke", &props.color)
    //                        .attr("stroke-width", "1")
    //                    }),
    //                ])
    //            }))
    //                .child(svg!("svg", {
    //                    .attr("viewBox", "0 0 20 15")
    //                    .attr("preserveAspectRatio", "xMidYMid meet")
    //                    .children([
    //                    // Dots
    //                    svg!("circle", {
    //                        .attr("cx", &(mid_x - 0.5).to_string())
    //                        .attr("cy", &dot_spacing.to_string())
    //                        .attr("r", &dot_radius.to_string())
    //                        .attr("fill", &props.color)
    //                    }),
    //                    svg!("circle", {
    //                        .attr("cx", &(mid_x - 1.0).to_string())
    //                        .attr("cy", &(2.0 * dot_spacing).to_string())
    //                        .attr("r", &(dot_radius - 0.1).to_string())
    //                        .attr("fill", &props.color)
    //                    }),
    //                    svg!("circle", {
    //                        .attr("cx", &(mid_x- 0.5).to_string())
    //                        .attr("cy", &(3.0 * dot_spacing).to_string())
    //                        .attr("r", &(dot_radius - 0.2).to_string())
    //                        .attr("fill", &props.color)
    //                    }),
    //                    svg!("circle", {
    //                        .attr("cx", &mid_x.to_string())
    //                        .attr("cy", &(4.0 * dot_spacing).to_string())
    //                        .attr("r", &(dot_radius - 0.3).to_string() )
    //                        .attr("fill", &props.color)
    //                    })
    //                    ])
    //
    //                }))
    //                .child(html!("div", {
    //                    .class("skull")
    //                }))
    //            }))
    //            .child(html!("div", {
    //                .class!(w[100%] h[100%] d[flex] f-d[column])
    //                .child(html!("div", {
    //                    .class!(list-item d[flex] a-i[center])
    //                    .child(html!("div", {
    //                        .class("p-left[5]")
    //                        .text("Przyjaciel")
    //                    }))
    //                }))
    //                .child(html!("div", {
    //                    .class!(list-item d[flex] a-i[center])
    //                    .child(html!("div", {
    //                        .class("p-left[5]")
    //                        .text("Członek klanu")
    //                    }))
    //                }))
    //                .child(html!("div", {
    //                    .class!(list-item d[flex] a-i[center])
    //                    .child(html!("div", {
    //                        .class("p-left[5]")
    //                        .text("Sojusznik klanu")
    //                    }))
    //                }))
    //                .child(html!("div", {
    //                    .class!(list-item d[flex] a-i[center])
    //                    .child(html!("div", {
    //                        .class("p-left[5]")
    //                        .text("Nieznajomy")
    //                    }))
    //                }))
    //            }))
    //        }))
    //    }

    fn hotkey_setting(self, hotkey: &'static Hotkey, mass_invite_hotkey: &'static Hotkey) -> Self {
        let hotkey_heading = Heading::builder()
            .class_list("m[0]")
            .text("Skróty klawiszowe").mixin(|builder| {
            apply_methods!(builder, {
                .info_bubble!({
                    .text("Do dokładniejszego zapraszania (np. według nicku) używaj przycisków z okna dodatku!")
                })
            })
        });
        let invite_checkbox = Checkbox::builder(hotkey.active.clone())
            .text("Klawisz do zapraszania")
            .on_click(Hotkey::on_click_factory(hotkey, mass_invite_hotkey));
        let invite_input = Input::builder()
            .class_list("keybind")
            .value(hotkey.value.get_cloned())
            .maxlength("1")
            .on_key_down(Hotkey::on_key_down_factory(hotkey, mass_invite_hotkey));
        let invite_section = ContentSection::new()
            .class_list("label j-c[space-between]")
            .checkbox(invite_checkbox)
            .input(invite_input);

        let mass_invite_checkbox = Checkbox::builder(mass_invite_hotkey.active.clone())
            .text("Klawisz do masowego zapraszania")
            .on_click(Hotkey::on_click_factory(mass_invite_hotkey, hotkey));
        let mass_invite_input = Input::builder()
            .class_list("keybind")
            .value(mass_invite_hotkey.value.get_cloned())
            .maxlength("1")
            .on_key_down(Hotkey::on_key_down_factory(mass_invite_hotkey, hotkey));
        let mass_invite_section = ContentSection::new()
            .class_list("label j-c[space-between]")
            .checkbox(mass_invite_checkbox)
            .input(mass_invite_input);

        self.heading(hotkey_heading)
            .section(invite_section)
            .section(mass_invite_section)
    }

    fn delay_range_setting(
        self,
        min: &'static LinkedInput<u32>,
        max: &'static LinkedInput<u32>,
    ) -> Self {
        let delay_heading = Heading::builder()
            .text("Opóźnienie między zaproszeniami")
            .mixin(|builder| apply_methods!(builder, {
                .info_bubble!({
                    .text("Zakres czasowy w milisekundach między wysyłaniem zaproszeń do grupy.")
                })
            }));
        let min_delay_input = Input::builder()
            .input_type(InputType::number(100.0, f64::MAX))
            .text("Minimalne")
            .size(InputSize::Custom("w[64]"))
            .placeholder_signal(min.value.signal_ref(|value| value.to_string()))
            .value(min.value.get().to_string())
            .store_root(&min.root)
            .with_tooltip(&min.custom_validity)
            .on_input(LinkedInput::on_input_factory(min, max));
        let max_delay_input = Input::builder()
            .input_type(InputType::number(100.0, f64::MAX))
            .text("Maksymalne")
            .size(InputSize::Custom("w[64]"))
            .placeholder_signal(max.value.signal_ref(|value| value.to_string()))
            .value(max.value.get().to_string())
            .store_root(&max.root)
            .with_tooltip(&max.custom_validity)
            .on_input(LinkedInput::on_input_factory(max, min));
        let delay_input_pair = InputPair::builder(min_delay_input, max_delay_input)
            .class_list("j-c[space-around]")
            .no_delimiter();

        self.heading(delay_heading).input_pair(delay_input_pair)
    }

    fn invite_setting(self, relations: &Relations) -> Self {
        let invite_setting_heading = Heading::builder().text("Automatycznie wysyłaj do");
        let none_checkbox = Checkbox::builder(relations.none.clone()).text("Nieznajomych");
        let friend_checkbox = Checkbox::builder(relations.friend.clone()).text("Przyjaciół");
        let clan_checkbox = Checkbox::builder(relations.clan.clone()).text("Członków klanu");
        let clan_ally_checkbox =
            Checkbox::builder(relations.clan_ally.clone()).text("Sojuszników klanu");

        self.heading(invite_setting_heading)
            .checkbox_pair(none_checkbox, friend_checkbox)
            .checkbox_pair(clan_checkbox, clan_ally_checkbox)
            .apply_if(WorldConfig::has_fractions(), |builder| {
                let fraction_ally_checkbox =
                    Checkbox::builder(relations.fraction_ally.clone()).text("Sojuszników frakcji");
                builder.checkbox(fraction_ally_checkbox)
            })
    }

    //TODO: Rewrite info bubble builder to require build step and make use of ContentSection
    //possible that way
    fn mass_invite_setting(self, mass_invite_peers: &FromPeers) -> Self {
        let mass_invite_heading = Heading::builder().text("Masowo zapraszaj");
        let friend_checkbox =
            Checkbox::builder(mass_invite_peers.friend.clone()).text("Przyjaciół");
        let clan_checkbox =
            Checkbox::builder(mass_invite_peers.clan.clone()).text("Członków klanu");
        let all_checkbox = Checkbox::builder(mass_invite_peers.from_location.clone())
            .text("Graczy z mapy")
            .info_bubble(InfoBubble::builder().mixin(|builder| apply_methods!(builder, {
                .tip!({
                    .child(html!("div", {
                        .class("m-bottom[3]")
                        .text("Zaproszenia będą wysyłane do graczy z aktualnej lokacji.")
                    }))
                    .child(html!("div", {
                        .text("Zapraszanie graczy z aktualnej lokacji przebiega według ustawień automatycznego zapraszania.")
                    }))
                })
            })).build());

        self.heading(mass_invite_heading)
            .checkbox_pair(friend_checkbox, clan_checkbox)
            .checkbox(all_checkbox)
    }

    pub(crate) fn excluded_nicks_setting(self, excluded_nicks: &'static NickInput) -> Self {
        let exclusion_list_input = Input::builder()
            .class_list("m-left[10]")
            .placeholder("Nick Gracza")
            .maxlength("21")
            .text_align(TextAlign::Left)
            .size(InputSize::Big)
            .store_root(&excluded_nicks.root)
            .with_tooltip(&excluded_nicks.custom_validity)
            .on_input(NickInput::on_input_factory(&excluded_nicks))
            .confirm_button(
                InputButton::builder()
                    .with_tooltip(&excluded_nicks.custom_validity)
                    .on_click(NickInput::on_click_factory(&excluded_nicks)),
            );
        let excluded_nicks_signal = excluded_nicks
            .signal_ref(move |excluded_nicks_vec| {
                excluded_nicks_vec
                    .iter()
                    .map(|excluded_nick| {
                        Input::builder()
                            .disabled()
                            .class_list("m-left[10] m-bottom[6]")
                            .value(excluded_nick)
                            .size(InputSize::Big)
                            .confirm_button(
                                InputButton::builder()
                                    .button_type(InputButtonType::Remove)
                                    .on_click(clone!(excluded_nick => move |_, _| {
                                        excluded_nicks
                                            .lock_mut()
                                            .retain(|nick| *nick != excluded_nick)
                                    }))
                                    .class_list("m-bottom[6]"),
                            )
                    })
                    .collect()
            })
            .to_signal_vec();

        self.input_signal_vec(excluded_nicks_signal)
            .input(exclusion_list_input)
    }

    fn invite_with_professions_setting(self, professions: &'static Professions) -> Self {
        self.checkbox(Checkbox::builder(professions.active.clone()).text("Zaproś według profesji"))
            .section_signal(professions.active.signal().map(move |active| {
                if !active {
                    return None;
                }

                Some(
                    ContentSection::new()
                        .class_list("label h[auto] f-d[column] j-c[space-between] g[3] p-top[6]")
                        .invite_with_profession_setting(
                            &professions,
                            &professions.warrior,
                            "Wojownicy",
                        )
                        .invite_with_profession_setting(&professions, &professions.mage, "Magowie")
                        .invite_with_profession_setting(&professions, &professions.hunter, "Łowcy")
                        .invite_with_profession_setting(
                            &professions,
                            &professions.paladin,
                            "Paladyni",
                        )
                        .invite_with_profession_setting(
                            &professions,
                            &professions.blade_dancer,
                            "Tancerze ostrzy",
                        )
                        .invite_with_profession_setting(
                            &professions,
                            &professions.tracker,
                            "Tropiciele",
                        ),
                )
            }))
    }

    fn invite_with_nick_setting(self, nicks: &'static Nicks) -> Self {
        let nick_checkbox = Checkbox::builder(nicks.active.clone())
            .text("Zaproś według nicków")
            .info_bubble(
                InfoBubble::builder()
                    .text("Wielkość liter nie ma znaczenia.")
                    .build(),
            );

        self.checkbox(nick_checkbox)
            .section_signal(nicks.active.signal_ref(move |&active| {
                if !active {
                    return None;
                }

                let with_nicks = &nicks.values;
                let nicks_input = Input::builder()
                    .class_list("m-left[10] p-left[10]")
                    .placeholder("Nick Gracza")
                    .maxlength("21")
                    .size(InputSize::Custom("w[127]"))
                    .text_align(TextAlign::Left)
                    .with_tooltip(&nicks.values.custom_validity)
                    .on_input(NickInput::on_input_factory(&with_nicks))
                    .confirm_button(
                        InputButton::builder()
                            .with_tooltip(&with_nicks.custom_validity)
                            .on_click(NickInput::on_click_factory(&with_nicks)),
                    );
                //TODO: Fix this monstrocity.
                let nicks_signal = with_nicks
                    .signal_ref(move |excluded_nicks| {
                        excluded_nicks
                            .iter()
                            .map(move |excluded_nick| {
                                Input::builder()
                                    .disabled()
                                    .class_list("m-left[10] m-bottom[6]")
                                    .value(excluded_nick)
                                    .size(InputSize::Custom("w[127]"))
                                    .confirm_button(
                                        InputButton::builder()
                                            .button_type(InputButtonType::Remove)
                                            .on_click(clone!(excluded_nick => move |_, _| {
                                                with_nicks
                                                    .lock_mut()
                                                    .retain(|nick| *nick != excluded_nick)
                                            }))
                                            .class_list("m-bottom[6]"),
                                    )
                            })
                            .collect()
                    })
                    .to_signal_vec();

                Some(
                    ContentSection::new()
                        .class_list("label h[auto] p-top[6] g[0] f-d[column]")
                        .input_signal_vec(nicks_signal)
                        .input(nicks_input),
                )
            }))
    }

    fn invite_with_lvl_setting(self, lvl_range: &'static LevelRange) -> Self {
        self.checkbox(Checkbox::builder(lvl_range.active.clone()).text("Zaproś według poziomów"))
            .input_pair_signal(lvl_range.active.signal_ref(move |&active| {
                if !active {
                    return None;
                }

                let min_lvl_range = &lvl_range.min;
                let max_lvl_range = &lvl_range.max;
                let min_lvl_range_input = Input::builder()
                    .input_type(InputType::number(1.0, 500.0))
                    .placeholder_signal(min_lvl_range.value.signal_ref(|lvl| lvl.to_string()))
                    .value(min_lvl_range.value.get().to_string())
                    .store_root(&min_lvl_range.root)
                    .with_tooltip(&min_lvl_range.custom_validity)
                    .on_input(LinkedInput::on_input_factory(min_lvl_range, max_lvl_range));
                let max_lvl_range_input = Input::builder()
                    .input_type(InputType::number(1.0, 500.0))
                    .placeholder_signal(max_lvl_range.value.signal_ref(|lvl| lvl.to_string()))
                    .value(max_lvl_range.value.get().to_string())
                    .store_root(&max_lvl_range.root)
                    .with_tooltip(&max_lvl_range.custom_validity)
                    .on_input(LinkedInput::on_input_factory(max_lvl_range, min_lvl_range));

                Some(
                    InputPair::builder(min_lvl_range_input, max_lvl_range_input)
                        .class_list("j-c[center] f[1]"),
                )
            }))
    }

    fn invite_buttons(
        self,
        active_settings: &'static ActiveSettings,
        settings: &'static Settings,
    ) -> Self {
        let invite_button = Button::builder()
            .class_list("w[80] f-s[11]")
            .text("Zaproś")
            .on_click(ActiveSettings::invite_button_onclick_factory(
                active_settings,
                settings,
            ));
        let mass_invite_button = Button::builder()
            .class_list("w[105] f-s[11]")
            .text("Zaproś masowo")
            .on_click(ActiveSettings::mass_invite_button_onclick_factory(
                active_settings,
                settings,
            ));
        let invite_button_pair =
            ButtonPair::builder(invite_button, mass_invite_button).class_list("p-top[6]");

        self.button_pair(invite_button_pair)
    }
}

impl ContentSection {
    fn invite_with_profession_setting(
        self,
        professions: &'static Professions,
        target_prof: &'static ProfessionSetting,
        text: &'static str,
    ) -> Self {
        let profession_checkbox = Checkbox::builder(target_prof.active.clone())
            .text(text)
            .class_list("p-left[10]")
            .on_click(Professions::on_click_factory(professions, target_prof));
        let profession_input = Input::builder()
            .input_type(InputType::number(0.0, 9.0))
            .placeholder_signal(target_prof.value.signal_ref(|value| value.to_string()))
            .value(target_prof.value.get().to_string())
            .store_root(&target_prof.root)
            .with_tooltip(&target_prof.custom_validity)
            .on_input(ProfessionSetting::on_input_factory(
                professions,
                target_prof,
            ));

        self.section(
            ContentSection::new()
                .class_list("profession-setting-wrapper")
                .checkbox(profession_checkbox)
                .input(profession_input),
        )
    }
}

impl Settings {
    fn render(&'static self) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_right(
                decors::CloseButton::builder()
                    .hide_window_on_click(false)
                    .on_click(Self::shake_on_unsaved(Self::WINDOW_TYPE))
                    .build(),
            )
            .push_left(decors::OpacityToggle::new())
            .build();
        let settings_window_header = WindowHeader::new(decor);

        let exclusion_list_heading = Heading::builder().text("Lista wykluczeń").mixin(|builder| {
            apply_methods!(builder, {
                .info_bubble!({
                    .child(html!("div", {
                        .class("m-bottom[3]")
                        .text("Gracze z tej listy nie będą automatycznie zapraszani do grupy.")
                    }))
                    .text("Wielkość liter nie ma znaczenia.")
                })
            })
        });

        let settings_window_content = WindowContent::builder()
            .hotkey_setting(&self.hotkey, &self.mass_invite.hotkey)
            .delay_range_setting(&self.delay.min, &self.delay.max)
            .invite_setting(&self.relations)
            .mass_invite_setting(&self.mass_invite.peers)
            //TODO: Smaller height add all prof, relation, nick, lvl make draggable animation.
            //.priority_setting()
            .heading(exclusion_list_heading)
            .excluded_nicks_setting(&self.excluded_nicks);

        SettingsWindow::builder(AddonName::BetterGroupInvites)
            .header(settings_window_header)
            .content(settings_window_content)
            .build()
    }
}

impl ActiveSettings {
    fn render(&'static self, settings: &'static Settings) -> JsResult<Dom> {
        let decor = HeaderDecor::builder()
            .push_left(decors::OpacityToggle::new())
            .push_left(
                decors::SettingsButton::builder()
                    .hide_window_on_click(false)
                    .on_click(Settings::shake_on_unsaved(Self::WINDOW_TYPE))
                    .build(),
            )
            .push_right(decors::CloseButton::new())
            .push_right(decors::CollapseButton::new())
            .build();
        let addon_window_header = WindowHeader::new(decor);

        let addon_window_content = WindowContent::builder()
            .invite_with_professions_setting(&self.with_prof)
            .invite_with_nick_setting(&self.with_nicks)
            .invite_with_lvl_setting(&self.with_lvl)
            .invite_buttons(&self, &settings);

        AddonWindow::builder(AddonName::BetterGroupInvites)
            .header(addon_window_header)
            .content(addon_window_content)
            .build()
    }
}

pub(super) fn init(
    settings: &'static Settings,
    active_settings: &'static ActiveSettings,
) -> JsResult<()> {
    let _settings_window_handle = WINDOWS_ROOT
        .try_append_dom(settings.render()?)
        .ok_or_else(|| err_code!())?;

    let _addon_window_handle = WINDOWS_ROOT
        .try_append_dom(active_settings.render(settings)?)
        .ok_or_else(|| err_code!())?;

    Ok(())
}
