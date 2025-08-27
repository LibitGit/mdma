use std::ops::Deref;

use common::{
    UnwrapJsExt, debug_log, err_code,
    messaging::{LogOutDetails, prelude::*},
};
use discard::Discard;
use dominator::{
    Dom, DomBuilder,
    animation::{MutableAnimation, Percentage},
    events::Click,
    html, styles, stylesheet,
};
use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
};
pub use obfstr::obfstr as s;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlAnchorElement, HtmlButtonElement, HtmlDivElement};

pub mod dispatcher;
mod exports;

thread_local! {
    static SPIN_ANIMATION: MutableAnimation = MutableAnimation::new(1000.0);
}

#[wasm_bindgen(start)]
async fn main() -> Result<(), JsValue> {
    init_global_stylesheets();

    let spinner_before_init = DomBuilder::<HtmlDivElement>::new_html("div")
        .class("popup")
        .child(Popup::spinner())
        .into_dom();
    let spinner_handle = dominator::append_dom(&dominator::body(), spinner_before_init);

    Message::builder(Task::UserData, Target::Background, MessageKind::Request)
        .build()
        .execute()
        .await?;

    let mut dispatcher = dispatcher::Dispatcher::init()?;
    let msg = dispatcher.recv().await.ok_or_else(|| err_code!())?;
    let validator = MessageValidator::builder(Target::Background)
        .task(Task::UserData)
        .kind(MessageKind::Response)
        .build();

    validator.validate(&msg)?;

    let state = msg
        .popup
        .ok_or_else(|| err_code!())?
        .state
        .ok_or_else(|| err_code!())?;
    let user_data = msg.username.map(|nick| UserData::new(nick, msg.premium));
    let popup: &'static _ = Box::leak(Box::new(Popup::new(state, user_data)));

    spinner_handle.discard();

    dominator::append_dom(&dominator::body(), popup.render());

    dispatcher.spawn_event_loop(popup);

    Ok(())
}

#[derive(Debug, Serialize)]
struct DateLocaleOptions<'a> {
    year: &'a str,
    month: &'a str,
    day: &'a str,
    hour: &'a str,
    minute: &'a str,
    second: &'a str,
}

