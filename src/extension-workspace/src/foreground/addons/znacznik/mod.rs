mod descriptors;
// TODO: ADDING OWN DESCRIPTORS ON SI.
//TODO: Remove pub(crate) from this module.
pub(crate) mod html;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::ops::Deref;
use std::rc::Rc;

use discard::Discard;
use dominator::events::{Click, Load, MouseButton};
use dominator::traits::StaticEvent;
use dominator::{Dom, DomBuilder, DomHandle, apply_methods, clone};
use futures::channel::oneshot;
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_map::MutableBTreeMap;
use html::SELECTING_ITEM;
use js_sys::Function;
use proc_macros::{ActiveSettings, Setting};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{intern, prelude::*};
use web_sys::{
    AddEventListenerOptions, HtmlCanvasElement, HtmlDivElement, HtmlElement, HtmlImageElement,
};

use crate::addon_window::ITEM_FRAME;
use crate::addon_window::ui_components::ContentSection;
use crate::prelude::*;

const ADDON_NAME: AddonName = AddonName::Znacznik;

#[derive(Clone)]
struct ItemSlot {
    rarity: Mutable<Option<Rarity>>,
    canvas: Mutable<Option<HtmlCanvasElement>>,
    clear: Rc<Cell<bool>>,
}

impl Default for ItemSlot {
    fn default() -> Self {
        Self {
            rarity: Mutable::default(),
            canvas: Mutable::default(),
            clear: Rc::new(Cell::new(true)),
        }
    }
}

impl ItemSlot {
    fn init(&self, value: HtmlCanvasElement) {
        self.canvas.set(Some(value));
        self.clear.set(true);
    }

    fn clear_canvas(&self) {
        if self.clear.get() {
            return;
        }

        self.rarity.set_neq(None);
        self.clear.set(true);
        let canvas_lock = self.canvas.lock_ref();
        let canvas = canvas_lock.as_ref().unwrap_js();
        let context = canvas
            .get_context("2d")
            .unwrap_js()
            .unwrap_js()
            .unchecked_into::<web_sys::CanvasRenderingContext2d>();
        context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    }

    fn draw_canvas(&self, canvas_element: &HtmlCanvasElement) {
        self.clear_canvas();
        self.clear.set(false);
        let canvas_lock = self.canvas.lock_ref();
        let canvas = canvas_lock.as_ref().unwrap_js();
        let context = canvas
            .get_context("2d")
            .unwrap_js()
            .unwrap_js()
            .unchecked_into::<web_sys::CanvasRenderingContext2d>();
        context
            .draw_image_with_html_canvas_element(canvas_element, 0.0, 0.0)
            .unwrap_js();
    }

    fn update_rarity(&self, new_rarity: Rarity) {
        self.rarity.set_neq(Some(new_rarity));
    }
}

#[derive(Default)]
struct SelectingItem {
    active: Mutable<bool>,
    hovering_over: ItemSlot,
}

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, Clone, Debug)]
struct LocationAlias(String);

impl fmt::Display for LocationAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
struct DescriptorSetting<T> {
    value: Mutable<Option<T>>,
    active: Mutable<bool>,
}

