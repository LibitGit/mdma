mod addons;
pub(crate) mod dom_utils;
pub(crate) mod tips_parser;
mod window;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::iter;
use std::ops::Deref;
use std::thread::LocalKey;

use dominator::events::{MouseButton, MouseDown, Wheel};
use common::messaging::prelude::*;
use dominator::{Dom, DomBuilder, DomHandle, EventOptions, html, with_cfg};
use futures_signals::signal::{Mutable, Signal};
use futures_signals::signal_vec::MutableVec;
#[cfg(feature = "ni")]
use js_sys::JsString;
use wasm_bindgen::{JsCast, JsValue, intern};
use web_sys::{HtmlDivElement, ShadowRoot};

#[cfg(feature = "ni")]
use crate::addon_window::ITEM_FRAME;
use crate::addon_window::MdmaAddonWindow;
use crate::addon_window::ui_components::{Checkbox, Input, InputType};
use crate::globals::addons::AddonData;
use crate::globals::{ManagerGlobals, ManagerHotkey};
use crate::prelude::*;

thread_local! {
    pub(crate) static CONSOLE_LOGS: RefCell<VecDeque<ConsoleLog>> = RefCell::new(VecDeque::with_capacity(500));
    pub(crate) static CONSOLE_MESSAGES: MutableVec<ConsoleLog> = MutableVec::with_capacity(500);
    pub(crate) static INTERFACE_VISIBLE: Mutable<bool> = Mutable::new(false);
    pub(crate) static WINDOWS_ROOT: RefCell<Option<ShadowRoot>> = const { RefCell::new(None) };
    pub(crate) static INTERFACE_ROOT: RefCell<Option<ShadowRoot>> = const { RefCell::new(None)};
    pub(crate) static WIDGET_ROOT: RefCell<Option<ShadowRoot>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ConsoleLogTypes {
    Error,
    Message,
    CommunicationData,
}

#[derive(Debug, Clone)]
pub(crate) struct ConsoleLog {
    type_: ConsoleLogTypes,
    inner: JsValue,
    created_at: Option<f64>,
}

impl ConsoleLog {
    pub(crate) fn new(type_: ConsoleLogTypes, inner: JsValue, created_at: Option<f64>) -> Self {
        Self {
            type_,
            inner,
            created_at,
        }
    }
}

pub(crate) trait ThreadLocalShadowRoot {
    fn try_append_dom(&'static self, dom: Dom) -> Option<DomHandle>;
}

impl ThreadLocalShadowRoot for LocalKey<RefCell<Option<ShadowRoot>>> {
    fn try_append_dom(&'static self, dom: Dom) -> Option<DomHandle> {
        self.with_borrow(|root| root.as_ref().map(|root| dominator::append_dom(root, dom)))
    }
}

pub(crate) fn get_windows_stylesheet() -> Dom {
    html!(s!("link"), {
        .attr(s!("rel"), s!("stylesheet"))
        // .attr(s!("href"), option_env!("WINDOWS_STYLESHEET").unwrap_or(&format!("http://localhost:3000/css/windows.css?c={}", js_sys::Math::random())))
        .attr(s!("href"), option_env!("WINDOWS_STYLESHEET").unwrap_or(&format!("http://localhost:3000/css/windows.css")))
        // .attr(s!("href"), "https://libit.ovh/css/windows.css")
    })
}

pub(crate) fn init_windows_layer() -> JsResult<()> {
    use crate::addon_window::BuildWindowsLayerEvents;

    #[cfg(feature = "ni")]
    {
        observe_change_frames()?;
        observe_change_overlays()?;
    }

    #[cfg(not(feature = "ni"))]
    {
        if let Some(loading) = document().get_element_by_id("loading") {
            let loading = loading.unchecked_into::<HtmlDivElement>();
            loading.style().set_property("z-index", "479")?;
            wasm_bindgen_futures::spawn_local(async move {
                wait_for_without_timeout(can_send_idle_request, 500).await;
                _ = loading.style().remove_property("z-index");
            });
        } else {
            debug_log!("loading element is None!");
        }
    }

    let mut event_options = EventOptions::bubbles();
    event_options.preventable = true;
    let windows_layer = DomBuilder::<HtmlDivElement>::new_html(s!("div"));

    #[cfg(not(feature = "ni"))]
    let windows_layer = windows_layer.event_with_options(
        &EventOptions::preventable(),
        |event: dominator::events::ContextMenu| event.prevent_default(),
    );
    let shadow = windows_layer
        .__internal_shadow_root(web_sys::ShadowRootMode::Closed)
        .apply(|root_builder| {
            WINDOWS_ROOT.with_borrow_mut(|windows_root| {
                *windows_root = Some(root_builder.__internal_element())
            });

            root_builder
        })
        .build_windows_layer_events()
        .child(get_windows_stylesheet())
        .event_with_options(&event_options, |event: Wheel| event.stop_propagation())
        .after_inserted(|_| debug_log!("WINDOWS ROOT INSERTED"));
    let windows_layer = windows_layer
        .__internal_transfer_callbacks(shadow)
        .into_dom();
    let alerts_layer = match cfg!(feature = "ni") {
        true => document()
            .get_elements_by_class_name(s!("alerts-layer"))
            .get_with_index(0)
            .ok_or_else(|| err_code!())?,
        false => document()
            .document_element()
            .ok_or_else(|| err_code!())?
            .unchecked_into(),
    };

    dominator::append_dom(&alerts_layer, windows_layer);

    Ok(())
}

/// No frame switching addon on si
#[cfg(feature = "ni")]
fn observe_change_frames() -> JsResult<()> {
    let ground_items = get_engine()
        .map()
        .ok_or_else(|| err_code!())?
        .ground_items()
        .ok_or_else(|| err_code!())?;
    let original_change_frames = ground_items
        .get_change_frames()
        .ok_or_else(|| err_code!())?;

    let new_change_frames = closure!(
        { let ground_items = ground_items.clone() },
        move |image_url: JsString, image_offset: JsValue| -> DefaultResult {
            //debug_log!("ITEM FRAMES UPDATED:", &image_url);

            if image_url.starts_with("..", 0) {
                ITEM_FRAME.with(|frame| frame.highlight_image_url.set_neq(None));
            } else {
                ITEM_FRAME.with(|frame| frame.highlight_image_url.set_neq(image_url.as_string()));
            }
            ITEM_FRAME.with(|frame| frame.highlight_offset.set_neq(image_offset.unchecked_into_f64()));

            original_change_frames.call2(&ground_items, &image_url, &image_offset)
        }
    );

    ground_items.set_change_frames(&new_change_frames);

    Ok(())
}

/// No overlay switching addon on si
#[cfg(feature = "ni")]
fn observe_change_overlays() -> JsResult<()> {
    let ground_items = get_engine()
        .map()
        .ok_or_else(|| err_code!())?
        .ground_items()
        .ok_or_else(|| err_code!())?;
    let original_change_overlays = ground_items
        .get_change_overlays()
        .ok_or_else(|| err_code!())?;
    let new_change_overlays = closure!(
        { let ground_items = ground_items.clone() },
        move |image_url: JsString, bg_pos_y: JsValue| -> DefaultResult {
            //debug_log!("ITEM overlays UPDATED:", &image_url);

            if image_url.length() == 0 {
                ITEM_FRAME.with(|frame| frame.overlay_image_url.set_neq(None));
            } else if image_url.starts_with("/", 0) {
                ITEM_FRAME.with(|frame| frame.overlay_image_url.set_neq(image_url.as_string().map(|url| format!("https://cronus.margonem.com{url}?v=1737721312664"))));
            } else {
                ITEM_FRAME.with(|frame| frame.overlay_image_url.set_neq(image_url.as_string()));
            }

            original_change_overlays.call2(&ground_items, &image_url, &bg_pos_y)
        }
    );

    ground_items.set_change_overlays(&new_change_overlays);

    Ok(())
}

pub(crate) fn init_interface(manager_globals: &'static ManagerGlobals) -> JsResult<()> {
    let document_element = document().document_element().ok_or_else(|| err_code!())?;
    let stylesheet_link = html!(s!("link"), {
        .attr(s!("rel"), s!("stylesheet"))
        .attr(s!("href"), option_env!("INTEFACE_STYLESHEET").unwrap_or(&format!("http://localhost:3000/css/interface.css?c={}", js_sys::Math::random())))
        // .attr(s!("href"), "https://libit.ovh/css/interface.css")
    });
    let interface_layer = DomBuilder::<HtmlDivElement>::new_html(s!("div"));
    let shadow = interface_layer
        .__internal_shadow_root(web_sys::ShadowRootMode::Closed)
        .apply(|root_builder| {
            INTERFACE_ROOT.with_borrow_mut(|interface_root| {
                *interface_root = Some(root_builder.__internal_element())
            });
            root_builder
        })
        .child(stylesheet_link)
        .child(render_interface(manager_globals))
        .after_inserted(|_| {
            logging::console_log(JsValue::from_str(s!("Interface init")));
        });
    let interface_layer = interface_layer
        .__internal_transfer_callbacks(shadow)
        .into_dom();

    dominator::append_dom(&document_element, interface_layer);

    Ok(())
}

fn render_interface(manager_globals: &'static ManagerGlobals) -> Dom {
    use dominator::events::KeyDown;
    use window::MdmaWindow;

    use crate::utils::window_events;

    let chat_visible: &'static Mutable<Option<&str>> = Box::leak(Box::new(Mutable::new(None)));

    #[cfg(feature = "ni")]
    if let Err(err) = observe_chat(&chat_visible) {
        console_error!(err);
    }

    let mdma_window =
        MdmaWindow::render(&mut MdmaWindow::new(WindowType::Mdma), get_mdma_content());
    let settings_window = MdmaWindow::render(
        &mut MdmaWindow::new(WindowType::Settings),
        get_settings_content(manager_globals),
    );
    let addons_window = MdmaWindow::render(
        &mut MdmaWindow::new(WindowType::Addons),
        get_addons_content(),
    );
    let console_window = MdmaWindow::render(
        &mut MdmaWindow::new(WindowType::Console),
        get_console_content(),
    );

    html!(s!("div"), {
        .class(s!("interface-layer"))
        .class_signal("active", INTERFACE_VISIBLE.with(|visible| visible.signal()))
        .style_signal("left", chat_visible.signal())
        .children([mdma_window, settings_window, addons_window, console_window]
        )
        .global_event_with_options(&EventOptions::preventable(), |event: KeyDown| {
            match window_events::validate_keydown_event(&event, manager_globals.hotkey.lock_ref().deref()) {
                Ok(key_matched) => if !key_matched { return },
                Err(err) => return console_error!(err),
            };

            event.prevent_default();
            event.stop_propagation();
            INTERFACE_VISIBLE.with(|visible| visible.set(!visible.get()));
        })
    })
}

#[cfg(feature = "ni")]
fn observe_chat(chat_visible: &'static Mutable<Option<&'static str>>) -> JsResult<()> {
    use web_sys::{MutationObserver, MutationObserverInit, MutationRecord};
    let mutation_callback = closure!(move |mutation_list: Vec<MutationRecord>| -> JsResult<()> {
        for mutation in mutation_list {
            if mutation
                .attribute_name()
                .is_some_and(|name| name != s!("class"))
            {
                continue;
            }

            let game_window_positioner = mutation
                .target()
                .ok_or_else(|| err_code!())?
                .dyn_into::<HtmlDivElement>()
                .map_err(map_err!())?;

            match game_window_positioner
                .class_list()
                .contains(s!("chat-size-1"))
            {
                true => chat_visible.set(Some("255px")),
                false => chat_visible.set(None),
            }
        }

        Ok(())
    });
    let mutation_observer = MutationObserver::new(&mutation_callback).map_err(map_err!())?;
    let mutation_observer_init = MutationObserverInit::new();
    mutation_observer_init.set_attributes(true);

    let game_window_positioner = document()
        .get_elements_by_class_name(s!("game-window-positioner default-cursor"))
        .get_with_index(0)
        .ok_or_else(|| err_code!())?;

    mutation_observer
        .observe_with_options(&game_window_positioner, &mutation_observer_init)
        .map_err(map_err!())
}

pub(crate) enum WindowType {
    Mdma,
    Settings,
    Addons,
    Console,
}

fn get_addons_content() -> Dom {
    let addons_content = Addons::get()
        .iter()
        .filter_map(|addon_option| addon_option)
        .map(|(name, data)| AddonData::render(data, name));

    html!(s!("div"), {
        .class(s!("mdma-content"))
        .class(s!("mdma-addons-content"))
        .children(addons_content)
    })
}

fn get_console_content() -> Dom {
    use futures_signals::signal_vec::SignalVecExt;

    let msgs = CONSOLE_MESSAGES.with(|logs| logs.signal_vec_cloned().map(get_console_text_div));

    html!(s!("div"), {
        .attr(s!("id"), s!("mdma-console-content"))
        .class(s!("mdma-content"))
        .children_signal_vec(msgs)
    })
}

fn get_console_text_div(log: ConsoleLog) -> Dom {
    html!(s!("div"), {
        .class(s!("mdma-console-text"))
        .apply_if(log.type_ == ConsoleLogTypes::Error, |builder| {
            builder.style(s!("color"), s!("red"))
        })
        .apply(|builder| match log.inner.as_string() {
            Some(text) => builder.text(&text),
            None => builder.text(intern(s!("An error has occured!"))),
        })
    })
}

enum MdmaContentButton {
    GitHub,
    Discord,
}

fn get_mdma_content() -> Dom {
    let version_elem = html!(s!("span"), {
        .class(s!("mdma-version"))
        .text(&(string!("Wersja: ") + s!(env!("CARGO_PKG_VERSION"))))
    });
    //let build_elem = html!(s!("span"), {
    //    .class(s!("mdma-build"))
    //    .text(s!("Build: TBA")) // TODO: get build from crate data
    //});
    //let search_bar = html!(s!("input"), {
    //    .class(s!("mdma-search"))
    //    .attr(s!("type"), s!("text"))
    //    .attr(s!("placeholder"), s!("Search"))
    //});

    html!(s!("div"), {
        .class(s!("mdma-content"))
        .children([
            version_elem,
            //build_elem,
            mdma_content_button(MdmaContentButton::GitHub, s!("GitHub Repo"), "https://github.com/LibitGit/MDMA"),
            mdma_content_button(MdmaContentButton::Discord, s!("Discord Server"), "https://libit.ovh/mdma"),
            //search_bar
        ])
    })
}

fn mdma_content_button(button_type: MdmaContentButton, inner_text: &str, url: &'static str) -> Dom {
    use crate::utils::window;
    use dominator::events::Click;

    html!(s!("button"), {
        .class(match button_type {
            MdmaContentButton::GitHub => "mdma-github-button",
            MdmaContentButton::Discord => "mdma-discord-button",
        })
        .text(inner_text)
        .event(|_: Click| {
            if let Err(err) =  window().open_with_url(url).map_err(map_err!()) {
                console_error!(err)
            }
        })
    })
}

pub(super) fn render_unauthorized() -> JsResult<()> {
    init_windows_layer()?;
    let widget = render_widget(false, futures_signals::signal::always(true));
    let parent = match cfg!(feature = "ni") {
        true => document()
            .get_elements_by_class_name(s!("bags-navigation"))
            .get_with_index(0)
            .unwrap_js(),
        false => document().get_element_by_id("panel").unwrap_js(),
    };
    dominator::append_dom(&parent, widget);

    Ok(())
}

pub(crate) fn render_widget(
    toggle_manager: bool,
    active_signal: impl Signal<Item = bool> + 'static,
) -> Dom {
    use self::tips_parser::tip;
    use dominator::events::{MouseEnter, MouseLeave};
    use dominator::{clone, shadow_root};
    use futures_signals::signal::SignalExt;
    use web_sys::ShadowRootMode;

    let mouse_on_widget = Mutable::new(false);

    html!(s!("div"), {
        .shadow_root!(ShadowRootMode::Closed => {
            .child(html!(s!("div"), {
                .class(s!("mdma-widget"))
                .style_signal("display", active_signal.map(|active| match active {
                    true => None,
                    false => Some("none"),
                }))
                .with_cfg!(feature = "ni", {
                    .style(s!("left"), s!("-40px"))
                })
                .with_cfg!(not(feature = "ni"), {
                    .style(s!("left"), s!("155px"))
                    .style(s!("top"), s!("467px"))
                    .event_with_options(
                        &EventOptions::preventable(),
                        |event: dominator::events::ContextMenu| event.prevent_default()
                    )
                })
                .style(s!("position"), s!("absolute"))
                .style(s!("height"), s!("36px"))
                .style(s!("width"), s!("35px"))
                .style(s!("border-radius"), s!("3px"))
                .style(s!("background"), s!("transparent url(\"https://i.imgur.com/TFpTRRF.png\") no-repeat"))
                .style(s!("background-size"), s!("35px 35px"))
                .style(s!("transition"), s!("all 0.3s ease"))
                .style_signal("background-color", mouse_on_widget.signal_cloned().map(|active| match active {
                    true => Some("rgba(0, 0, 0, 0.4)"),
                    false => None,
                }))
                .style_signal("box-shadow", mouse_on_widget.signal_cloned().map(|active| match active {
                    true => Some("0 0 1px #010101, 0 0 0 1px #ccc, 0 0 0 2px #0c0d0d, 1px 1px 2px 2px #0c0d0d66"),
                    false => None,
                }))
                .style_signal("cursor", mouse_on_widget.signal_cloned().map(|active| match active {
                    true => Some("url(\"https://experimental.margonem.pl/img/gui/cursor/5n.png\") 4 0, url(\"https://experimental.margonem.pl/img/gui/cursor/5n.cur\") 4 0, auto"),
                    false => None,
                }))
                .tip!({
                    .child(html!(intern(s!("i")), {
                        .text(intern(s!("Multipurpose Discord to Margonem Addons")))
                    }))
                    .child(html!(intern(s!("div")), {
                        .class(intern(s!("line")))
                    }))
                    .apply_if(toggle_manager, |builder| {
                        builder
                            .child(html!(intern(s!("br")), {}))
                            .text(intern(s!("LPM aby otworzyć okno zestawu dodatków")))
                    })
                    .child(html!(intern(s!("br")), {}))
                    .text(intern(s!("PPM aby otworzyć menu rozszerzenia")))
                })
                .event(move |event: MouseDown| {
                    if toggle_manager && event.button() == MouseButton::Left {
                        INTERFACE_VISIBLE.with(|visible| visible.set(!visible.get()));
                    } else if event.button() == MouseButton::Right {
                        wasm_bindgen_futures::spawn_local(async {
                            if let Err(err_code) = Port::send(&Message::new(Task::OpenPopup, Target::Background, MessageKind::Request)).await {
                                console_error!(err_code);
                            }
                        });
                    }
                })
                .event(clone!(mouse_on_widget => move |_:MouseEnter| {
                    mouse_on_widget.set(true);
                }))
                .event(clone!(mouse_on_widget => move |_: MouseLeave| {
                    mouse_on_widget.set(false);
                }))
            }))
        })
    })
}

pub const ALLOWED_CHARS: &str = "!@#$%^&*()_+-={}[]\\|;:'\",.<>/?`~€§";

fn get_settings_content(manager_globals: &'static ManagerGlobals) -> Dom {
    let widget = render_widget(true, manager_globals.widget_active.signal());
    let parent = match cfg!(feature = "ni") {
        true => document()
            .get_elements_by_class_name(s!("bags-navigation"))
            .get_with_index(0)
            .unwrap_js(),
        false => document().get_element_by_id("panel").unwrap_js(),
    };
    dominator::append_dom(&parent, widget);

    let hotkey_lock = manager_globals.hotkey.lock_ref();
    let display_value = hotkey_lock
        .ctrl_key
        .then_some("Ctrl")
        .into_iter()
        .chain(hotkey_lock.alt_key.then_some("Alt"))
        .chain(hotkey_lock.shift_key.then_some("Shift"))
        .chain(iter::once(hotkey_lock.value.as_str()))
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(" + ");

    drop(hotkey_lock);

    let manager_keybind_input = Input::builder()
        .class_list("m[0] p[0]")
        .input_type(InputType::keybind())
        .value(display_value)
        .maxlength("1")
        .on_key_down(move |event, input_elem| {
            if event.repeat() {
                return;
            }

            let value = event.key();
            let mut chars = value.chars();

            if chars
                .next()
                .is_none_or(|c| !is_from_polish_alphabet(c) && !ALLOWED_CHARS.contains(c))
            {
                event.prevent_default();
                event.stop_propagation();
                input_elem.blur().unwrap_js();
                return;
            }
            if chars.next().is_some() {
                event.prevent_default();
                event.stop_propagation();

                if value != intern("Tab") {
                    if value == intern("Escape")
                        || !matches!(value.as_str(), "Control" | "Alt" | "Shift")
                    {
                        input_elem.blur().unwrap_js();
                    }
                    return;
                }
            }

            let value = value
                .chars()
                .map(polish_to_ascii)
                .collect::<String>()
                .to_ascii_uppercase();
            let mut new_hotkey = ManagerHotkey::default();
            let display_value = event
                .ctrl_key()
                .then(|| {
                    new_hotkey.ctrl_key = true;
                    "Ctrl"
                })
                .into_iter()
                .chain(event.alt_key().then(|| {
                    new_hotkey.alt_key = true;
                    "Alt"
                }))
                .chain(event.shift_key().then(|| {
                    new_hotkey.shift_key = true;
                    "Shift"
                }))
                .chain(iter::once(value.as_str()))
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join(" + ");

            event.prevent_default();
            event.stop_propagation();
            input_elem.blur().unwrap_js();
            input_elem.set_value(&display_value);
            new_hotkey.value = value;
            manager_globals.hotkey.set(new_hotkey);
        });

    html!(s!("div"), {
        .class(s!("mdma-content"))
        .child(html!(s!("div"), {
            .class(s!("widget-label"))
            .checkbox(Checkbox::builder(manager_globals.widget_active.clone()).text(s!("Wyświetlaj widżet")))
        }))
        .child(html!(s!("div"), {
            .class(s!("widget-label"))
            .text("Otwórz manager")
            .input(manager_keybind_input)
        }))
    })
}
