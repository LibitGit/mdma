// FIXME: Two of the same item in slots (single, weapon) -> icon not loading in group slot.
// FIXME: Dont add items from any slot into buffer.
//TODO: List below:
//enhancement&action=status&item=1072351542
//1. artisanship&action=open
//In both cases of point 2 there is a comma separated list of ingredients.
//2.a. enhancement&action=progress&item=1073005505&ingredients=1073005568&answer1001012=1&answer1001015=1
//1001012 - progress overflow (utracisz reszte punktow...)
//1001015 - bound confirm (ulepszenie powoduje zwiazanie)
//2.b for salvaging => salvager&action=salvage&selectedItems=1073050400
// TODO: Remove item_tpl and item templates from ws message on progress and upgrade.
mod html;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ops::{Index, Not};
use std::pin::Pin;

#[cfg(feature = "ni")]
use common::map_err;
use common::{closure, debug_log, err_code, throw_err_code};
use dominator_helpers::FreeEquipmentSlotsSignal;
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt, from_stream};
use futures_signals::signal_map::MutableBTreeMap;
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use html::{ANIMATION_ACTIVE, SELECTING_ITEM, init_animation};
use js_sys::Function;
use proc_macros::{ActiveSettings, Setting, Settings};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use wasm_bindgen::{intern, prelude::*};
use web_sys::HtmlCanvasElement;

use crate::addon_window::prelude::ItemNameInput;
#[cfg(feature = "ni")]
use crate::addons::znacznik::GameItem;
use crate::bindings::engine::types::{EquipmentItemGroup, ItemClass};
use crate::disable_items::{DISABLED_ITEMS, DisabledItems};
use crate::prelude::*;

use super::znacznik::ItemContainer;

const ADDON_NAME: AddonName = AddonName::SmartForge;
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

struct ItemSlot {
    rarity: Mutable<Option<Rarity>>,
    canvas: Mutable<Option<HtmlCanvasElement>>,
    item_id: Mutable<Option<Id>>,
    slot_type: Mutable<Option<SlotType>>,
    clear: Cell<bool>,
}

impl Default for ItemSlot {
    fn default() -> Self {
        Self {
            rarity: Mutable::default(),
            canvas: Mutable::default(),
            item_id: Mutable::default(),
            slot_type: Mutable::default(),
            clear: Cell::new(true),
        }
    }
}

impl ItemSlot {
    fn init(&self, slot_type: SlotType, settings: &Settings) {
        let Some(canvas) = settings.item_slots[slot_type].lock_ref().canvas.clone() else {
            return console_error!();
        };
        self.canvas.set(Some(canvas));
        self.clear.set(true);
        self.slot_type.set(Some(slot_type));
        DISABLED_ITEMS.with_borrow_mut(DisabledItems::disable_items);
        // self.disable_items(slot_type, settings);
    }

    fn clear_canvas(&self, settings: &Settings) {
        if self.clear.get() {
            return;
        }
        debug_log!("CLEARING CANVAS");

        let Some(slot_type) = self.slot_type.get() else {
            return console_error!();
        };
        // FIXME: This is bad since it spams the slot setting change.
        settings.item_slots.clear_slot(slot_type);
        self.rarity.set_neq(None);
        self.clear.set(true);
    }

    fn draw_canvas(&self, settings: &Settings, item_canvas: &HtmlCanvasElement) {
        self.clear.set(false);

        let Some(slot_type) = self.slot_type.get() else {
            return console_error!();
        };
        settings.item_slots.draw_in_slot(slot_type, item_canvas);
    }
}

#[derive(Default)]
struct SelectingItem {
    active: Mutable<bool>,
    hovering_over: ItemSlot,
}

impl SelectingItem {
    fn deselect(&self) -> Option<(SlotType, Rarity)> {
        let _active = self.active.replace(false);
        let _clear = self.hovering_over.clear.replace(true);
        let _canvas = self.hovering_over.canvas.replace(None);
        let _item_id = self.hovering_over.item_id.replace(None);
        let rarity = self.hovering_over.rarity.replace(None);
        let slot_type = self.hovering_over.slot_type.replace(None);

        slot_type.zip(rarity)
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[repr(u8)]
enum UpgradingMode {
    #[default]
    Single,
    Group,
    Hybrid,
}

impl From<UpgradingMode> for &'static str {
    fn from(value: UpgradingMode) -> Self {
        match value {
            UpgradingMode::Hybrid => "Hybrydowe",
            UpgradingMode::Group => "Po typie",
            UpgradingMode::Single => "Proste",
        }
    }
}

impl<'a> From<&'a UpgradingMode> for &'static str {
    fn from(value: &'a UpgradingMode) -> Self {
        match value {
            UpgradingMode::Hybrid => "Hybrydowe",
            UpgradingMode::Group => "Po typie",
            UpgradingMode::Single => "Proste",
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct UpgradeSlot {
    item_id: Option<Id>,
    ///Item view canvas.
    #[serde(skip)]
    canvas: Option<HtmlCanvasElement>,
    ///Item view rarity.
    #[serde(skip)]
    rarity: Option<Rarity>,
    #[serde(skip)]
    original_draw: Option<Function>,
    #[serde(skip)]
    current: Option<u32>,
    #[serde(skip)]
    max: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Setting)]
struct SingleSlot {
    slot: Mutable<UpgradeSlot>,
    ///Item preview canvas.
    #[serde(skip)]
    #[setting(skip)]
    preview_canvas: RefCell<Option<HtmlCanvasElement>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Setting)]
struct GroupSlots {
    armor: Mutable<UpgradeSlot>,
    jewelry: Mutable<UpgradeSlot>,
    weapons: Mutable<UpgradeSlot>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, Setting)]
struct ItemSlots {
    single: SingleSlot,
    group: GroupSlots,
}