impl<T> DescriptorSetting<T> {
    fn new(value: Option<T>, active: bool) -> Self {
        Self {
            value: Mutable::new(value),
            active: Mutable::new(active),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
struct Descriptor {
    text: DescriptorSetting<String>,
    img: DescriptorSetting<String>,
}

impl Descriptor {
    fn new(text: String, img: String) -> Self {
        Self {
            text: DescriptorSetting::new(Some(text), true),
            img: DescriptorSetting::new(Some(img), true),
        }
    }
}

#[derive(Clone, Setting)]
struct DamageTypeDisplay {
    active: Mutable<bool>,
    with_border: Mutable<bool>,
}

impl Default for DamageTypeDisplay {
    fn default() -> Self {
        Self {
            active: Mutable::new(true),
            with_border: Mutable::new(false),
        }
    }
}

type DomHandleMap = Rc<RefCell<HashMap<Id, DomHandle>>>;

#[derive(Setting, Clone)]
struct DefaultDescriptors {
    #[setting(skip)]
    search_text: Mutable<Option<String>>,
    values: MutableBTreeMap<Id, Descriptor>,
    #[setting(skip)]
    alias_list: HashMap<Id, LocationAlias>,
    display_dmg_type: DamageTypeDisplay,
    only_text: Mutable<bool>,
    only_img: Mutable<bool>,
    #[setting(skip)]
    dom_handles: DomHandleMap,
}

impl Default for DefaultDescriptors {
    fn default() -> Self {
        Self {
            search_text: Mutable::default(),
            values: descriptors::get_default(),
            alias_list: descriptors::alias_list(),
            display_dmg_type: DamageTypeDisplay::default(),
            only_text: Mutable::default(),
            only_img: Mutable::default(),
            dom_handles: DomHandleMap::default(),
        }
    }
}

impl DefaultDescriptors {
    fn try_remove_one_descriptor(&self, item_id: i32) {
        if let Some(shadow_handle) = self.dom_handles.borrow_mut().remove(&item_id) {
            shadow_handle.discard();
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct OwnDescriptor {
    text: DescriptorSetting<String>,
    img: DescriptorSetting<String>,
    rarity: DescriptorSetting<Rarity>,
}

impl OwnDescriptor {
    fn canvas_display_signal(
        &self,
        addon_active: &Mutable<bool>,
    ) -> impl Signal<Item = Option<&'static str>> + use<> {
        map_ref! {
            let addon_active = addon_active.signal(),
            let rarity = self.img.active.signal_ref(|active| match active {
                true => Some("none"),
                false => None,
            }) => {
                match addon_active {
                    true => *rarity,
                    false => None,
                }
            }
        }
    }

    fn overlay_signal(
        &self,
        disabled: Mutable<bool>,
        item_container: ItemContainer,
    ) -> impl Signal<Item = Option<Dom>> + use<> {
        map_ref! {
            let highlight_active = self.rarity.active.signal(),
            let highlight_value = self.rarity.value.signal(),
            let _ = dominator_helpers::DomMutationSignal::new(&*item_container),
            let overlay_value = ITEM_FRAME.with(|frame| frame.overlay_image_url_signal()) => {
                //debug_log!(@f "highlight change: {} {:?}", *highlight_active, *highlight_value);
                match (*highlight_active, highlight_value, overlay_value.as_ref()) {
                    (true, highlight_value, Some(url)) => Some(dominator::html!("div", {
                        .class!(overlay w[32] h[32] pos[absolute] z-i[1] p-e[none] u-s[none])
                        .style("background-image", url)
                        .apply(|builder| {
                            if let Some(upgrade_lvl) = item_container.get_data_upgrade() {
                                //common::debug_log!("BACKGROUND POS Y PRESENT");
                                builder.style("background-position-y", format!("{}px", (upgrade_lvl + 1) as i32 * -32))
                            } else {
                                builder
                            }
                        })
                        .apply_if(highlight_value.is_some(), |b| b.attr("data-rarity", highlight_value.unwrap_js().into()))
                        .class_signal("o[40%]", map_ref! {
                            let disabled = disabled.signal(),
                            let selecting_item = SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal()) => {
                                *disabled || *selecting_item
                            }
                        })
                    })),
                    _ => None
                }
            }
        }
    }

    fn highlight_signal(&self, disabled: Mutable<bool>) -> impl Signal<Item = Option<Dom>> + use<> {
        map_ref! {
            let highlight_active = self.rarity.active.signal(),
            let highlight_value = self.rarity.value.signal() => {
                //debug_log!(@f "highlight change: {} {:?}", *highlight_active, *highlight_value);
                match (*highlight_active, *highlight_value) {
                    (true, Some(rarity)) => Some(dominator::html!("div", {
                        .class!(highlight p-e[none] u-s[none])
                        .attr("data-rarity", rarity.into())
                        .class_signal("o[40%]", map_ref! {
                            let disabled = disabled.signal(),
                            let selecting_item = SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal()) => {
                                //common::debug_log!(*disabled, *selecting_item);
                                *disabled || *selecting_item
                            }
                        })
                        .style_important_signal("--item-highlight", ITEM_FRAME.with(|frame| frame.image_url_signal()))
                        .style_important_signal("--item-offset", ITEM_FRAME.with(|frame| frame.offset_signal()))
                    })),
                    _ => None
                }
            }
        }
    }

    fn text_signal(&self, disabled: Mutable<bool>) -> impl Signal<Item = Option<Dom>> + use<> {
        map_ref! {
            let disabled = disabled.signal(),
            let selecing_item = SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal()),
            let text_opt = self.text.value.signal_cloned(),
            let text_active_signal = self.text.active.signal() => {
                match (*text_active_signal, text_opt.as_ref()) {
                    (true, Some(text)) => Some(dominator::html!(intern(s!("b")), {
                        .text(text)
                        .apply_if(*disabled || *selecing_item, |builder| builder.class("o[40%]"))
                        .class!(pos[absolute] w[100%] l[0] t[0] z-i[1] h[100%] p-e[none] u-s[none] t-a[center] l-h[12] c[white] overflow[hidden])
                        .style("font-size", if text.len() < 5 { "8px" } else { "7px" })
                        .style(s!("text-shadow"), s!("0 0 2px #000"))
                        .style(s!("font-family"), s!("'Arial Bold', 'Arial Black', Gadget, sans-serif"))
                    })),
                    _ => None,
                }
            }
        }
    }

    fn canvas_signal(&self, disabled: Mutable<bool>) -> impl Signal<Item = Option<Dom>> + use<> {
        let img: HtmlImageElement = document()
            .create_element(intern(s!("img")))
            .unwrap_js()
            .unchecked_into();
        let img_value = self.img.value.signal_cloned().map_future(move |src| clone!(img => async move {
            let src = src?;
            let on_load_options = AddEventListenerOptions::new();
            on_load_options.set_once(true);

            let (sender, receiver) = oneshot::channel();
            let listener = &closure!(
                @once
                { let img = img.clone() },
                move || {
                    let mut width = img.width() as f64;
                    let mut height = img.height() as f64;
                    if height > 32.0 {
                        width *= 32.0 / height;
                        height = 32.0;
                    }
                    if width > 32.0 {
                        height *= 32.0 / width;
                        width = 32.0;
                    }
                    let offset = (32.0 - width) / 2.0;
                    img.set_width(width as u32);
                    img.set_height(height as u32);
                    let style = img.style();
                    style.set_property(intern(s!("left")), &format!("{}px", offset as i32)).unwrap_js();
                    style.set_property(intern(s!("display")), intern(s!("block"))).unwrap_js();
                    if let Err(_err) = sender.send(img) {
                        common::debug_log!(_err)
                    }
                }
            );
            img.add_event_listener_with_callback_and_add_event_listener_options(Load::EVENT_TYPE, listener, &on_load_options).unwrap_js();
            img.set_src(src.as_str());

            receiver.await.ok()
        }));
        map_ref! {
            let img_active = self.img.active.signal(),
            img_value => {
                match (img_active, img_value.as_ref().and_then(Option::as_ref)) {
                    (true, Some(img_value)) => {
                        let image_builder = apply_methods!(DomBuilder::<HtmlImageElement>::new(img_value.clone()), {
                            .style("width", "32px")
                            .style("height", "32px")
                            .class!(pos[absolute] z-i[1] p-e[none] u-s[none])
                            .class_signal("o[40%]", map_ref! {
                                let disabled = disabled.signal(),
                                let selecing_item = SELECTING_ITEM.with(|selecting_item| selecting_item.active.signal()) => {
                                    *disabled || *selecing_item
                                }
                            })
                        });
                        Some(image_builder.into_dom())
                    }
                    _ => None
                }
            }
        }
    }

    fn highlight_display_signal(
        &self,
        addon_active: &Mutable<bool>,
    ) -> impl Signal<Item = Option<&'static str>> + use<> {
        map_ref! {
            let addon_active = addon_active.signal(),
            let rarity = self.rarity.active.signal_ref(|active| match active {
                true => Some("none"),
                false => None,
            }) => {
                match addon_active {
                    true => *rarity,
                    false => None,
                }
            }
        }
    }
}

#[repr(transparent)]
#[derive(Clone)]
pub(crate) struct ItemContainer(HtmlDivElement);

impl Deref for ItemContainer {
    type Target = HtmlDivElement;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<HtmlDivElement> for ItemContainer {
    fn as_ref(&self) -> &HtmlDivElement {
        &self.0
    }
}

impl ItemContainer {
    pub(crate) fn into_builder(self) -> DomBuilder<HtmlDivElement> {
        DomBuilder::new(self.0)
    }

