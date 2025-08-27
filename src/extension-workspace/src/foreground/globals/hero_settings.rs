use std::sync::OnceLock;

use futures::StreamExt;
use futures_signals::signal::{Mutable, MutableLockMut, Signal, SignalExt};
use wasm_bindgen::intern;

use crate::{
    bindings::{
        engine::communication::{CharacterSettings, HeroSettingsData},
        message,
    },
    s,
    utils::JsResult,
};

static HERO_SETTINGS: OnceLock<HeroSettings> = OnceLock::new();

#[derive(Debug)]
pub struct HeroSettings(Mutable<HeroSettingsData>);

impl HeroSettings {
    pub(super) fn init() -> JsResult<()> {
        HERO_SETTINGS
            .set(Self(Mutable::default()))
            .map_err(|_| ::common::err_code!())
    }

    pub fn get() -> &'static Self {
        HERO_SETTINGS.wait()
    }

    pub(crate) async fn init_server_option_config() {
        let future = HeroSettings::get()
            .signal_ref(|settings_list| {
                let mut ids_to_update = (None, None);

                if settings_list
                    .friend_login_notif
                    .as_ref()
                    .is_none_or(|data| data.value.as_ref().is_none_or(|active| !active))
                {
                    ids_to_update.0 = Some(CharacterSettings::FRIEND_NOTIF_ID)
                }
                if settings_list
                    .clan_login_notif
                    .as_ref()
                    .is_none_or(|data| data.value.as_ref().is_none_or(|active| !active))
                {
                    ids_to_update.1 = Some(CharacterSettings::CLAN_NOTIF_ID)
                }

                ids_to_update
            })
            .to_stream()
            .skip(1)
            .for_each(|ids_to_update| async move {
                let (friend_notif_id, clan_notif_id) = ids_to_update;
                if let Some(friend_notif_id) = friend_notif_id {
                    let _ = message(intern(s!(
                        "[MDMA::RS] Włączam informowanie o logowaniu się przyjaciół."
                    )));
                    let _ = message(intern(s!(
                        "[MDMA::RS] Ta opcja jest potrzebna do poprawnego działania zestawu!"
                    )));
                    CharacterSettings::init_setting(friend_notif_id).await
                }
                if let Some(clan_notif_id) = clan_notif_id {
                    let _ = message(intern(s!(
                        "[MDMA::RS] Włączam informowanie o logowaniu się klanowiczów."
                    )));
                    let _ = message(intern(s!(
                        "[MDMA::RS] Ta opcja jest potrzebna do poprawnego działania zestawu!"
                    )));
                    CharacterSettings::init_setting(clan_notif_id).await
                }
            });
        wasm_bindgen_futures::spawn_local(future);

        //let settings = wait_for_val(|| get_engine().settings(), 10, 60_000)
        //    .await
        //    .unwrap_js();

        //settings.observe_toggle_server_option().unwrap_js();
    }

    #[inline]
    pub(crate) fn signal_ref<B, F>(&self, f: F) -> impl Signal<Item = B>
    where
        F: FnMut(&HeroSettingsData) -> B,
    {
        self.0.signal_ref(f)
    }

    #[inline]
    fn lock_mut(&self) -> MutableLockMut<'_, HeroSettingsData> {
        self.0.lock_mut()
    }

    pub(crate) fn merge(new_character_settings: HeroSettingsData) {
        let mut character_settings_lock = Self::get().lock_mut();

        let HeroSettingsData {
            friend_login_notif,
            clan_login_notif,
        } = new_character_settings;

        if friend_login_notif
            .as_ref()
            .is_some_and(|data| data.value.is_some())
        {
            character_settings_lock.friend_login_notif = friend_login_notif;
        }
        if clan_login_notif
            .as_ref()
            .is_some_and(|data| data.value.is_some())
        {
            character_settings_lock.clan_login_notif = clan_login_notif;
        }
    }
}
