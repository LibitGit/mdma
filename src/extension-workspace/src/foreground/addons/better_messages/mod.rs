mod html;

use std::cell::RefCell;
use std::rc::Rc;

use futures_signals::signal::Mutable;
use proc_macros::{ActiveSettings, Setting};

use crate::prelude::*;

const ADDON_NAME: AddonName = AddonName::BetterMessages;

#[derive(ActiveSettings)]
struct ActiveSettings {
    font: Font,
    color: Color,
    pointer_events: Mutable<bool>,
    #[setting(skip)]
    testing: Testing,
}

impl Default for ActiveSettings {
    fn default() -> Self {
        Self {
            font: Font::default(),
            color: Color::default(),
            pointer_events: Mutable::new(false),
            testing: Testing {
                interval_id: Mutable::new(None),
                active: Mutable::new(false),
            },
        }
    }
}

#[derive(Setting)]
struct Font {
    size: Mutable<u16>,
    active: Mutable<bool>,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            size: Mutable::new(15),
            active: Mutable::new(true),
        }
    }
}

#[derive(Setting)]
struct Color {
    code: Mutable<String>,
    active: Mutable<bool>,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            code: Mutable::new(string!("#000000")),
            active: Mutable::new(false),
        }
    }
}

struct Testing {
    interval_id: Mutable<Option<i32>>,
    active: Mutable<bool>,
}

impl ActiveSettings {
    fn init_tests(&self) -> JsResult<()> {
        use crate::bindings::message;
        use js_sys::Function;
        use wasm_bindgen::{closure::Closure, JsCast};

        message(s!(
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit..."
        ))
        .unwrap_js();

        let index = Rc::new(RefCell::new(0));
        let better_messages_test =
            Closure::<dyn FnMut()>::new(move || Self::better_messages_tests(&index))
                .into_js_value()
                .dyn_into::<Function>()
                .map_err(map_err!())?;
        let interval_id = window()
            .set_interval_with_callback_and_timeout_and_arguments_0(&better_messages_test, 2000)
            .map_err(map_err!())?;

        self.testing.interval_id.set(Some(interval_id));

        Ok(())
    }

    fn better_messages_tests(index: &Rc<RefCell<i32>>) {
        use crate::bindings::message;

        let mut idx = index.borrow_mut();
        *idx += 1;
        let jd = match *idx {
            10 => message(s!("Achaja to gruba kvrwa i dz1wka")),
            11 => message(s!("Thinker - sławomir koczy - to szajbus i d3bil")),
            12 => message(s!(
                "cala administracja bierze glebokie chvje w gardlo, psy j3bane"
            )),
            13 => message(s!("sprzedam konto za psc, 2g tez sprzedam za psc jak co")),
            14 => message(s!("Dobra koniec żartów testuj sobie dalej :))")),
            25 => message(s!("Serio jeszcze testujesz?!")),
            26 => message(s!("Ile można zmieniać kolorki...")),
            27 => message(s!("Skończ już te testy, nic tu po tobie.")),
            38 => message(s!("Człowieku wyłącz już te wiadomości")),
            39 => message(s!("Serio skończ testy!")),
            id if id > 39 && id < 99 => {
                let mut msg = String::new();
                for i in 0..=((id - 40) % 8) {
                    match i % 2 == 0 {
                        true => msg += s!("skończ"),
                        false => msg += s!(" testować "),
                    }
                }
                message(msg.as_str())
            }
            99 => message(s!("SKOŃCZ TESTOWAĆ")),
            300 => {
                *idx = 299;
                message(s!("[EASTER EGG MDMA]
                Opisz na kanale #easter-eggs jak go znalazłeś
                Jeżeli jesteś pierwszą osobą, która go odkryła otrzymasz nagrodę!
                [EASTER EGG MDMA]"))
            }
            _ => message(s!(
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit..."
            )),
        };
        jd.unwrap_js();
    }
}

pub(crate) fn init() -> JsResult<()> {
    let addon_window = ActiveSettings::new(ADDON_NAME);

    html::init(addon_window)
}