    #[cfg(feature = "ni")]
    pub(crate) fn find(item_id: Id) -> JsResult<Option<Self>> {
        document()
            .query_selector(&format!(".inventory-item.item-id-{}", item_id))
            .map(|container_opt| {
                container_opt.map(|item_container| Self(item_container.unchecked_into()))
            })
    }

    #[cfg(not(feature = "ni"))]
    pub(crate) fn find(item_id: Id) -> JsResult<Option<Self>> {
        document()
            .query_selector(&format!("#item{}", item_id))
            .map(|container_opt| {
                container_opt.map(|item_container| Self(item_container.unchecked_into()))
            })
    }

    pub(crate) fn get_data_upgrade(&self) -> Option<u8> {
        self.0
            .get_attribute("data-upgrade")
            .and_then(|upgrade_lvl| upgrade_lvl.parse().ok())
    }

    // Bags have no highlight on SI
    fn item_highlight(&self) -> JsResult<Option<HtmlDivElement>> {
        //debug_log!("in highlight", &self.0);
        let selectors = match cfg!(feature = "ni") {
            true => intern(".highlight"),
            false => intern(".itemHighlighter"),
        };

        self.0
            .query_selector(selectors)
            .map_err(map_err!())?
            .map(JsCast::dyn_into)
            .transpose()
            .map_err(map_err!())
    }

    fn item_canvas(&self) -> JsResult<HtmlElement> {
        //debug_log!("in canvas", &self.0);
        let selectors = match cfg!(feature = "ni") {
            true => intern(".icon.canvas-icon"),
            false => intern("img"),
        };

        self.0
            .query_selector(selectors)
            .map_err(map_err!())?
            .ok_or_else(|| err_code!())?
            .dyn_into()
            .map_err(map_err!())
    }

    #[cfg(feature = "ni")]
    fn item_notice(&self) -> JsResult<HtmlCanvasElement> {
        self.0
            .query_selector(intern(s!(".canvas-notice")))
            .map_err(map_err!())?
            .ok_or_else(|| err_code!())?
            .dyn_into()
            .map_err(map_err!())
    }

    fn item_amount(&self) -> JsResult<Option<HtmlElement>> {
        let selectors = match cfg!(feature = "ni") {
            true => intern(".amount"),
            false => intern("small"),
        };
        let Some(amount) = self.0.query_selector(selectors).map_err(map_err!())? else {
            return Ok(None);
        };

        Ok(Some(amount.dyn_into().map_err(map_err!())?))
    }