impl ItemSlots {
    async fn init_slot_progress(&'static self, slot_type: SlotType) -> JsResult<()> {
        let Some(item_id) = self[slot_type].lock_ref().item_id else {
            return Ok(());
        };
        if !Items::get().lock_ref().contains_key(&item_id) {
            //common::debug_log!("no item in items");
            return Ok(());
        }

        let interceptor = move |socket_response: &mut Response| {
            let mut slot_lock = self[slot_type].lock_mut();
            let Some(item_id) = slot_lock.item_id else {
                // Happens if slot gets cleared before response is received.
                return Ok(());
            };
            let items_lock = Items::get().lock_ref();
            let item_name = items_lock
                .get(&item_id)
                .ok_or_else(|| err_code!())?
                .name
                .as_ref()
                .ok_or_else(|| err_code!())?;
            let _item_tpl = socket_response
                .item
                .as_mut()
                .ok_or_else(|| err_code!())?
                .remove(&-item_id);
            // debug_log!("jd", _item_tpl.is_some(), -item_id);

            let enhancement = socket_response
                .enhancement
                .take()
                .ok_or_else(|| err_code!())?;

            let Some(enhance_progress) = enhancement.progressing else {
                drop(slot_lock);
                self.clear_slot(slot_type);

                let _ = message(&format!(
                    r#"[MDMA::RS] Zwalniam slot ulepszania ze względu na poziom ulepszenia przedmiotu "{item_name}"..."#
                ));
                return Ok(());
            };
            // TODO: Test removing items with loc = "u" thoroughly.
            let items = socket_response.item.take().unwrap_js();
            if !items
                .iter()
                .all(|(_, item)| item.loc.as_deref() == Some("u"))
            {
                return Err(err_code!());
            }

            let current = enhance_progress.current.ok_or_else(|| err_code!())?;
            let max = enhance_progress.max.ok_or_else(|| err_code!())?;

            //debug_log!(@f "{current}/{max}");
            slot_lock.current = Some(current);
            slot_lock.max = Some(max);

            Ok(())
        };

        let event = EmitterEvent::Enhancement;
        let callback_id = Emitter::intercept_once(event, move |socket_response| {
            let res = interceptor(socket_response);
            Box::pin(async move { res })
        })?;

        __send_task(&format!("enhancement&action=status&item={item_id}")).await?;
        //debug_log!("before waiting for receiver");
        Emitter::wait_for_intercept(event, callback_id).await
        //debug_log!("after waiting for receiver");
    }

    #[cfg(feature = "ni")]
    fn init_animation(&'static self, slot_type: SlotType, item_id: Id) {
        //Could already be some from mouse_over events.
        if self[slot_type].lock_ref().original_draw.is_some() {
            return;
        }

        let item_view_canvas = self[slot_type]
            .lock_ref()
            .canvas
            .as_ref()
            .unwrap_js()
            .clone();
        let ctx = item_view_canvas
            .get_context("2d")
            .unwrap_js()
            .unwrap_js()
            .unchecked_into::<web_sys::CanvasRenderingContext2d>();
        let preview_ctx = match slot_type == SlotType::Single {
            true => Some(
                self.single
                    .preview_canvas
                    .borrow()
                    .as_ref()
                    .unwrap_js()
                    .get_context("2d")
                    .unwrap_js()
                    .unwrap_js()
                    .unchecked_into::<web_sys::CanvasRenderingContext2d>(),
            ),
            false => None,
        };
        let game_item = get_engine()
            .items_manager()
            .unwrap_js()
            .get_item_by_id(item_id)
            .unwrap_js()
            .unchecked_into::<GameItem>();
        let original_draw_icon = game_item.get_draw_icon();

        self[slot_type].lock_mut().original_draw = Some(original_draw_icon.clone());

        let new_draw_icon = closure!(
            { let game_item = game_item.clone() },
            move |item_ctx: JsValue, canvas: JsValue| -> JsResult<JsValue> {
                let sy = if game_item.static_animation().is_truthy() {
                    0.0
                } else {
                    game_item.active_frame() * 32.0
                };

                if let Some(preview_ctx) = preview_ctx.as_ref() {
                    preview_ctx.clear_rect(
                        0.0,
                        0.0,
                        item_view_canvas.width() as f64,
                        item_view_canvas.height() as f64,
                    );
                    preview_ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &game_item.sprite(),
                        0.0,
                        sy,
                        32.0,
                        32.0,
                        0.0,
                        0.0,
                        32.0,
                        32.0,
                    )?;
                }

                ctx.clear_rect(
                    0.0,
                    0.0,
                    item_view_canvas.width() as f64,
                    item_view_canvas.height() as f64,
                );
                ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &game_item.sprite(),
                    0.0,
                    sy,
                    32.0,
                    32.0,
                    0.0,
                    0.0,
                    32.0,
                    32.0,
                )?;
                original_draw_icon.call2(&game_item, &item_ctx, &canvas)
            },
        );
        game_item.set_draw_icon(&new_draw_icon);
    }

    #[cfg(not(feature = "ni"))]
    fn init_animation(&'static self, slot_type: SlotType, _item_id: Id) {
        //Could already be some from mouse_over events.
        if self[slot_type].lock_ref().original_draw.is_some() {
            return;
        }

        let original_draw_icon = JsValue::UNDEFINED.unchecked_into();

        self[slot_type].lock_mut().original_draw = Some(original_draw_icon);
    }

    fn occupied(&self, slot_type: SlotType) -> bool {
        self[slot_type].lock_ref().item_id.is_some()
    }

    fn get_by_id(&self, item_id: Id) -> Option<(&Mutable<UpgradeSlot>, SlotType)> {
        [
            SlotType::Single,
            SlotType::Armor,
            SlotType::Jewelry,
            SlotType::Weapons,
        ]
        .into_iter()
        .find_map(|slot_type| {
            let slot = &self[slot_type];

            slot.lock_ref()
                .item_id
                .is_some_and(|id| id == item_id)
                .then_some((slot, slot_type))
        })
    }

    fn draw_in_slot(&self, slot_type: SlotType, item_canvas: &HtmlCanvasElement) {
        debug_log!("drawing...");
        self.index(slot_type)
            .lock_ref()
            .canvas
            .as_ref()
            .unwrap_js()
            .get_context("2d")
            .unwrap_js()
            .unwrap_js()
            .unchecked_into::<web_sys::CanvasRenderingContext2d>()
            .draw_image_with_html_canvas_element(item_canvas, 0.0, 0.0)
            .unwrap_js();

        if slot_type == SlotType::Single {
            self.draw_preview(item_canvas);
        }
    }

    fn draw_preview(&self, canvas: &HtmlCanvasElement) {
        let preview_lock = self.single.preview_canvas.borrow();
        let preview_canvas = preview_lock.as_ref().unwrap_js();
        let context = preview_canvas
            .get_context("2d")
            .unwrap_js()
            .unwrap_js()
            .unchecked_into::<web_sys::CanvasRenderingContext2d>();
        context
            .draw_image_with_html_canvas_element(canvas, 0.0, 0.0)
            .unwrap_js();
    }

    fn clear_slot(&self, slot_type: SlotType) {
        let slot_lock = self[slot_type].lock_ref();
        // Already cleared
        // if slot_lock.item_id.is_none() && slot_lock.original_draw.is_none() {
        //     debug_log!(slot_lock.item_id.is_none(), slot_lock.original_draw.is_none());
        //     return;
        // }

        #[cfg(feature = "ni")]
        if let Some(original_draw) = slot_lock.original_draw.as_ref() {
            let item_id = slot_lock
                .item_id
                .or_else(|| SELECTING_ITEM.with(|s| s.hovering_over.item_id.get()))
                .unwrap_js();
            let game_item = get_engine()
                .items_manager()
                .unwrap_js()
                .get_item_by_id(item_id)
                .unwrap_js()
                .unchecked_into::<GameItem>();
            game_item.set_draw_icon(original_draw);
        }
        drop(slot_lock);

        self.clear_slot_icon(slot_type);

        self[slot_type].replace_with(|slot| UpgradeSlot {
            item_id: None,
            canvas: Some(slot.canvas.take().unwrap_js()),
            rarity: None,
            original_draw: None,
            current: None,
            max: None,
        });
    }

    fn clear_slot_icon(&self, slot_type: SlotType) {
        // debug_log!("CLEARING SLOT ICON");
        if slot_type == SlotType::Single {
            let preview_lock = self.single.preview_canvas.borrow();
            let preview_canvas = preview_lock.as_ref().unwrap_js();
            let context = preview_canvas
                .get_context("2d")
                .unwrap_js()
                .unwrap_js()
                .unchecked_into::<web_sys::CanvasRenderingContext2d>();
            context.clear_rect(
                0.0,
                0.0,
                preview_canvas.width() as f64,
                preview_canvas.height() as f64,
            );
        }

        let slot_lock = self[slot_type].lock_ref();
        let canvas = slot_lock.canvas.as_ref().unwrap_js();
        let context = canvas
            .get_context("2d")
            .unwrap_js()
            .unwrap_js()
            .unchecked_into::<web_sys::CanvasRenderingContext2d>();
        context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
enum SlotType {
    Single,
    Armor,
    Jewelry,
    Weapons,
}

impl TryFrom<SlotType> for EquipmentItemGroup {
    type Error = f64;

    fn try_from(value: SlotType) -> Result<Self, Self::Error> {
        match value {
            SlotType::Single => Err(err_code!(as_num)),
            SlotType::Armor => Ok(Self::Armor),
            SlotType::Jewelry => Ok(Self::Jewelry),
            SlotType::Weapons => Ok(Self::Weapons),
        }
    }
}

impl SlotType {
    fn get_attribute(self) -> &'static str {
        match self {
            Self::Single => throw_err_code!(),
            Self::Armor => "armor",
            Self::Jewelry => "jewelry",
            Self::Weapons => "weapons",
        }
    }
}

