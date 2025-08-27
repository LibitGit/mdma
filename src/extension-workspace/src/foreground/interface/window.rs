use common::{debug_log, map_err};
use dominator::{Dom, DomBuilder, html};
use futures_signals::signal::{Mutable, Signal};
use wasm_bindgen::intern;
use web_sys::HtmlElement;

use crate::{console_error, s, string};

use super::WindowType;

/// Struct for creating a new addon window with the given header name.
#[derive(Clone)]
pub(crate) struct MdmaWindow {
    name: &'static str,
    active: Mutable<bool>,
}

impl MdmaWindow {
    pub(crate) fn new(window_type: WindowType) -> Self {
        let window_name = match window_type {
            WindowType::Addons => "Dodatki",
            WindowType::Console => "Konsola",
            WindowType::Mdma => "MDMA",
            WindowType::Settings => "Ustawienia",
        };
        Self {
            name: window_name,
            active: Mutable::new(true),
        }
    }

    pub(crate) fn render(state: &mut Self, content: Dom) -> Dom {
        html!(s!("div"), {
            .class(s!("interface-window"))
            .class_signal("active", state.active.signal())
            .class(state.name.to_lowercase())
            .style_signal("left", Self::random_position(false))
            .style_signal("top", Self::random_position(true))
            .children(&mut [
                html!(s!("div"), {
                    .class(s!("mdma-header"))
                    .child(html!(s!("span"), {
                        .class(s!("mdma-title"))
                        .text(state.name)
                    }))
                    .apply(|interface_builder| {
                        Self::create_header_elements(state, interface_builder)
                    })
                }),
                content
            ])
        })
    }

    /// Creates a random position signal in pixels.
    fn random_position(allow_smaller_than_viewport: bool) -> impl Signal<Item = String> {
        use super::INTERFACE_VISIBLE;
        use dominator::window_size;
        use futures_signals::map_ref;
        use js_sys::Math;

        map_ref! {let window_size_signal = window_size(), let is_active_signal = INTERFACE_VISIBLE.with(|visible| visible.signal()) => {
            match is_active_signal {
                true => string!("0px"),
                false => {
                    let (width, height) = (window_size_signal.width, window_size_signal.height);
                    let radius = width.max(height) * 1.5 + width.max(height) * Math::random();
                    let sign = if Math::random() <= 0.5 { -1.0 } else { 1.0 };
                    let displacement = match allow_smaller_than_viewport {
                        true => radius * sign * Math::random(),
                        false => radius * sign
                    };
                    let mut displacement = displacement.trunc().to_string();
                    displacement.push_str(intern(s!("px")));

                    displacement
                }
            }
        }}
    }

    /// Creates custom window control elements.
    fn create_header_elements(
        state: &MdmaWindow,
        interface_builder: DomBuilder<HtmlElement>,
    ) -> DomBuilder<HtmlElement> {
        use dominator::{clone, events::Click};

        match state.name == s!("Konsola") {
            false => interface_builder.child(html!(s!("span"), {
                .class(s!("mdma-dropdown-toggle"))
                .event(clone!(state => move |_: Click| {
                    state.active.set(!state.active.get())
                }))
            })),
            true => interface_builder.child(html!(s!("span"), {
                .class(s!("mdma-copy-button"))
                .text(s!("📋"))
                .event(|_: Click| {
                    wasm_bindgen_futures::spawn_local(async {
                        Self::copy_console_text().await
                    });
                })
            })),
        }
    }

    /// Copies parsed console logs.
    async fn copy_console_text() {
        use wasm_bindgen_futures::JsFuture;

        use crate::utils::window;

        use super::CONSOLE_LOGS;

        let console_logs = CONSOLE_LOGS.with_borrow_mut(|logs| {
            logs.make_contiguous()
                .sort_by(|a, b| b.created_at.partial_cmp(&a.created_at).unwrap());

            logs.as_slices()
                .0
                .iter()
                .fold(String::new(), |mut accumulator, elem| {
                    if let Some(log) = elem.inner.as_string() {
                        accumulator.push_str(&log);
                        accumulator.push('\n');
                    } else if let Some(error_code) = elem.inner.as_f64() {
                        accumulator.push_str(&format!(
                            "{{\"mdma_err\": {error_code}, \"ev\": \"{:?}\"}}\n",
                            elem.created_at
                        ))
                    } else {
                        debug_log!(&elem.inner);
                    }
                    accumulator
                })
        });

        if let Err(err) = JsFuture::from(window().navigator().clipboard().write_text(&console_logs))
            .await
            .map_err(map_err!())
        {
            console_error!(err);
        }
    }
}