    #[cfg(feature = "ni")]
    fn last_item_cooldown(&self) -> JsResult<Option<HtmlDivElement>> {
        let cooldowns = self
            .0
            .query_selector_all(intern(s!(".cooldown")))
            .map_err(map_err!())?;

        Ok(cooldowns
            .get(cooldowns.length().saturating_sub(1))
            .map(|cooldown| cooldown.unchecked_into()))
    }
}

/// Item display in own descriptors sections
#[derive(Default, PartialEq)]
struct ItemDisplay {
    /// Rarity of in game item
    rarity: Option<Rarity>,
    /// Canvas of in game item
    item_canvas: Option<HtmlElement>,
    /// Canvas of item in own descriptors section
    descriptor_item_canvas: Option<HtmlCanvasElement>,
    /// Canvas of rendered item
    rendered_item_canvas: Option<HtmlCanvasElement>,
}

impl ItemDisplay {
    fn new(item_id: Id, item_canvas: HtmlElement) -> Self {
        let items_lock = Items::get().lock_ref();
        let item_data = items_lock.get(&item_id).unwrap_js();
        let item_stats = item_data.parse_stats().unwrap_js();

        Self {
            rarity: Some(item_stats.rarity),
            item_canvas: Some(item_canvas),
            descriptor_item_canvas: None,
            rendered_item_canvas: None,
        }
    }
}

type ShadowTreeHandle = Mutable<Option<DomHandle>>;

struct OwnItemHandle {
    item_id: Id,
    item_container: Option<ItemContainer>,
    item_display: Mutable<ItemDisplay>,
    shadow_tree_handle: ShadowTreeHandle,
}

impl OwnItemHandle {
    fn new(item_id: Id, own_descriptor: &OwnDescriptor) -> Self {
        //debug_log!("new item handle for ", item_id);
        let item_container = ItemContainer::find(item_id).ok().flatten();
        let item_display = match item_container
            .as_ref()
            .map(|container| container.item_canvas().unwrap_js())
        {
            None => {
                //debug_log!("placeholder for", item_id);
                ItemDisplay::default()
            }
            Some(item_canvas) => ItemDisplay::new(item_id, item_canvas),
        };
        let item_display = Mutable::new(item_display);
        let item_handle = Self {
            item_id,
            item_container,
            item_display,
            shadow_tree_handle: ShadowTreeHandle::default(),
        };

        if let Some(item_container) = item_handle.item_container.as_ref() {
            let items_lock = Items::get().lock_ref();
            let item_data = items_lock.get(&item_id).unwrap_js();
            let disabled = &item_data.disabled;

            if let Err(err_code) =
                item_handle.init_own_descriptor(item_container, own_descriptor, disabled)
            {
                console_error!(err_code);
            }
        }

        item_handle
    }

    fn update(&mut self, item_id: Id, own_descriptor: &OwnDescriptor) {
        // debug_log!("On update container:", self.item_container.is_some());
        if self.item_container.is_some() {
            //debug_log!("item", item_id, "is not placeholder");
            return;
        }
        //common::debug_log!("TESTETSTET");

        let Some(item_container) = ItemContainer::find(item_id).ok().flatten() else {
            //TODO: Is this correct ?
            return;
        };
        let item_canvas = match item_container.item_canvas() {
            Ok(canvas) => canvas,
            Err(err_code) => return console_error!(err_code),
        };

        let item_display = ItemDisplay::new(item_id, item_canvas);
        self.item_display.set_neq(item_display);
        let items_lock = Items::get().lock_ref();
        let item_data = items_lock.get(&item_id).unwrap_js();
        let disabled = &item_data.disabled;

        if let Err(err) = self.init_own_descriptor(&item_container, own_descriptor, disabled) {
            console_error!(err);
        }

        self.item_container = Some(item_container);
    }

