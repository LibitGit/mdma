use std::sync::OnceLock;

use common::{err_code, map_err, web_extension_sys::cookies::CookieDetails};
use futures_signals::{
    map_ref,
    signal::{Mutable, Signal},
};
use gloo_net::http::{Method, RequestBuilder};
use serde::Deserialize;
use web_sys::RequestCredentials;

use crate::{
    bindings::{
        engine::{
            communication::{HeroClan, HeroData, Id},
            other::Other,
        },
        get_engine, is_pl,
    },
    s,
    utils::{JsResult, window},
};

use super::{GlobalsError, port::Port};

static HERO: OnceLock<Hero> = OnceLock::new();

#[derive(Debug, Default)]
pub struct Hero {
    pub account_string: String,
    pub account: Id,
    pub char_id_string: String,
    pub char_id: Id,
    // pub attr: Option<u8>,
    // pub bag: Option<u32>,
    // pub bagi: Option<u32>,
    // pub bint: Option<u32>,
    // pub blockade: Option<u32>,
    // pub bstr: Option<u32>,
    pub clan: Mutable<Option<HeroClan>>,
    // pub credits: Option<u32>,
    // pub cur_battle_set: Option<u32>,
    // pub cur_skill_set: Option<u32>,
    // pub dir: Option<u8>,
    // pub exp: Option<u64>,
    // pub gender: Option<String>,
    // pub gold: Option<u64>,
    // pub goldlim: Option<u64>,
    // pub healpower: Option<u32>,
    // pub honor: Option<u32>,
    // pub img: Option<String>,
    // pub is_blessed: Option<u8>,
    pub lvl: Mutable<u16>,
    // pub mails: Option<u32>,
    // pub mails_all: Option<u32>,
    // pub mails_last: Option<String>,
    // pub mpath: Option<String>,
    pub nick: Mutable<String>,
    // pub opt: Option<u32>,
    // pub party: Option<u32>,
    // pub passive_stats: Option<String>,
    // pub prof: Option<String>,
    // pub pvp: Option<u32>,
    // pub runes: Option<u32>,
    // pub stamina: Option<u32>,
    // pub stamina_renew_sec: Option<u32>,
    // pub stamina_ts: Option<u32>,
    pub stasis: Mutable<bool>,
    pub stasis_incoming_seconds: Mutable<Option<u8>>,
    // pub trade: Option<u32>,
    // pub ttl: Option<i32>,
    // pub ttl_del: Option<u32>,
    // pub ttl_end: Option<u32>,
    // pub ttl_value: Option<u32>,
    // pub uprawnienia: Option<u32>,
    pub vip: Mutable<bool>,
    // pub wanted: Option<u32>,
    // pub warrior_stats: Option<WarriorStats>,
    pub x: Mutable<Option<u8>>,
    pub y: Mutable<Option<u8>>,
}