impl Index<SlotType> for ItemSlots {
    type Output = Mutable<UpgradeSlot>;

    // Required method
    fn index(&self, index: SlotType) -> &Self::Output {
        match index {
            SlotType::Single => &self.single.slot,
            SlotType::Armor => &self.group.armor,
            SlotType::Jewelry => &self.group.jewelry,
            SlotType::Weapons => &self.group.weapons,
        }
    }
}

#[derive(Setting, Clone)]
struct UpgradeButton {
    active: Mutable<bool>,
    unique: Mutable<bool>,
    heroic: Mutable<bool>,
    from_event: Mutable<bool>,
}

impl Default for UpgradeButton {
    fn default() -> Self {
        Self {
            active: Mutable::new(true),
            unique: Mutable::default(),
            heroic: Mutable::default(),
            from_event: Mutable::default(),
        }
    }
}

impl UpgradeButton {
    fn disabled_signal_factory(&self, settings: &'static Settings) -> impl Signal<Item = bool> {
        let stats_validator = settings.upgrade_button.stats_validator_factory();
        map_ref! {
            let items = Items::get().entries_cloned().to_signal_cloned(),
            let _ = settings.mode.signal(),
            let _ = settings.item_slots[SlotType::Single].signal_ref(|_| ()),
            let _ = settings.item_slots[SlotType::Armor].signal_ref(|_| ()),
            let _ = settings.item_slots[SlotType::Jewelry].signal_ref(|_| ()),
            let _ = settings.item_slots[SlotType::Weapons].signal_ref(|_| ()),
            let _ = self.unique.signal(),
            let _ = self.heroic.signal(),
            let _ = self.from_event.signal(),
            let _ = from_stream(to_stream(settings.item_types.signal_map())),
            let _ = settings.excluded_items.signal_ref(|_| ()) => {
                items.iter()
                    .filter(move |(_, item_data)| {
                        settings.filter_buffer_item(item_data, stats_validator)
                    })
                    .peekable()
                    .peek()
                    .is_none()
            }
        }
        .dedupe()
    }