impl Default for DateLocaleOptions<'_> {
    fn default() -> Self {
        Self {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LoadingReason {
    LoggingIn,
    LoggingOut,
}

#[derive(Debug)]
struct LoadingQueue(Mutable<Vec<LoadingReason>>);

impl LoadingQueue {
    fn new(state: PopupState) -> Self {
        let initial = match state {
            PopupState::LoggedIn | PopupState::LoggedOut | PopupState::JoinDiscord => vec![],
            PopupState::LoggingIn => vec![LoadingReason::LoggingIn],
            PopupState::LoggingOut => vec![LoadingReason::LoggingOut],
        };
        Self(Mutable::new(initial))
    }

    fn remove_reason(&self, reason: LoadingReason) -> Option<LoadingReason> {
        let mut queue_lock = self.lock_mut();

        queue_lock
            .iter()
            .position(|r| *r == reason)
            .map(|index| queue_lock.swap_remove(index))
    }

    fn insert_reason(&self, reason: LoadingReason) {
        self.lock_mut().push(reason);
    }

    fn signal_on(&self, reason: LoadingReason) -> impl Signal<Item = bool> + use<> {
        self.signal_ref(move |queue| queue.contains(&reason))
            .dedupe()
    }
}

impl Deref for LoadingQueue {
    type Target = Mutable<Vec<LoadingReason>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
struct Popup {
    user: Mutable<Option<UserData>>,
    loading: LoadingQueue,
    message: Mutable<Option<DisplayMessage>>,
}

impl Popup {
    fn new(state: PopupState, user_data: Option<UserData>) -> Self {
        Self {
            user: Mutable::new(user_data),
            loading: LoadingQueue::new(state),
            message: Mutable::new(None),
        }
    }

    fn render(&'static self) -> Dom {
        DomBuilder::<HtmlDivElement>::new_html("div")
            .class("popup")
            .child_signal(self.init_messages())
            .child_signal(self.init_spinner())
            .child_signal(self.render_login_button())
            .child_signal(self.render_login_status())
            .child_signal(self.render_welcome())
            .child_signal(self.render_logout_buttons())
            .into_dom()
    }

    fn init_messages(&self) -> impl Signal<Item = Option<Dom>> + 'static {
        self.message.signal_ref(|msg| {
            let msg = msg.as_ref()?;

            let dom = DomBuilder::<HtmlDivElement>::new_html("div")
                .class(msg.kind.to_class())
                .apply(|b| match msg.kind == DisplayMessageKind::JoinDiscord {
                    false => b.text(&msg.txt),
                    true => b
                        .text("Dołącz na ")
                        .child(
                            DomBuilder::<HtmlAnchorElement>::new_html("a")
                                .text("nasz discord")
                                .attr("href", "https://discord.gg/dodatki-margonem")
                                .attr("target", "_blank")
                                .into_dom(),
                        )
                        .text(" i spróbuj ponownie!"),
                })
                .into_dom();

            Some(dom)
        })
    }

    fn init_spinner(&'static self) -> impl Signal<Item = Option<Dom>> + 'static {
        self.loading
            .signal_ref(|queue| !queue.is_empty())
            .map(|loading| loading.then(|| Self::spinner()))
    }

    fn spinner() -> Dom {
        DomBuilder::<HtmlDivElement>::new_html("div")
            .future(SPIN_ANIMATION.with(|anim| anim.signal()).for_each(|t| {
                SPIN_ANIMATION.with(|anim| match t {
                    Percentage::START => anim.animate_to(Percentage::END),
                    Percentage::END => anim.jump_to(Percentage::START),
                    _ => (),
                });

                async {}
            }))
            .class("loading-spinner")
            .style_signal(
                "transform",
                SPIN_ANIMATION
                    .with(|anim| anim.signal())
                    .map(|t| Some(format!("rotate({}deg)", t.range_inclusive(0.0, 360.0)))),
            )
            .into_dom()
    }

    fn render_login_button(&'static self) -> impl Signal<Item = Option<Dom>> + 'static {
        map_ref! {
            let user_not_logged_in = self.user.signal_ref(Option::is_none),
            let logging_in = self.loading.signal_on(LoadingReason::LoggingIn) => {
                *user_not_logged_in && !logging_in
            }
        }
        .map(move |display_button| {
            display_button.then(|| {
                DomBuilder::<HtmlButtonElement>::new_html("button")
                    .class("login-btn")
                    .text("Logowanie")
                    .event(move |_: Click| {
                        wasm_bindgen_futures::spawn_local(async move {
                            if let Err(err_code) = self.start_login().await {
                                console_error!(err_code)
                            }
                        });
                    })
                    .into_dom()
            })
        })
    }

    async fn start_login(&self) -> Result<(), JsValue> {
        if self.user.take().is_some() {
            debug_log!("user was already logged in!")
        }

        self.message.take();
        self.loading.insert_reason(LoadingReason::LoggingIn);

        Message::builder(Task::OAuth2, Target::Background, MessageKind::Request)
            .build()
            .execute()
            .await
    }

    fn render_login_status(&self) -> impl Signal<Item = Option<Dom>> + 'static {
        self.loading
            .signal_on(LoadingReason::LoggingIn)
            .map(|logging_in| {
                logging_in.then(|| {
                    DomBuilder::<HtmlDivElement>::new_html("div")
                        .class("status")
                        .text("Rozpoczęto logowanie...")
                        .child(html!("br", {}))
                        .child(html!("small", {
                            .text("Dokończ logowanie wewnątrz okna Discord!")
                        }))
                        .into_dom()
                })
            })
    }

    fn render_welcome(&self) -> impl Signal<Item = Option<Dom>> + 'static {
        self.user.signal_ref(|user_data| {
            let user_data = user_data.as_ref()?;
            let username = user_data.nick.as_str();
            let premium = user_data.premium.as_ref();

            let welcome = DomBuilder::<HtmlDivElement>::new_html("div")
                .class(["status", "success"])
                .child(html!("strong", {
                    .text(&format!("Witaj {username}!"))
                }))
                .child(html!("br", {}))
                .child(
                    DomBuilder::<HtmlDivElement>::new_html("div")
                        .class("info-box")
                        .text(match premium.is_some() {
                            true => "Premium aktywne!",
                            false => "Premium: brak :(",
                        })
                        .apply_if(premium.is_some(), |b| {
                            b.child(html!("br", {}))
                                .text(&format!(
                                        "Ważne do: {}",
                                        js_sys::Date::new(&JsValue::from_f64(
                                            (premium.unwrap().exp * 1000) as f64
                                        ))
                                        .to_locale_date_string(
                                            "pl-PL",
                                            &serde_wasm_bindgen::to_value(
                                                &DateLocaleOptions::default()
                                            )
                                            .unwrap_js(),
                                        )
                                        .as_string()
                                        .unwrap_js(),
                                    ))
                                .child(html!("br", {}))
                                .child(html!("br", {}))
                                .text(match premium.unwrap().neon {
                                    true => "✅ Neon Bohatera",
                                    false => "❌ Neon Bohatera",
                                })
                                .child(html!("br", {}))
                                .text(match premium.unwrap().animation {
                                    true => "✅ Animacja Chodzenia",
                                    false => "❌ Animacja Chodzenia",
                                })
                        })
                        .into_dom(),
                )
                .into_dom();

            Some(welcome)
        })
    }

    fn render_logout_buttons(&'static self) -> impl Signal<Item = Option<Dom>> + use<> {
        self.user.signal_ref(move |user_data| {
            user_data.is_some().then(|| {
                DomBuilder::<HtmlDivElement>::new_html("div")
                    .class("logout-btns")
                    .child(self.render_logout_button("Wyloguj", false))
                    .child(self.render_logout_button("Wyloguj ze wszystkich urządzeń", true))
                    .into_dom()
            })
        })
    }

    fn render_logout_button(&'static self, text: &str, all_devices: bool) -> Dom {
        DomBuilder::<HtmlButtonElement>::new_html("button")
            .class("logout-btn")
            .text(text)
            .event(move |_: Click| {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(err_code) = self.log_out(all_devices).await {
                        console_error!(err_code);
                    }
                })
            })
            .into_dom()
    }

    async fn log_out(&self, all_devices: bool) -> Result<(), JsValue> {
        self.loading.insert_reason(LoadingReason::LoggingOut);

        Message::builder(Task::LogOut, Target::Background, MessageKind::Request)
            .log_out(LogOutDetails::new(all_devices))
            .build()
            .execute()
            .await?;

        self.user.take();
        self.loading.remove_reason(LoadingReason::LoggingOut);

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UserData {
    nick: String,
    premium: Option<Premium>,
}

impl UserData {
    fn new(nick: String, premium: Option<Premium>) -> Self {
        Self { nick, premium }
    }
}

#[derive(Debug, PartialEq)]
pub struct DisplayMessage {
    pub kind: DisplayMessageKind,
    pub txt: String,
}

impl DisplayMessage {
    pub fn success(txt: String) -> Self {
        Self {
            kind: DisplayMessageKind::Success,
            txt,
        }
    }

    pub fn error(txt: String) -> Self {
        Self {
            kind: DisplayMessageKind::Error,
            txt,
        }
    }

    pub fn join_discord() -> Self {
        Self {
            kind: DisplayMessageKind::JoinDiscord,
            txt: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMessageKind {
    Success,
    Error,
    JoinDiscord,
}

impl DisplayMessageKind {
    fn to_class(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Error => "error",
            Self::JoinDiscord => "warn",
        }
    }
}

#[macro_export]
macro_rules! console_error {
    () => {{
        use ::wasm_bindgen::{JsValue, intern};

        let error_code = ::common::err_code!();
        ::web_sys::console::error_5(
            &JsValue::from_str(intern($crate::s!("%c MDMA %c %c Rust "))),
            &JsValue::from_str(intern($crate::s!(
                "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
            ))),
            &JsValue::from_str(intern($crate::s!(""))),
            &JsValue::from_str(intern($crate::s!(
                "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
            ))),
            &error_code,
        );
        // $crate::utils::logging::__internal_console_error(error_code);
    }};
    ($error_code:expr) => {{
        use ::wasm_bindgen::{JsValue, intern};

        // let _ = $crate::bindings::message(intern($crate::s!("[MDMA::RS] Wystąpił
        // błąd!")));
        ::web_sys::console::error_5(
            &JsValue::from_str(intern($crate::s!("%c MDMA %c %c Rust "))),
            &JsValue::from_str(intern($crate::s!(
                "background: #8CAAEE; color: black; font-weight: bold; border-radius: 5px;"
            ))),
            &JsValue::from_str(intern($crate::s!(""))),
            &JsValue::from_str(intern($crate::s!(
                "background: #CE412B; color: black; font-weight: bold; border-radius: 5px;"
            ))),
            &$error_code,
        );
        // $crate::utils::logging::__internal_console_error($error_code);
    }};
}

fn init_global_stylesheets() {
    stylesheet!(".loading-spinner", {
        .styles! {
            width: "40px",
            height: "40px",
            border: "3px solid #f3f3f3",
            "border-top": "3px solid #7289DA",
            "border-radius": "50%",
        }
    });
    stylesheet!("body", {
        .styles! {
            width: "400px",
            height: "500px",
            margin: "0",
            padding: "20px",
            background: "hsl(226, 23%, 11%)",
        }
    });
    stylesheet!(".status", {
        .styles! {
            padding: "12px",
            "border-radius": "4px",
            "text-align": "center",
            width: "100%",
            color: "white",
        }
    });
    stylesheet!(".status .info-box", {
        .styles! {
            "margin-top": "10px",
            padding: "12px",
            "text-align": "left",
        }
    });
    stylesheet!(".success", {
        .styles! {
            background: "#e6ffe6",
            color: "#006600",
            padding: "5px",
            "border-radius": "5px",
        }
    });
    stylesheet!(".error", {
        .styles! {
            background: "#ffe6e6",
            color: "#660000",
            padding: "5px",
            "border-radius": "5px",
        }
    });
    stylesheet!(".warn", {
        .styles! {
            background: "#FFF3CD",
            color: "#856404",
            padding: "5px",
            "border-radius": "5px",
        }
    });
    stylesheet!(".popup", {
        .styles! {
            "display": "flex",
            "flex-direction": "column",
            "align-items": "center",
            gap: "20px",
            height: "100%",
        }
    });
    stylesheet!(".login-btn", {
        .styles! {
            width: "200px",
            padding: "12px",
            background: "#7289DA",
            color: "white",
            border: "none",
            "border-radius": "4px",
            cursor: "pointer",
            "font-size": "14px",
            transition: "background 0.2s",
        }
    });
    stylesheet!(".login-btn:hover", {
        .style("background", "#5b73c7")
    });
    stylesheet!(".logout-btns", {
        .styles! {
            "margin-top": "auto",
            display: "flex",
            width: "100%",
            "flex-direction": "row",
            "justify-content": "space-around",
            "align-items": "center",
        }
    });
    stylesheet!(".logout-btn", {
        .styles! {
            padding: "12px 20px",
            background: "#dc3545",
            color: "white",
            border: "none",
            "border-radius": "4px",
            cursor: "pointer",
            "font-size": "14px",
            transition: "background 0.2s",
        }
    });
    stylesheet!(".logout-btn:hover", {
        .style("background", "#b02a37")
    });
}