impl Hero {
    pub fn get() -> &'static Self {
        HERO.wait()
    }

    /// SAFETY: Can be called only if Port is initialized.
    // TODO: Move the account_string and char_id into separate encrypted variables
    // and init hero later on ?
    pub(super) async fn init() -> Result<(), GlobalsError> {
        let mut hero = Self::default();
        let Ok(account_string) = Self::fetch_account_id().await else {
            crate::bindings::message(s!(
                "[MDMA::RS] Nie udało się wczytać zestawu ze względu na API Margonem! Spróbuj ponownie za chwilę..."
            ))?;
            return Err(GlobalsError::Unauthorized);
        };
        hero.account = account_string.parse().map_err(map_err!(from))?;
        hero.account_string = account_string;

        let id_string = Port::fetch_cookie(CookieDetails {
            name: string!("mchar_id"),
            url: window().location().href().map_err(map_err!())?,
        })
        .await?
        .value;

        hero.char_id = id_string.parse().map_err(map_err!(from))?;
        hero.char_id_string = id_string;

        HERO.set(hero).map_err(|_| err_code!())?;

        Ok(())
    }

    async fn fetch_account_id() -> JsResult<String> {
        match is_pl().map_err(map_err!())? {
            true => {
                let addon_list: AddonListResponse = RequestBuilder::new(s!(
                    "https://public-api.margonem.pl/addons/list?tab=mine&page=1"
                ))
                .credentials(RequestCredentials::Include)
                .method(Method::GET)
                .build()
                .map_err(map_err!(from))?
                .send()
                .await
                .map_err(map_err!(from))?
                .json()
                .await
                .map_err(map_err!(from))?;

                Ok(addon_list.list.account_id)
            }
            false => Ok(Port::fetch_cookie(CookieDetails {
                name: string!("user_id"),
                url: window().location().href().map_err(map_err!())?,
            })
            .await?
            .value),
        }
    }

    //TODO: Check account and character ids with:
    //https://public-api.margonem.pl/account/charlist?hs3={xxx}
    //https://public-api.margonem.pl/addons/list?tab=mine&page=1
    //cookies: mchar_id, user_id (also validate if document.cookies has the same
    // value and the prototype and shi is correct) Engine.hero.d
    //https://addons2.margonem.pl/get/'+Math.floor(id/1000)+'/'+id+build+'.js
    //optionally: https://www.margonem.pl/profile/view,0
    //TODO: Different solution for account_string and id_string.
    pub(crate) fn merge(new_hero_data: HeroData) {
        let Hero {
            x,
            y,
            nick,
            clan,
            lvl,
            vip,
            stasis,
            stasis_incoming_seconds,
            ..
        } = Self::get();

        if let Some(new_lvl) = new_hero_data.lvl {
            lvl.set_neq(new_lvl);
        }
        if let Some(new_x) = new_hero_data.x {
            x.set_neq(Some(new_x));
        }
        if let Some(new_y) = new_hero_data.y {
            y.set_neq(Some(new_y));
        }
        if let Some(new_nick) = new_hero_data.nick {
            nick.set_neq(new_nick);
        }
        if let Some(new_clan) = new_hero_data.clan {
            clan.set_neq(Some(new_clan));
        }
        if let Some(new_vip) = new_hero_data.vip.map(|vip| vip != 0) {
            vip.set_neq(new_vip);
        }
        if let Some(new_stasis) = new_hero_data.stasis {
            stasis.set_neq(new_stasis);
        }
        if let Some(new_stasis_incoming_seconds) = new_hero_data.stasis_incoming_seconds {
            let new_stasis_incoming_seconds =
                (new_stasis_incoming_seconds != 0).then_some(new_stasis_incoming_seconds);
            stasis_incoming_seconds.set_neq(new_stasis_incoming_seconds);
        }
    }

    pub(crate) fn reload() {
        let this = Self::get();
        this.x.set_neq(None);
        this.y.set_neq(None);
    }

    pub(crate) fn is_in_clan() -> bool {
        Self::get().clan.lock_ref().is_some()
    }

    pub fn coords_signal(&self) -> impl Signal<Item = Option<(u8, u8)>> + use<> {
        map_ref! {
            let x = self.x.signal(),
            let y = self.y.signal() => {
                x.zip(*y)
            }
        }
    }

    /// Calculates whether a player is within one tile range from hero.
    pub(crate) fn is_non_peer_in_invite_range(other_data: &Other) -> bool {
        let hero = Self::get();

        if let Some(hero_x) = hero.x.get()
            && let Some(hero_y) = hero.y.get()
        {
            let other_x = other_data.x.get() as i32;
            let other_y = other_data.y.get() as i32;

            return (hero_x as i32 - other_x).abs() <= 1 && (hero_y as i32 - other_y).abs() <= 1;
        }

        false
    }

    // TODO: Return Result.
    #[cfg(feature = "ni")]
    pub(crate) fn in_battle() -> bool {
        use crate::utils::UnwrapJsExt;

        let battle = get_engine().battle().unwrap_js();

        battle.show().unwrap_js() && !battle.end_battle_for_me().unwrap_js()
    }

    #[cfg(not(feature = "ni"))]
    pub(crate) fn in_battle() -> bool {
        get_engine().battle().is_some()
    }

    //pub(crate) fn get_closest_player<'a>(
    //    &self,
    //    first: &'a Other,
    //    second: &'a Other,
    //) -> JsResult<&'a Other> {
    //    let distance_to_first = self.try_get_distance(first)?;
    //    let distance_to_second = self.try_get_distance(second)?;
    //
    //    let closest = match distance_to_first <= distance_to_second {
    //        true => first,
    //        false => second,
    //    };
    //    Ok(closest)
    //}
    //
    //pub fn try_get_distance(&self, other: &Other) -> JsResult<f64> {
    //    let hero_lock = self.lock_ref();
    //
    //    let distance = (other.x.get() as f64 - hero_lock.x.ok_or_else(||
    // err_code!())? as f64)        .hypot(other.y.get() as f64 -
    // hero_lock.y.ok_or_else(|| err_code!())? as f64);
    //
    //    Ok(distance)
    //}
    //
    //pub(crate) fn in_attack_range(&self, other: &Other) -> bool {
    //    self.try_get_distance(other).unwrap_or(4f64) <= 2.5f64
    //}
}

#[derive(Deserialize)]
struct AddonListResponse {
    list: AddonListData,
}

#[derive(Deserialize)]
struct AddonListData {
    // TODO: Verify this is the account id not character id.
    #[serde(rename = "char")]
    account_id: String,
}