    fn stats_validator_factory(&self) -> impl Fn(&ItemStats) -> bool + use<'_> + Copy {
        |stats| {
            let from_rarity = match stats.rarity {
                Rarity::Upgraded | Rarity::Legendary | Rarity::Artifact => false,
                Rarity::Unique if !self.unique.get() => false,
                Rarity::Heroic if !self.heroic.get() => false,
                _ => true,
            };
            let from_event = !stats.from_event || self.from_event.get();

            from_rarity && from_event
        }
    }

    fn on_click(&self, active_settings: &'static ActiveSettings, settings: &'static Settings) {
        let stats_validator = self.stats_validator_factory();
        let buffer: Vec<_> = Items::get()
            .lock_ref()
            .iter()
            .filter(|&(_, item_data)| settings.filter_buffer_item(item_data, stats_validator))
            .map(|(id, data)| (*id, data.clone()))
            .collect();

        //debug_log!("buffer length", buffer.len());
        if buffer.is_empty() {
            return;
        }

        let mut buffer_queue_lock = active_settings.buffer_queue.lock_mut();
        buffer_queue_lock.replace_cloned(buffer);

        //debug_log!("Buffer queue length:", buffer_queue_lock.len());

        drop(buffer_queue_lock);
        wasm_bindgen_futures::spawn_local(active_settings.clear_buffer(settings));
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum BufferMode {
    #[default]
    Fixed = 0,
    Dynamic = 1,
}

impl From<BufferMode> for bool {
    fn from(value: BufferMode) -> Self {
        match value {
            BufferMode::Fixed => false,
            BufferMode::Dynamic => true,
        }
    }
}

impl Not for BufferMode {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Fixed => Self::Dynamic,
            Self::Dynamic => Self::Fixed,
        }
    }
}

#[derive(Settings)]
struct Settings {
    /// Whether to upgrade single item or by group or both.
    mode: Mutable<UpgradingMode>,
    item_slots: ItemSlots,
    /// Types with which slotted items are going to be upgraded with.
    item_types: MutableBTreeMap<ItemClass, bool>,
    upgrade_button: UpgradeButton,
    excluded_items: ItemNameInput,
    buffer_size: Mutable<u8>,
    buffer_mode: Mutable<BufferMode>,
    common: Mutable<bool>,
    unique: Mutable<bool>,
    #[setting(skip)]
    hit_usages_limit: Cell<bool>,
    #[setting(skip)]
    usages: Mutable<UsagesPreview>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mutable::default(),
            item_slots: ItemSlots::default(),
            item_types: MutableBTreeMap::from(BTreeMap::from([
                (ItemClass::OneHandWeapon, true),
                (ItemClass::TwoHandWeapon, true),
                (ItemClass::OneAndHalfHandWeapon, true),
                (ItemClass::DistanceWeapon, true),
                (ItemClass::HelpWeapon, true),
                (ItemClass::WandWeapon, true),
                (ItemClass::OrbWeapon, true),
                (ItemClass::Armor, true),
                (ItemClass::Helmet, true),
                (ItemClass::Boots, true),
                (ItemClass::Gloves, true),
                (ItemClass::Ring, true),
                (ItemClass::Necklace, true),
                (ItemClass::Shield, true),
                (ItemClass::Upgrade, false),
                (ItemClass::Quiver, true),
            ])),
            upgrade_button: UpgradeButton::default(),
            excluded_items: ItemNameInput::default(),
            buffer_size: Mutable::new(5),
            buffer_mode: Mutable::new(BufferMode::Fixed),
            common: Mutable::new(true),
            unique: Mutable::default(),
            hit_usages_limit: Cell::default(),
            usages: Mutable::default(),
        }
    }
}

