use std::{cell::RefCell, collections::BTreeMap, fmt};

use discard::Discard;
use dominator::DomHandle;
use wasm_bindgen::JsCast;
use web_sys::HtmlDivElement;

#[cfg(feature = "ni")]
use crate::bindings::engine::interface::JQueryObject;
use crate::prelude::*;

thread_local! {
    pub static DISABLED_ITEMS: RefCell<DisabledItems> = const { RefCell::new(DisabledItems::new()) };
}

pub struct DisabledItems {
    enhancement: BTreeMap<Id, DomHandle>,
}

impl fmt::Debug for DisabledItems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DisabledItems")
            .field("enhancement", &self.enhancement.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(not(feature = "ni"))]
static ITEM_DISABLE_CLASS: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    dominator::class! {
        .style_important("width", "15px")
        .style_important("height", "15px")
        .style_important("position", "absolute")
        .style_important("left", "50%")
        .style_important("top", "50%")
        .style_important("transform", "translate(-50%, -50%)")
        .style_important("background", "url(https://experimental.margonem.pl/img/X-blackoutline.gif)")
    }
});

impl DisabledItems {
    const fn new() -> Self {
        Self {
            enhancement: BTreeMap::new(),
        }
    }

    pub fn disable_items(&mut self) {
        let items_lock = Items::get().lock_ref();
        let items = items_lock
            .iter()
            .filter_map(|(item_id, item)| Self::should_disable_item(item).then_some(*item_id));

        for item_id in items {
            debug_log!("itemid", item_id);
            let item_bag_view = match Self::get_bag_view(item_id) {
                Ok(view) => view,
                Err(err_code) => {
                    console_error!(err_code);
                    return;
                }
            };
            if item_bag_view.has_attribute("bag") {
                continue;
            }

            item_bag_view
                .class_list()
                .add_2("EHNAHCE-disable", "disable-item-mark")
                .unwrap_js();

            #[cfg(not(feature = "ni"))]
            let in_bag = item_bag_view
                .parent_element()
                .is_some_and(|parent| parent.id() == "bag");
            #[cfg(not(feature = "ni"))]
            if !in_bag {
                item_bag_view
                    .query_selector("img")
                    .unwrap_js()
                    .unwrap_js()
                    .unchecked_into::<web_sys::HtmlImageElement>()
                    .style()
                    .set_property("opacity", "0.3")
                    .unwrap_js();
            }

            let disable_icon = dominator::html!("div", {
                .class("disable-icon")
                .dominator::with_cfg!(not(feature = "ni"), {
                    .apply_if(!in_bag, |b| b.class(&*ITEM_DISABLE_CLASS))
                })
            });
            let dom_handle = dominator::append_dom(&item_bag_view, disable_icon);

            self.enhancement.insert(item_id, dom_handle);
        }
    }

    #[cfg(feature = "ni")]
    fn get_bag_view(item_id: i32) -> JsResult<HtmlDivElement> {
        get_engine()
            .items_manager()
            .ok_or_else(|| err_code!())?
            .get_all_views_by_id_and_view_name(item_id, "BAG_VIEW")?
            .get(0)
            .unchecked_into::<JQueryObject>()
            .get(0)
            .dyn_into::<HtmlDivElement>()
            .map_err(|_| err_code!())
    }

    #[cfg(not(feature = "ni"))]
    fn get_bag_view(item_id: i32) -> JsResult<HtmlDivElement> {
        document()
            .get_element_by_id(&format!("item{item_id}"))
            .ok_or_else(|| err_code!())?
            .dyn_into::<HtmlDivElement>()
            .map_err(|_| err_code!())
    }

    fn should_disable_item(item: &Item) -> bool {
        if item.loc.as_deref() != Some("g") {
            return false;
        }

        let Some(item_cl) = item.cl else {
            return true;
        };

        let from_cl = !matches!(
            item_cl,
            ItemClass::OneHandWeapon
                | ItemClass::TwoHandWeapon
                | ItemClass::OneAndHalfHandWeapon
                | ItemClass::DistanceWeapon
                | ItemClass::HelpWeapon
                | ItemClass::WandWeapon
                | ItemClass::OrbWeapon
                | ItemClass::Armor
                | ItemClass::Helmet
                | ItemClass::Boots
                | ItemClass::Gloves
                | ItemClass::Ring
                | ItemClass::Necklace
                | ItemClass::Shield
                | ItemClass::Quiver
        );
        if from_cl {
            return true;
        }

        let Some(item_stats) = item.parse_stats() else {
            return true;
        };

        // We also return true if there is no lvl requirement.
        if item_stats.lvl < Some(20) {
            return true;
        }
        if item_stats.cursed {
            return true;
        }
        if item_stats.enhancement_upgrade_lvl == Some(5) {
            return true;
        }

        false
    }

    pub fn end_disable_items(&mut self) {
        while let Some((item_id, dom_handle)) = self.enhancement.pop_first() {
            let item_bag_view = match Self::get_bag_view(item_id) {
                Ok(view) => view,
                Err(err_code) => {
                    console_error!(err_code);
                    return;
                }
            };

            item_bag_view
                .class_list()
                .remove_2("EHNAHCE-disable", "disable-item-mark")
                .unwrap_js();

            #[cfg(not(feature = "ni"))]
            let in_bag = item_bag_view
                .parent_element()
                .is_some_and(|parent| parent.id() == "bag");
            #[cfg(not(feature = "ni"))]
            if !in_bag {
                item_bag_view
                    .query_selector("img")
                    .unwrap_js()
                    .unwrap_js()
                    .unchecked_into::<web_sys::HtmlImageElement>()
                    .style()
                    .remove_property("opacity")
                    .unwrap_js();
            }

            dom_handle.discard();
        }
    }
}