    fn get_item_view(
        &self,
        own_descriptor: OwnDescriptor,
        own_descriptors: &'static OwnDescriptors,
    ) -> ContentSection {
        ContentSection::new()
            .class_list("game-item t[1] l[1]")
            .section_signal(
                self.item_display
                    .signal_ref(|display| -> Option<ContentSection> {
                        let rarity = display.rarity.as_ref()?;
                        Some(
                            ContentSection::new()
                                .mixin(|builder| builder.class(["highlight", rarity.into()])),
                        )
                    }),
            )
            .mixin(|builder| {
                let item_id = self.item_id;

                builder.child_signal(self.item_display.signal_ref(move |display| {
                    let Some(item_canvas) = display.item_canvas.as_ref() else {
                        let item_canvas_builder: DomBuilder<HtmlCanvasElement> =
                            own_descriptors.prepare_canvas(DomBuilder::new_html("canvas"), item_id);
                        // TODO: Drawing placeholder icon on si.
                        #[cfg(feature = "ni")]
                        {
                            let canvas = item_canvas_builder.__internal_element();
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

                        return Some(item_canvas_builder.into_dom());
                    };

                    let item_canvas_builder: DomBuilder<HtmlCanvasElement> = own_descriptors
                        .prepare_canvas(DomBuilder::new_html("canvas"), item_id)
                        .event(clone!(own_descriptor => move |event: Click| {
                            if event.button() != MouseButton::Left {
                                return;
                            }

                            //debug_log!("On leftclick", item_id);
                            own_descriptors
                                .editing
                                .set(Some((item_id, own_descriptor.clone())));
                        }));
                    let item_view_canvas = item_canvas_builder.__internal_element();
                    let context = item_view_canvas
                        .get_context("2d")
                        .unwrap_js()
                        .unwrap_js()
                        .unchecked_into::<web_sys::CanvasRenderingContext2d>();
                    context.clear_rect(
                        0.0,
                        0.0,
                        item_view_canvas.width() as f64,
                        item_view_canvas.height() as f64,
                    );
                    // Cast should be safe since for drawImage it does not matter whether it's actually a canvas or an img.
                    context
                        .draw_image_with_html_canvas_element(item_canvas.unchecked_ref(), 0.0, 0.0)
                        .unwrap_js();

                    #[cfg(feature = "ni")]
                    {
                        let game_item = get_engine()
                            .items_manager()
                            .unwrap_js()
                            .get_item_by_id(item_id)
                            .unwrap_js()
                            .unchecked_into::<GameItem>();
                        game_item.observe_draw_icon(context, item_view_canvas);
                    }

                    Some(item_canvas_builder.into_dom())
                }))
            })
    }

    fn disable_signal(
        &self,
        style: &'static str,
    ) -> impl Signal<Item = Option<&'static str>> + use<> {
        map_ref! {
            let selecing_item = SELECTING_ITEM.with(|selecing_item| selecing_item.active.signal()),
            let has_descriptor = self.shadow_tree_handle.signal_ref(|handle_opt| handle_opt.is_some()) => {
                match *selecing_item && *has_descriptor {
                    true => Some(style),
                    false => None,
                }
            }
        }
    }
}

type OwnItemDomHandleMap = Rc<RefCell<HashMap<Id, OwnItemHandle>>>;

#[derive(Clone, Setting, Default)]
struct OwnDescriptors {
    values: MutableBTreeMap<Id, OwnDescriptor>,
    #[setting(skip)]
    item_dom_handles: OwnItemDomHandleMap,
    #[setting(skip)]
    editing: Mutable<Option<(Id, OwnDescriptor)>>,
}

impl OwnDescriptors {
    #[cfg(feature = "ni")]
    fn init(&'static self) -> JsResult<()> {
        let items_manager = get_engine().items_manager().ok_or_else(|| err_code!())?;
        let original_update_placeholder = items_manager
            .get_update_placeholder()
            .ok_or_else(|| err_code!())?;
        let new_update_placeholder = closure!(
            { let items_manager = items_manager.clone() },
            // This item_id is a string, not a number!
            move |item_id: JsValue| -> JsResult<()> {
                original_update_placeholder.call1(&items_manager, &item_id)?;

                let item_id_num = item_id.unchecked_into_f64() as Id;

                if let Some(item_handle) = self.item_dom_handles.borrow_mut().get_mut(&item_id_num)
                {
                    item_handle.item_container = None;
                    if let Some(shadow_tree_handle) =
                        item_handle.shadow_tree_handle.lock_mut().take()
                    {
                        shadow_tree_handle.discard();
                    }
                }
                if let Err(err) = self.try_render_for_one_item(item_id_num) {
                    console_error!(err)
                }

                Ok(())
            },
        );

        items_manager.set_update_placeholder(&new_update_placeholder);

        Ok(())
    }

    // TODO: PLACEHOLDER ON SI: https://experimental.margonem.pl/img/def-item.gif
    #[cfg(not(feature = "ni"))]
    fn init(&'static self) -> JsResult<()> {
        let original_load_img = crate::bindings::window()
            .get_load_img()
            .ok_or_else(|| err_code!())?;
        let new_load_img = closure!(
            { let original_load_img = original_load_img.clone() },
            move |url: JsValue, id: JsValue, clb: Function| -> JsResult<()> {
                original_load_img.call3(&crate::bindings::window(), &url, &id, &clb)?;

                let item_id_num = id.unchecked_into_f64() as Id;

                // FIXME: Temp solution since we can't directly hook to afterLoadClb (need to access this keyword from WASM)
                wasm_bindgen_futures::spawn_local(async move {
                    loop {
                        delay(50).await;
                        // common::debug_log!("id:", item_id_num);
                        let item_img = document()
                            .query_selector(&format!("#item{} > img", item_id_num))
                            .unwrap_js();
                        if item_img.is_none_or(|item_img| !item_img.has_attribute("dest")) {
                            continue;
                        }
                        // common::debug_log!("item_img:", &item_img);

                        if let Some(item_handle) = self.item_dom_handles.borrow_mut().get_mut(&item_id_num)
                        {
                            item_handle.item_container = None;
                            if let Some(shadow_tree_handle) =
                                item_handle.shadow_tree_handle.lock_mut().take()
                            {
                                shadow_tree_handle.discard();
                            }
                        }
                        if let Err(err) = self.try_render_for_one_item(item_id_num) {
                            console_error!(err)
                        }
                        break;
                    }
                });

                Ok(())
            },
        );
        crate::bindings::window().set_load_img(&new_load_img);

        Ok(())
    }

    fn try_render_for_one_item(&'static self, item_id: Id) -> JsResult<()> {
        // TODO: Update descriptor render.
        let own_descriptors_lock = self.values.lock_ref();
        let Some(own_descriptor) = own_descriptors_lock.get(&item_id) else {
            return Ok(());
        };
        let mut item_handles_lock = self.item_dom_handles.borrow_mut();
        let Some(item_handle) = item_handles_lock.get_mut(&item_id) else {
            return Ok(());
        };
        item_handle.update(item_id, own_descriptor);

        Ok(())
    }