impl Settings {
    /// Calculates the buffer upper bound based on the current `BufferMode`.
    fn buffer_limit_signal(
        &self,
        active_settings: &ActiveSettings,
    ) -> impl Signal<Item = u8> + use<'_> {
        map_ref! {
            let current_mode = self.buffer_mode.signal(),
            let current_size = self.buffer_size.signal(),
            let free_slots = FreeEquipmentSlotsSignal::new(),
            let buffer_queue_len = active_settings.buffer_queue.signal_vec_cloned().len() => {
                //let free_slots = get_engine().hero_equipment().unwrap_js().get_free_slots();
                //let buffre_queue_lock = active_settings.buffer_queue.lock_ref();
                //let buffer_queue_len = buffre_queue_lock.len();
                match current_mode {
                    _ if *free_slots == 0 => 1,
                    BufferMode::Fixed  => match *free_slots >= *current_size {
                        true => *current_size,
                        false => *free_slots,
                    },
                    BufferMode::Dynamic => (*free_slots + *buffer_queue_len as u8).checked_sub(*current_size).unwrap_or(1).max(1),
                }
            }
        }
    }

    fn filter_buffer_item<C>(&self, item_data: &Item, stat_validator: C) -> bool
    where
        C: FnOnce(&ItemStats) -> bool,
    {
        if item_data
            .loc
            .as_ref()
            .is_none_or(|loc| loc.as_str() != intern(s!("g")))
        {
            return false;
        }
        let Some(item_name) = item_data.name.as_ref() else {
            return false;
        };
        if self
            .excluded_items
            .lock_ref()
            .contains(&item_name.to_lowercase())
        {
            return false;
        }

        let Some(stats) = item_data.parse_stats() else {
            return false;
        };

        if stats.artisan_worthless || stats.bonus_reselect || stats.personal {
            return false;
        }
        // TODO: Check for each slotted item rarity when calculating ingredients?
        if stats.target_rarity.is_some() {
            return false;
        }

        if !stat_validator(&stats) {
            return false;
        }

        let Some(item_class) = item_data.cl else {
            //debug_log!("No item_class");
            return false;
        };

        if stats
            .bind
            .is_none_or(|bind_type| bind_type != BindType::Binds)
            && item_class != ItemClass::Upgrade
        {
            return false;
        }
        //Check whether user wants to destroy items from this class.
        if self
            .item_types
            .lock_ref()
            .get(&item_class)
            .is_none_or(|burn_from_class| !burn_from_class)
        {
            return false;
        }

        let current_mode = self.mode.get();
        //Anything matching allowed item_types can go into SlotType::Single
        if current_mode == UpgradingMode::Single {
            return self.item_slots.occupied(SlotType::Single);
        }
        if current_mode == UpgradingMode::Hybrid && self.item_slots.occupied(SlotType::Single) {
            return true;
        }

        //Check whether item matches any occupied slot group.
        [SlotType::Armor, SlotType::Jewelry, SlotType::Weapons]
            .into_iter()
            .any(|slot_type| {
                self.item_slots.occupied(slot_type)
                    && item_class.is_in_group(slot_type.try_into().unwrap_js())
            })
    }

    fn observe_enhancement_event(&'static self) -> JsResult<()> {
        let on_enhancement = |socket_response: &Response| {
            // FIXME: Temporary solution. To fix it:
            // If an artisanship intersector is run before this (in this case it is look `artisanship_intersector_factory`)
            // and it deletes the property don't emit the `Enhancement` event.
            let Some(usages_preview) = socket_response
                .enhancement
                .as_ref()
                .and_then(|e| e.usages_preview.as_ref())
            else {
                //debug_log!(msg.clone());
                return Ok(());
            };
            let mut usages_lock = self.usages.lock_mut();

            if usages_preview.count.is_some() {
                usages_lock.count = usages_preview.count
            } else {
                return Err(err_code!());
            }
            if usages_preview.limit.is_some() {
                usages_lock.limit = usages_preview.limit
            } else {
                return Err(err_code!());
            }

            Ok(())
        };

        Emitter::register_on(EmitterEvent::Enhancement, move |socket_response| {
            let res = on_enhancement(socket_response);
            Box::pin(async move { res })
        })?;

        Ok(())
    }

    /// Validates whether an item can be removed from buffer automatically.
    fn can_destroy_item(&self, item_data: &Item) -> bool {
        self.filter_buffer_item(item_data, |stats| {
            let from_rarity = match stats.rarity {
                Rarity::Common => self.common.get(),
                Rarity::Unique => self.unique.get(),
                _ => false,
            };
            from_rarity && !stats.from_event
        })
    }

    fn intercept_artisanship<'a>(
        &'a self,
        socket_response: &'a mut Response,
    ) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'a>> {
        Box::pin(async move {
            if socket_response.artisanship.take().is_none() {
                return Ok(());
            }

            let Some(usages_preview) = socket_response
                .enhancement
                .take()
                .and_then(|enhancement| enhancement.usages_preview)
            else {
                return Ok(());
            };
            let mut usages_lock = self.usages.lock_mut();

            usages_lock.count = Some(usages_preview.count.ok_or_else(|| err_code!())?);
            usages_lock.limit = Some(usages_preview.limit.ok_or_else(|| err_code!())?);

            Ok(())
        })
    }

    fn item_not_found(&self, slot_type: SlotType) -> bool {
        let Some(item_id) = self.item_slots[slot_type].lock_ref().item_id else {
            return false;
        };

        matches!(
            document().query_selector(&format!(
                ".inventory-item.item-id-{item_id} .icon.canvas-icon"
            )),
            Ok(None) | Err(_)
        )
    }

    fn item_not_found_signal(&'static self, slot_type: SlotType) -> impl Signal<Item = bool> {
        ANIMATION_ACTIVE.with(|animation_active| {
            animation_active.signal_ref(move |state| {
                matches!(state, ItemState::Tip { item_id } if self.item_slots[slot_type].lock_ref().item_id.is_some_and(|slotted_id| slotted_id == *item_id))
            })
        })
    }

    async fn set_item_in_slot(
        &'static self,
        slot_type: SlotType,
        item_id: Id,
        rarity: Rarity,
    ) -> JsResult<()> {
        let _empty_slot = self.item_slots[slot_type].replace_with(|slot| UpgradeSlot {
            item_id: Some(item_id),
            canvas: slot.canvas.take(),
            rarity: Some(rarity),
            original_draw: slot.original_draw.take(),
            current: None,
            max: None,
        });
        self.item_slots.init_animation(slot_type, item_id);
        let item_container = ItemContainer::find(item_id)?.ok_or_else(|| err_code!())?;
        init_animation(&item_container, item_id)?;

        let event = EmitterEvent::Enhancement;
        let callback_id = Emitter::intercept_once(event, |socket_response| {
            self.intercept_artisanship(socket_response)
        })?;

        __send_task("artisanship&action=open").await?;

        //debug_log!("before waiting for receiver");
        Emitter::wait_for_intercept(event, callback_id).await?;
        //debug_log!("after waiting for receiver");

        let lock_manager = get_engine().lock_manager().ok_or_else(|| err_code!())?;
        lock_manager.add_lock("crafting")?;
        //debug_log!("before add item");

        self.item_slots.init_slot_progress(slot_type).await?;
        //debug_log!("after add item");

        lock_manager.remove_lock("crafting")
    }

    fn rarity_signal(&self, slot_type: SlotType) -> impl Signal<Item = Option<Rarity>> {
        self.item_slots[slot_type].signal_ref(|upgrade_slot| upgrade_slot.rarity)
    }

    fn slot_occupied_signal(&self, slot_type: SlotType) -> impl Signal<Item = bool> {
        self.item_slots[slot_type].signal_ref(|slot| slot.item_id.is_some())
    }

    async fn init_usages_and_progress(&'static self) -> JsResult<()> {
        let event = EmitterEvent::Enhancement;
        let callback_id = Emitter::intercept_once(event, |socket_response| {
            self.intercept_artisanship(socket_response)
        })?;

        __send_task("artisanship&action=open").await?;
        Emitter::wait_for_intercept(event, callback_id).await?;

        let lock_manager = get_engine().lock_manager().ok_or_else(|| err_code!())?;
        // Prevent hero from moving during the addon's initialization.
        lock_manager.add_lock("crafting")?;

        self.item_slots.init_slot_progress(SlotType::Single).await?;
        // Add a delay at the end in order to not receive a warning response.
        delay_range(100, 200).await;
        self.item_slots.init_slot_progress(SlotType::Armor).await?;
        delay_range(100, 200).await;
        self.item_slots
            .init_slot_progress(SlotType::Jewelry)
            .await?;
        delay_range(100, 200).await;
        self.item_slots
            .init_slot_progress(SlotType::Weapons)
            .await?;

        lock_manager.remove_lock("crafting")
    }

    /// Checks if the usages limit was hit on every usages change.
    fn observe_artisanship_usages(&'static self) {
        let future = self.usages.signal().for_each(|preview| {
            if let Some(count) = preview.count
                && let Some(limit) = preview.limit
                && count >= limit
            {
                self.hit_usages_limit.set(true);
            }

            async {}
        });
        wasm_bindgen_futures::spawn_local(future);
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
                let Some(game_item) = items_manager
                    .get_item_by_id(item_id_num)
                    .map(JsCast::unchecked_into::<GameItem>)
                else {
                    // TODO: Is this correct?
                    //Has to do something with changing location when leaving stasis.
                    return Ok(());
                };
                let Some((slot, slot_type)) = self.item_slots.get_by_id(item_id_num) else {
                    return Ok(());
                };

                let item_canvas = game_item.get_canvas_icon().get_canvas_elem();
                let item_data: Item = serde_wasm_bindgen::from_value(game_item.into())
                    .map_err(map_err!(from))?;
                let rarity = item_data.parse_stats().map(|stats| stats.rarity);

                self.item_slots.clear_slot_icon(slot_type);
                self.item_slots.draw_in_slot(slot_type, &item_canvas);
                self.item_slots.init_animation(slot_type, item_id_num);
                slot.lock_mut().rarity = rarity;

                if let Some(item_container) = ItemContainer::find(item_id_num)? {
                    init_animation(&item_container, item_id_num)?;
                };

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
                if self.item_slots.get_by_id(item_id_num).is_none() {
                    return Ok(());
                };

                let items_lock = Items::get().lock_ref();
                let item = items_lock.get(&item_id_num).unwrap_js();
                let rarity = item.parse_stats().map(|stats| stats.rarity);
                wasm_bindgen_futures::spawn_local(async move {
                    loop {
                        delay(50).await;
                        let item_img = document()
                            .query_selector(&format!("#item{} > img", item_id_num))
                            .unwrap_js()
                            .unwrap_js()
                            .unchecked_into::<web_sys::HtmlCanvasElement>();
                        if !item_img.has_attribute("dest") {
                            continue;
                        }
                        common::debug_log!("item_img:", &item_img);

                        let Some((slot, slot_type)) = self.item_slots.get_by_id(item_id_num) else {
                            return;
                        };
                        self.item_slots.clear_slot_icon(slot_type);
                        self.item_slots.draw_in_slot(slot_type, &item_img);
                        self.item_slots.init_animation(slot_type, item_id_num);
                        slot.lock_mut().rarity = rarity;

                        if let Some(item_container) = ItemContainer::find(item_id_num).unwrap_js() {
                            init_animation(&item_container, item_id_num).unwrap_js();
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

    #[cfg(feature = "ni")]
    fn observe_new_item(&'static self) {
        let items_mark_manager = get_engine().items_mark_manager().unwrap_js();
        let original_new_item = items_mark_manager.get_new_item().unwrap_js();
        let new_new_item = closure!(
            { let items_mark_manager = items_mark_manager.clone() },
            move |game_item: GameItem| -> DefaultResult {
                let res = original_new_item.call1(&items_mark_manager, &game_item);

                if !game_item.img_loaded().is_some_and(|loaded| loaded) {
                    return res;
                }

                let item_id = game_item.id().unchecked_into_f64() as Id;
                let Some((slot, slot_type)) = self.item_slots.get_by_id(item_id) else {
                    return res;
                };
                if slot.lock_ref().original_draw.is_some() {
                    return res;
                }

                let item_canvas = game_item.get_canvas_icon().get_canvas_elem();
                self.item_slots.clear_slot_icon(slot_type);
                self.item_slots.draw_in_slot(slot_type, &item_canvas);
                self.item_slots.init_animation(slot_type, item_id);
                let item_data: Item = serde_wasm_bindgen::from_value(game_item.into()).map_err(map_err!(from))?;
                let rarity = item_data.parse_stats().map(|stats| stats.rarity);
                slot.lock_mut().rarity = rarity;

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
                let res = original_new_inventory_items.call2(&hero_equipment, &game_item, &finish);

                if !game_item.img_loaded().is_some_and(|loaded| loaded) {
                    return res;
                }

                let item_id = game_item.id().unchecked_into_f64() as Id;
                let Some((slot, slot_type)) = self.item_slots.get_by_id(item_id) else {
                    return res;
                };
                if slot.lock_ref().original_draw.is_some() {
                    return res;
                }

                let item_canvas = game_item.get_canvas_icon().get_canvas_elem();
                self.item_slots.clear_slot_icon(slot_type);
                self.item_slots.draw_in_slot(slot_type, &item_canvas);
                self.item_slots.init_animation(slot_type, item_id);

                let item_container = ItemContainer::find(item_id).unwrap_js().unwrap_js();
                init_animation(&item_container, item_id).unwrap_js();

                let item_data: Item = serde_wasm_bindgen::from_value(game_item.into()).map_err(map_err!(from))?;
                let rarity = item_data.parse_stats().map(|stats| stats.rarity);
                slot.lock_mut().rarity = rarity;

                res
            }
        );
        hero_equipment.set_new_inventory_items(&new_new_inventory_items);
    }
}

#[derive(ActiveSettings)]
struct ActiveSettings {
    #[setting(skip)]
    loot_queue: MutableVec<Id>,
    #[setting(skip)]
    buffer_queue: MutableVec<(Id, Item)>,
}

impl Default for ActiveSettings {
    fn default() -> Self {
        Self {
            loot_queue: MutableVec::with_capacity(30),
            buffer_queue: MutableVec::with_capacity(130),
        }
    }
}

pub(crate) fn init() -> JsResult<()> {
    let settings = Settings::new(ADDON_NAME);
    settings.observe_artisanship_usages();
    wasm_bindgen_futures::spawn_local(async move {
        // TODO: Schedule addon deactivation on Err.
        if let Err(err_code) = settings.init_usages_and_progress().await {
            console_error!(err_code)
        }
    });
    settings.observe_update_placeholder()?;
    #[cfg(feature = "ni")]
    {
        settings.observe_new_inventory_items();
        settings.observe_new_item();
    }
    settings.observe_enhancement_event()?;

    let active_settings = ActiveSettings::new(ADDON_NAME);

    active_settings.observe_loot_event(settings)?;
    active_settings.observe_item_event(settings)?;

    html::init(settings, active_settings)
}

impl ActiveSettings {
    fn observe_loot_event(&'static self, settings: &'static Settings) -> JsResult<()> {
        let on_loot = move |socket_response: &Response| {
            if !Addons::is_active(ADDON_NAME) || settings.hit_usages_limit.get() {
                return Ok(());
            }

            let mut loot_queue_lock = self.loot_queue.lock_mut();
            let loot = socket_response.loot.as_ref().ok_or_else(|| err_code!())?;
            if loot.init.is_none() {
                return Ok(());
            }
            if loot
                .source
                .as_deref()
                .is_none_or(|source| source != "fight")
            {
                return Ok(());
            }
            let Some(states) = loot.states.as_ref() else {
                debug_log!(@f "{socket_response:?}");
                return Ok(());
            };

            for (item_id, want_state) in states {
                if *want_state == LootWantState::NotWant {
                    loot_queue_lock.retain(|queue_entry| queue_entry != item_id);
                    continue;
                }

                loot_queue_lock.push(*item_id);

                // Fallback for removing enqueued loot after 20s.
                // FIXME: CAN PANIC
                let item_id = *item_id;
                let future = async move {
                    delay(20_000).await;

                    self.loot_queue
                        .lock_mut()
                        .retain(|queue_entry| *queue_entry != item_id);
                };
                wasm_bindgen_futures::spawn_local(future);
            }

            debug_log!(@f "LOOT QUEUE UPDATED: {:?}", loot_queue_lock);

            Ok(())
        };

        Emitter::register_on(EmitterEvent::Loot, move |socket_response| {
            let res = on_loot(socket_response);
            Box::pin(async move { res })
        })?;

        Ok(())
    }

    fn observe_item_event(&'static self, settings: &'static Settings) -> JsResult<()> {
        let on_item = move |socket_response: &Response| {
            if self.loot_queue.lock_ref().is_empty() && socket_response.f.is_none() {
                return Ok(());
            }

            let items_map = socket_response.item.as_ref().ok_or_else(|| err_code!())?;
            common::debug_log!("BEFORE LOCK LOOT QUEUE");
            let mut loot_queue_lock = self.loot_queue.lock_mut();
            common::debug_log!("BEFORE LOCK BUFFER QUEUE");
            let mut buffer_queue_lock = self.buffer_queue.lock_mut();
            common::debug_log!("AFTER LOCK BUFFER QUEUE");
            // FIXME: This is not a correct implementation, cause an item might just get moved not
            // created in the same tick as the loot gets received.
            let free_slots = get_engine()
                .hero_equipment()
                .unwrap_js()
                .get_free_slots()
                .saturating_sub(items_map.len() as u8);

            match socket_response.f.is_some() {
                // If fight started or ended clear the loot_queue and push all items into the buffer_queue.
                true => {
                    loot_queue_lock.clear();
                    items_map.iter().for_each(|(item_id, item_data)| {
                        buffer_queue_lock.push_cloned((*item_id, item_data.clone()));
                    });
                }
                // Otherwise only push items from the loot queue.
                false => items_map.iter().for_each(|(item_id, item_data)| {
                    let original_len = loot_queue_lock.len();

                    loot_queue_lock.retain(|queue_entry| queue_entry != item_id);

                    if loot_queue_lock.len() < original_len {
                        buffer_queue_lock.push_cloned((*item_id, item_data.clone()));
                    }
                }),
            }

            drop(buffer_queue_lock);
            self.update_buffer(settings);

            debug_log!("Buffer queue length:", self.buffer_queue.lock_ref().len());

            //// No free slots in equipment
            //if get_engine().hero_equipment().unwrap_js().get_free_slots() == 0 {
            //    return Ok(());
            //}

            common::debug_log!("BEFORE LOCK BUFFER QUEUE AGAIN");
            let mut buffer_queue_lock = self.buffer_queue.lock_mut();
            common::debug_log!("AFTER LOCK BUFFER QUEUE AGAIN");
            let buffer_queue_len = buffer_queue_lock.len();
            let usages_lock = settings.usages.lock_ref();

            if let Some(usages_limit) = usages_lock.limit {
                let usages_left = usages_limit
                    .saturating_sub(usages_lock.count.unwrap_js())
                    .into();
                if buffer_queue_len >= usages_left {
                    buffer_queue_lock.truncate(usages_left);
                }
            }
            if buffer_queue_lock.is_empty() {
                return Ok(());
            }

            let at_capacity = match settings.buffer_mode.get() {
                BufferMode::Fixed => buffer_queue_len >= settings.buffer_size.get().into(),
                BufferMode::Dynamic => {
                    buffer_queue_len
                        >= (free_slots + buffer_queue_len as u8)
                            .saturating_sub(settings.buffer_size.get())
                            .into()
                }
            };
            //common::debug_log!(buffer_queue_len, free_slots, settings.buffer_size.get());

            if !at_capacity {
                return Ok(());
            }

            drop(usages_lock);
            drop(buffer_queue_lock);

            wasm_bindgen_futures::spawn_local(self.clear_buffer(settings));

            Ok(())
        };

        Emitter::register_on(EmitterEvent::Item, move |socket_response| {
            let res = on_item(socket_response);
            Box::pin(async move { res })
        })?;

        Ok(())
    }

    ///Retain items that can be destroyed according to current settings and are still in the eq.
    fn update_buffer(&self, settings: &Settings) {
        self.buffer_queue
            .lock_mut()
            .retain(|(_, item_data)| settings.can_destroy_item(item_data));
    }

    async fn clear_buffer(&self, settings: &'static Settings) {
        let event = EmitterEvent::Enhancement;
        let callback_id = Emitter::intercept_once(event, |socket_response| {
            settings.intercept_artisanship(socket_response)
        })
        .unwrap_js();

        if let Err(_err) = __send_task("artisanship&action=open").await {
            debug_log!(_err);
        }
        //debug_log!("before waiting for receiver");
        Emitter::wait_for_intercept(event, callback_id)
            .await
            .unwrap_js();
        //debug_log!("after waiting for receiver");

        let lock_manager = get_engine().lock_manager().unwrap_js();
        lock_manager.add_lock("crafting").unwrap_js();
        debug_log!("lock_manager be4:", &lock_manager.lock_list());

        //let _ = message(intern(s!("[MDMA::RS] Rozpoczynam czyszczenie bufora...")));
        match settings.mode.get() {
            //Buffer only fills up if slot is occupied
            UpgradingMode::Single => {
                self.intercept_progress(SlotType::Single, settings).await;
            }
            UpgradingMode::Group => {
                let slot_types = [SlotType::Armor, SlotType::Jewelry, SlotType::Weapons];

                for slot_type in slot_types {
                    if settings.item_slots.occupied(slot_type) {
                        self.intercept_progress(slot_type, settings).await;
                    }
                }
            }
            UpgradingMode::Hybrid => {
                let slot_types = [
                    SlotType::Armor,
                    SlotType::Jewelry,
                    SlotType::Weapons,
                    SlotType::Single,
                ];

                for slot_type in slot_types {
                    if settings.item_slots.occupied(slot_type) {
                        self.intercept_progress(slot_type, settings).await;
                    }
                }
            }
        }

        debug_log!("lock_manager after:", &lock_manager.lock_list());
        lock_manager.remove_lock("crafting").unwrap_js();
        //let _ = message(intern(s!("[MDMA::RS] Bufor wyczyszczony pomyślnie.")));
    }

    ///`SlotType` passed into this function has to have an occupied slot.
    ///`buffer_queue` has to be pre-filtered to only include items that will get burnt.
    async fn intercept_progress(&self, slot_type: SlotType, settings: &'static Settings) {
        let mut buffer_queue_lock = self.buffer_queue.lock_mut();

        if buffer_queue_lock.is_empty() {
            return;
        }

        let ingredients = buffer_queue_lock
            .iter()
            .filter_map(|(item_id, item_data)| {
                if slot_type == SlotType::Single {
                    return Some(*item_id);
                }

                let item_class = item_data.cl?;

                item_class
                    .is_in_group(slot_type.try_into().unwrap_js())
                    .then_some(*item_id)
            })
            .collect::<Vec<_>>();

        if ingredients.is_empty() {
            return;
        }

        if slot_type == SlotType::Single {
            //Clear buffer.
            debug_assert!(ingredients.len() == buffer_queue_lock.len());
            buffer_queue_lock.clear()
        } else {
            //Remove all non-ingredients.
            buffer_queue_lock.retain(|(item_id, _)| {
                !ingredients
                    .iter()
                    .any(|ingredient_id| *ingredient_id == *item_id)
            })
        }

        drop(buffer_queue_lock);

        let slotted_item_id = settings.item_slots[slot_type]
            .lock_ref()
            .item_id
            .unwrap_js();
        let ingredients = ingredients
            .into_iter()
            .filter(|ingredient_id| *ingredient_id != slotted_item_id)
            .map(|ingredient_id| ingredient_id.to_string())
            .collect::<Vec<String>>();
        let ingredient_chunks = ingredients.chunks(25).map(|chunk| chunk.join(","));

        for ingredients in ingredient_chunks {
            let (tx, rx) = futures::channel::oneshot::channel::<bool>();
            Emitter::intercept_once(EmitterEvent::Enhancement, move |socket_response| {
                Box::pin(async move {
                    let items = socket_response.item.as_mut().ok_or_else(|| err_code!())?;
                    let mut slot_lock = settings.item_slots[slot_type].lock_mut();
                    let item_id = slot_lock.item_id.ok_or_else(|| err_code!())?;
                    let _item_tpl = items.remove(&-item_id)
                        .ok_or_else(|| err_code!())?;
                    let item_name = items
                        .get(&item_id)
                        .ok_or_else(|| err_code!())?
                        .name
                        .as_ref()
                        .ok_or_else(|| err_code!())?;

                    let enhancement = socket_response
                        .enhancement
                        .take()
                        .ok_or_else(|| err_code!())?;
                    let usages_preview = enhancement.usages_preview.ok_or_else(|| err_code!())?;
                    let mut usages_lock = settings.usages.lock_mut();
                    let count = usages_preview.count.ok_or_else(|| err_code!())?;
                    let limit = usages_preview.limit.ok_or_else(|| err_code!())?;

                    usages_lock.count = Some(count);
                    usages_lock.limit = Some(limit);

                    let Some(enhance_progress) = enhancement.progressing else {
                        drop(slot_lock);
                        settings.item_slots.clear_slot(slot_type);
                        let _ = message(&format!(
                            r#"[MDMA::RS] Zwalniam slot ulepszania ze względu na poziom ulepszenia przedmiotu "{item_name}"..."#
                        ));

                        tx.send(false).map_err(|_| err_code!())?;
                        return Ok(());
                    };
                    let new_current = enhance_progress.current.ok_or_else(|| err_code!())?;
                    let new_max = enhance_progress.max.ok_or_else(|| err_code!())?;

                    debug_log!(@f "{new_current}/{new_max}");
                    slot_lock.current = Some(new_current);
                    slot_lock.max = Some(new_max);
                    let _ = message(&format!(
                        r#"[MDMA::RS] Ulepszono "{item_name}" do {:.2}%"#,
                        (new_current as f64 / new_max as f64) * 100.0
                    ));

                    tx.send(true).map_err(|_| err_code!())?;

                    Ok(())
                })
            }).unwrap_js();
            __send_task(&format!("enhancement&action=progress&item={slotted_item_id}&ingredients={ingredients}&answer1001012=1&answer1001015=1")).await.unwrap_js();
            //debug_log!("before waiting for receiver");
            if !rx.await.unwrap_js() {
                debug_log!("break on none for receiver");
                break;
            }
            //debug_log!("after waiting for receiver");
        }
    }
}
