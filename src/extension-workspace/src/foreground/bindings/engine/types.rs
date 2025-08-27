use std::fmt;

use common::err_code;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EquipmentSlot {
    InBag = 0,
    Head = 1,
    Finger = 2,
    Necklace = 3,
    Gloves = 4,
    MainWeapon = 5,
    Armor = 6,
    WeaponShieldOrArrow = 7,
    Shoes = 8,
    Purse = 9,
    Bless = 10,
    FirstBagSlot = 20,
    SecondBagSlot = 21,
    ThirdBagSlot = 22,
    SpecialBagSlot = 26,
    Undefined,
}

impl From<u8> for EquipmentSlot {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::InBag,
            1 => Self::Head,
            2 => Self::Finger,
            3 => Self::Necklace,
            4 => Self::Gloves,
            5 => Self::MainWeapon,
            6 => Self::Armor,
            7 => Self::WeaponShieldOrArrow,
            8 => Self::Shoes,
            9 => Self::Purse,
            10 => Self::Bless,
            20 => Self::FirstBagSlot,
            21 => Self::SecondBagSlot,
            22 => Self::ThirdBagSlot,
            26 => Self::SpecialBagSlot,
            _ => Self::Undefined,
        }
    }
}

#[derive(PartialEq)]
pub(crate) enum MapMode {
    NonPvp = 0,
    AgreePvp = 1,
    Pvp = 2,
    InstanceSolo = 3,
    Arena = 4,
    InstanceGrp = 5,
}

impl TryFrom<u8> for MapMode {
    type Error = JsValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NonPvp),
            1 => Ok(Self::AgreePvp),
            2 => Ok(Self::Pvp),
            3 => Ok(Self::InstanceSolo),
            4 => Ok(Self::Arena),
            5 => Ok(Self::InstanceGrp),
            _ => Err(err_code!()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum ItemClass {
    OneHandWeapon = 1,
    TwoHandWeapon = 2,
    OneAndHalfHandWeapon = 3,
    DistanceWeapon = 4,
    HelpWeapon = 5,
    WandWeapon = 6,
    OrbWeapon = 7,
    Armor = 8,
    Helmet = 9,
    Boots = 10,
    Gloves = 11,
    Ring = 12,
    Necklace = 13,
    Shield = 14,
    Neutral = 15,
    Consume = 16,
    Gold = 17,
    Keys = 18,
    Quest = 19,
    Renewable = 20,
    Arrows = 21,
    Talisman = 22,
    Book = 23,
    Bag = 24,
    Bless = 25,
    Upgrade = 26,
    Recipe = 27,
    Coinage = 28,
    Quiver = 29,
    Outfits = 30,
    Pets = 31,
    Teleports = 32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentItemGroup {
    Weapons,
    Jewelry,
    Armor,
}

impl ItemClass {
    pub fn is_in_group(self, group: EquipmentItemGroup) -> bool {
        match group {
            EquipmentItemGroup::Jewelry => matches!(self, ItemClass::Ring | ItemClass::Necklace),
            EquipmentItemGroup::Armor => matches!(
                self,
                ItemClass::Armor
                    | ItemClass::Shield
                    | ItemClass::Helmet
                    | ItemClass::Gloves
                    | ItemClass::Boots
            ),
            EquipmentItemGroup::Weapons => matches!(
                self,
                ItemClass::OneHandWeapon
                    | ItemClass::TwoHandWeapon
                    | ItemClass::OneAndHalfHandWeapon
                    | ItemClass::DistanceWeapon
                    | ItemClass::HelpWeapon
                    | ItemClass::WandWeapon
                    | ItemClass::OrbWeapon
                    | ItemClass::Quiver
            ),
        }
    }

    pub fn to_str_pretty(self) -> &'static str {
        match self {
            ItemClass::OneHandWeapon => "Broń jednoręczna",
            ItemClass::TwoHandWeapon => "Broń dwuręczna",
            ItemClass::OneAndHalfHandWeapon => "Broń półtoraręczna",
            ItemClass::DistanceWeapon => "Broń dystansowa",
            ItemClass::HelpWeapon => "Broń pomocnicza",
            ItemClass::WandWeapon => "Różdżki magiczne",
            ItemClass::OrbWeapon => "Orby magiczne",
            ItemClass::Armor => "Zbroje",
            ItemClass::Helmet => "Hełmy",
            ItemClass::Boots => "Buty",
            ItemClass::Gloves => "Rękawice",
            ItemClass::Ring => "Pierścienie",
            ItemClass::Necklace => "Naszyjniki",
            ItemClass::Shield => "Tarcze",
            ItemClass::Neutral => "Neutralne",
            ItemClass::Consume => "Konsumpcyjne",
            ItemClass::Gold => "Złoto",
            ItemClass::Keys => "Klucze",
            ItemClass::Quest => "Questowe",
            ItemClass::Renewable => "Odnawialne", // ???
            ItemClass::Arrows => "Strzały",
            ItemClass::Talisman => "Talizmany",
            ItemClass::Book => "Książki",
            ItemClass::Bag => "Torby",
            ItemClass::Bless => "Błogosławieństwa",
            ItemClass::Upgrade => "Ulepszenia",
            ItemClass::Recipe => "Recepty",
            ItemClass::Coinage => "Waluta",
            ItemClass::Quiver => "Strzały", // In game it's also denoted like that ???
            ItemClass::Outfits => "Stroje",
            ItemClass::Pets => "Maskotki",
            ItemClass::Teleports => "Teleporty",
        }
    }
}

impl fmt::Display for ItemClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ItemClass::OneHandWeapon => write!(f, "one_hand_weapon"),
            ItemClass::TwoHandWeapon => write!(f, "two_hand_weapon"),
            ItemClass::OneAndHalfHandWeapon => write!(f, "one_and_half_hand_weapon"),
            ItemClass::DistanceWeapon => write!(f, "distance_weapon"),
            ItemClass::HelpWeapon => write!(f, "help_weapon"),
            ItemClass::WandWeapon => write!(f, "wand_weapon"),
            ItemClass::OrbWeapon => write!(f, "orb_weapon"),
            ItemClass::Armor => write!(f, "armor"),
            ItemClass::Helmet => write!(f, "helmet"),
            ItemClass::Boots => write!(f, "boots"),
            ItemClass::Gloves => write!(f, "gloves"),
            ItemClass::Ring => write!(f, "ring"),
            ItemClass::Necklace => write!(f, "necklace"),
            ItemClass::Shield => write!(f, "shield"),
            ItemClass::Neutral => write!(f, "neutral"),
            ItemClass::Consume => write!(f, "consume"),
            ItemClass::Gold => write!(f, "gold"),
            ItemClass::Keys => write!(f, "keys"),
            ItemClass::Quest => write!(f, "quest"),
            ItemClass::Renewable => write!(f, "renewable"),
            ItemClass::Arrows => write!(f, "arrows"),
            ItemClass::Talisman => write!(f, "talisman"),
            ItemClass::Book => write!(f, "book"),
            ItemClass::Bag => write!(f, "bag"),
            ItemClass::Bless => write!(f, "bless"),
            ItemClass::Upgrade => write!(f, "upgrade"),
            ItemClass::Recipe => write!(f, "recipe"),
            ItemClass::Coinage => write!(f, "coinage"),
            ItemClass::Quiver => write!(f, "quiver"),
            ItemClass::Outfits => write!(f, "outfits"),
            ItemClass::Pets => write!(f, "pets"),
            ItemClass::Teleports => write!(f, "teleports"),
        }
    }
}