    fn remove_one(&self, item_id: &Id) {
        let Some(own_descriptor) = self.values.lock_mut().remove(item_id) else {
            return;
        };
        own_descriptor.text.active.set_neq(false);
        own_descriptor.img.active.set_neq(false);
        own_descriptor.rarity.active.set_neq(false);

        let mut editing_lock = self.editing.lock_mut();
        if editing_lock
            .as_ref()
            .is_some_and(|editing| editing.0 == *item_id)
        {
            *editing_lock = None;
        }

        let mut handles_lock = self.item_dom_handles.borrow_mut();
        let Some(item_handle) = handles_lock.get_mut(item_id) else {
            //debug_log!("no item dom handle for id", *item_id);
            return;
        };
        let Some(shadow_tree_handle) = item_handle.shadow_tree_handle.lock_mut().take() else {
            return;
        };
        shadow_tree_handle.discard();
    }
}

#[derive(Default, ActiveSettings, Clone)]
struct ActiveSettings {
    default_descriptors: DefaultDescriptors,
    character_descriptors: OwnDescriptors,
}

impl ActiveSettings {
    fn try_render_default(&'static self, item_id: Id, item_data: &Item) -> JsResult<()> {
        //Item is not in hero equipment.
        if item_data.loc.as_ref().is_none_or(|loc| loc != "g") {
            //debug_log!(@f "returning since not in loc g {:?}", item_data.loc);
            return Ok(());
        }
        //Shadow tree is already rendered.
        if self
            .default_descriptors
            .dom_handles
            .borrow()
            .contains_key(&item_id)
        {
            //common::debug_log!(@f "returning since already rendered {:?}", item_id);
            return Ok(());
        }
        let Ok(Some(item_container)) = ItemContainer::find(item_id) else {
            return Ok(());
        };
        let stats = item_data.parse_stats().ok_or_else(|| err_code!())?;

        let shadow_tree_handle = if let Some((map_id, _, _, _)) = stats.custom_teleport {
            let descriptors_lock = self.default_descriptors.values.lock_ref();
            let Some(descriptor) = descriptors_lock.get(&map_id) else {
                return Ok(());
            };

            if self
                .character_descriptors
                .values
                .lock_ref()
                .contains_key(&item_id)
            {
                common::debug_log!("REMOVING character_descriptor:", item_id);
                self.character_descriptors.remove_one(&item_id);
            }

            descriptor.render(
                &self.default_descriptors.only_img,
                &self.default_descriptors.only_text,
                &item_data.disabled,
                item_container,
            )?
        } else if let Some(dmg_type_class) = stats.dmg_type.into_class() {
            self.render_dmg_type(item_id, dmg_type_class, item_container, &item_data.disabled)?
        } else {
            return Ok(());
        };

        self.default_descriptors
            .dom_handles
            .borrow_mut()
            .insert(item_id, shadow_tree_handle);

        Ok(())
    }

    fn init_default_descriptors(&'static self) -> JsResult<()> {
        let future = self
            .default_descriptors
            .only_text
            .signal()
            .for_each(|only_text| {
                if only_text {
                    self.default_descriptors.only_img.set_neq(false);
                }

                async {}
            });
        wasm_bindgen_futures::spawn_local(future);

        let future = self
            .default_descriptors
            .only_img
            .signal()
            .for_each(|only_img| {
                if only_img {
                    self.default_descriptors.only_text.set_neq(false);
                }

                async {}
            });
        wasm_bindgen_futures::spawn_local(future);

        self.observe_update_placeholder()
    }

    #[cfg(feature = "ni")]
    fn observe_update_placeholder(&'static self) -> JsResult<()> {
        let items_manager = get_engine().items_manager().ok_or_else(|| err_code!())?;
        let original_update_placeholder = items_manager
            .get_update_placeholder()
            .ok_or_else(|| err_code!())?;
        let new_update_placeholder = closure!(
            { let items_manager = items_manager.clone() },
            // This item_id is a string, not a number!
            move |item_id: JsValue| -> JsResult<()> {
                original_update_placeholder.call1(&items_manager, &item_id)?;

                let item_id_num = item_id.unchecked_into_f64() as Id;

                if item_id_num == 0 {
                    //common::debug_log!("no item when placehoder updated", updated_item_id.clone());
                    return Ok(());
                }

                let items_lock = Items::get().lock_ref();
                let Some(item_data) = items_lock.get(&item_id_num) else {
                    //TODO: error when fighting collosus after death on loot.
                    //debug_log!(@f "{:?}", items_lock);
                    //debug_log!("item_id: ", updated_item_id.clone());
                    //return console_error!();
                    return Ok(());
                };
                //let items_lock = globals.items.lock_ref();
                //let item_data = items_lock.get(&item_id_num).unwrap_js();

                //common::debug_log!("Update placeholder remove default");
                self.default_descriptors
                    .try_remove_one_descriptor(item_id_num);
                if let Err(err) = self.try_render_default(item_id_num, item_data) {
                    console_error!(err);
                }

                Ok(())
            },
        );

        items_manager.set_update_placeholder(&new_update_placeholder);

        Ok(())
    }

    // TODO: PLACEHOLDER ON SI: https://experimental.margonem.pl/img/def-item.gif
    #[cfg(not(feature = "ni"))]
    fn observe_update_placeholder(&'static self) -> JsResult<()> {
        let original_load_img = crate::bindings::window()
            .get_load_img()
            .ok_or_else(|| err_code!())?;
        let new_load_img = closure!(
            { let original_load_img = original_load_img.clone() },
            move |url: JsValue, id: JsValue, clb: Function| -> JsResult<()> {
                original_load_img.call3(&crate::bindings::window(), &url, &id, &clb)?;

                let item_id_num = id.unchecked_into_f64() as Id;

                if item_id_num == 0 || !Items::get().lock_ref().contains_key(&item_id_num) {
                    //common::debug_log!("no item when placehoder updated", updated_item_id.clone());
                    return Ok(());
                }

                // FIXME: Temp solution since we can't directly hook to afterLoadClb (need to access this keyword from WASM)
                wasm_bindgen_futures::spawn_local(async move {
                    loop {
                        delay(50).await;
                        // common::debug_log!("id:", item_id_num);
                        let item_img = document()
                            .query_selector(&format!("#item{} > img", item_id_num))
                            .unwrap_js()
                            .unwrap_js()
                            .unchecked_into::<web_sys::HtmlCanvasElement>();
                        if !item_img.has_attribute("dest") {
                            continue;
                        }
                        // common::debug_log!("item_img:", &item_img);

                        let items_lock = Items::get().lock_ref();
                        let item_data = items_lock.get(&item_id_num).unwrap_js();
                        //let items_lock = globals.items.lock_ref();
                        //let item_data = items_lock.get(&item_id_num).unwrap_js();

                        //common::debug_log!("Update placeholder remove default");
                        self.default_descriptors
                            .try_remove_one_descriptor(item_id_num);
                        if let Err(err) = self.try_render_default(item_id_num, item_data) {
                            console_error!(err);
                        }
                        break;
                    }
                });

                Ok(())
            },
        );
        crate::bindings::window().set_load_img(&new_load_img);

        Ok(())
    }
}

#[allow(non_snake_case)]
#[cfg(feature = "ni")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type GameItem;

    #[wasm_bindgen(method, getter = "id")]
    pub(crate) fn id(this: &GameItem) -> JsValue;

    #[wasm_bindgen(method, getter = "imgLoaded")]
    pub(crate) fn img_loaded(this: &GameItem) -> Option<bool>;

    #[wasm_bindgen(method, getter = "sprite")]
    pub(crate) fn sprite(this: &GameItem) -> HtmlImageElement;

    #[wasm_bindgen(method, getter = "staticAnimation")]
    pub(crate) fn static_animation(this: &GameItem) -> JsValue;

    #[wasm_bindgen(method, getter = "activeFrame")]
    pub(crate) fn active_frame(this: &GameItem) -> f64;

    #[wasm_bindgen(method, getter = "loc")]
    pub(crate) fn loc(this: &GameItem) -> JsValue;

    #[wasm_bindgen(method, getter = "drawIcon")]
    pub(crate) fn get_draw_icon(this: &GameItem) -> Function;

    #[wasm_bindgen(method, setter = "drawIcon")]
    pub(crate) fn set_draw_icon(this: &GameItem, value: &Function);

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) type JqueryCanvasIcon;

    #[wasm_bindgen(method, getter = "$canvasIcon")]
    pub(crate) fn get_canvas_icon(this: &GameItem) -> JqueryCanvasIcon;

    #[wasm_bindgen(method, getter = "0")]
    pub(crate) fn get_canvas_elem(this: &JqueryCanvasIcon) -> HtmlCanvasElement;

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    type JqueryItemSelector;

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    type JqueryItemSelectorData;
    #[wasm_bindgen(method, js_name = "find")]
    pub(crate) fn find(this: &JqueryItemSelector, selector: &str) -> JqueryItemSelector;

    #[wasm_bindgen(method, js_name = "css")]
    pub(crate) fn css(
        this: &JqueryItemSelector,
        property_name: &str,
        value: &str,
    ) -> JqueryItemSelector;

    #[wasm_bindgen(catch, method, js_name = "data")]
    pub(crate) fn data(this: &JqueryItemSelector) -> JsResult<JqueryItemSelectorData>;

    #[wasm_bindgen (extends = ::js_sys::Object)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    type JqueryItemData;

    #[wasm_bindgen(method, getter = "item")]
    pub(crate) fn item(this: &JqueryItemSelectorData) -> JqueryItemData;

    #[wasm_bindgen(method, getter = "id")]
    pub(crate) fn id(this: &JqueryItemData) -> JsValue;
}

#[cfg(feature = "ni")]
impl GameItem {
    pub(crate) fn observe_draw_icon(
        &self,
        ctx: web_sys::CanvasRenderingContext2d,
        view_canvas: HtmlCanvasElement,
    ) {
        let original_draw_icon = self.get_draw_icon();
        let this = self.clone();
        let new_draw_icon = closure!(move |item_ctx: JsValue, canvas: JsValue| -> DefaultResult {
            let sy = if this.static_animation().is_truthy() {
                0.0
            } else {
                this.active_frame() * 32.0
            };
            ctx.clear_rect(
                0.0,
                0.0,
                view_canvas.width() as f64,
                view_canvas.height() as f64,
            );
            ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &this.sprite(),
                0.0,
                sy,
                32.0,
                32.0,
                0.0,
                0.0,
                32.0,
                32.0,
            )?;
            original_draw_icon.call2(&this, &item_ctx, &canvas)
        });
        self.set_draw_icon(&new_draw_icon);
    }
}

#[cfg(feature = "ni")]
#[derive(Serialize)]
pub(crate) struct RemoveCallbackObj<'a> {
    pub(crate) loc: &'a str,
    #[serde(rename = "k")]
    pub(crate) key: &'a str,
}

impl ActiveSettings {
    #[cfg(feature = "ni")]
    fn observe_remove_disable_icon() {
        let disable_items_manager = get_engine().disable_items_manager().unwrap_js();
        let original_remove_disable_icon =
            disable_items_manager.get_remove_disable_icon().unwrap_js();
        let new_remove_disable_icon = closure!(
            { let disable_items_manager = disable_items_manager.clone() },
            move |jquery_item: JqueryItemSelector| -> DefaultResult {
                //common::debug_log!("remove_disable_icon UPDATED:", &jquery_item);
                let item_id = jquery_item.data().unwrap_js().item().id().unchecked_into_f64() as Id;
                Items::get().lock_ref().get(&item_id).unwrap_js().disabled.set_neq(false);
                original_remove_disable_icon.call1(&disable_items_manager, &jquery_item)
            }
        );

        disable_items_manager.set_remove_disable_icon(&new_remove_disable_icon);
    }

    #[cfg(feature = "ni")]
    fn observe_add_disable_icon() {
        let disable_items_manager = get_engine().disable_items_manager().unwrap_js();
        let original_add_disable_icon = disable_items_manager.get_add_disable_icon().unwrap_js();
        let new_add_disable_icon = closure!(
            { let disable_items_manager = disable_items_manager.clone() },
            move |jquery_item: JqueryItemSelector| -> DefaultResult {
                //common::debug_log!("add_disable_icon UPDATED:", &jquery_item);
                let item_id = jquery_item.data().unwrap_js().item().id().unchecked_into_f64() as Id;
                Items::get().lock_ref().get(&item_id).unwrap_js().disabled.set_neq(true);

                let res = original_add_disable_icon.call1(&disable_items_manager, &jquery_item);
                jquery_item.find(".disable-icon").css("z-index", "1");

                res
            }
        );

        disable_items_manager.set_add_disable_icon(&new_add_disable_icon);
    }

    #[cfg(feature = "ni")]
    fn observe_new_item(&'static self) {
        let items_mark_manager = get_engine().items_mark_manager().unwrap_js();
        let original_new_item = items_mark_manager.get_new_item().unwrap_js();
        let new_new_item = closure!(
            { let items_mark_manager = items_mark_manager.clone() },
            move |game_item: GameItem| -> DefaultResult {
                //common::debug_log!("ITEM UPDATED:", &game_item);
                let res = original_new_item.call1(&items_mark_manager, &game_item);

                if game_item.img_loaded().is_some_and(|loaded| loaded) {
                    let item_id = game_item.id().unchecked_into_f64() as Id;
                    self.character_descriptors.try_render_for_one_item(item_id)?;
                    let item_data: Item = serde_wasm_bindgen::from_value(game_item.into()).map_err(map_err!(from))?;
                    self.try_render_default(item_id, &item_data)?;
                }

                res
            }
        );

        items_mark_manager.set_new_item(&new_new_item);
    }

    #[cfg(feature = "ni")]
    fn observe_new_inventory_items(&'static self) {
        let hero_equipment = get_engine().hero_equipment().unwrap_js();
        let original_new_inventory_items = hero_equipment.get_new_inventory_items().unwrap_js();
        let new_new_inventory_items = closure!(
            { let hero_equipment = hero_equipment.clone() },
            move |game_item: GameItem, finish: JsValue| -> DefaultResult {
                //common::debug_log!("EQUIPMENT ITEM UPDATED:", &game_item);
                let res = original_new_inventory_items.call2(&hero_equipment, &game_item, &finish);

                if game_item.img_loaded().is_some_and(|loaded| loaded) {
                    let item_id = game_item.id().unchecked_into_f64() as Id;
                    self.character_descriptors.try_render_for_one_item(item_id)?;
                    let item_data: Item = serde_wasm_bindgen::from_value(game_item.into()).map_err(map_err!(from))?;
                    self.try_render_default(item_id, &item_data)?;
                }

                res
            }
        );

        let items_manager = get_engine().items_manager().unwrap_js();
        let remove_clb_data = serde_wasm_bindgen::to_value(&RemoveCallbackObj {
            loc: "g",
            key: "NEW_INVENTORY_ITEM",
        })
        .unwrap_js()
        .unchecked_into();
        items_manager.remove_callback(&remove_clb_data).unwrap_js();
        items_manager
            .add_callback("g", "NEW_INVENTORY_ITEM", &new_new_inventory_items)
            .unwrap_js();
    }

    #[cfg(feature = "ni")]
    fn observe_items(&'static self) {
        self.observe_new_item();
        self.observe_new_inventory_items();
        Self::observe_add_disable_icon();
        Self::observe_remove_disable_icon();
    }
}

pub(crate) fn init() -> JsResult<()> {
    let active_settings = ActiveSettings::new(ADDON_NAME);

    html::init(active_settings)?;
    active_settings.init_default_descriptors()?;
    active_settings.character_descriptors.init()?;
    #[cfg(feature = "ni")]
    active_settings.observe_items();

    Ok(())
}
