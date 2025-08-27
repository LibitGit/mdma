use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
    signal_vec::{MutableVec, MutableVecLockRef, SignalVec, SignalVecExt, VecDiff},
};
use wasm_bindgen::JsValue;

use crate::{
    color_mark::{Color, ColorMark},
    prelude::*,
};

/// In-game player instance.
#[derive(Debug, Clone)]
pub struct Other {
    /// Account id of the player.
    pub account: u32,
    /// Character id of the player.
    pub char_id: OtherId,
    /// List of emotions currently displayed by the player.
    pub emo: Emotions,
    /// Player's current clan if any.
    pub clan: Mutable<Option<Clan>>,
    /// Current level of the player.
    pub lvl: Mutable<u16>,
    /// Current operational level of the player.
    /// If the player's level is <= 300 the operational level will have the exact same value.
    pub operational_lvl: Mutable<u16>,
    /// Player's in-game nick.
    pub nick: Mutable<String>,
    /// Player's in-game profession.
    pub prof: Mutable<Profession>,
    /// Hero's relation relative to the player.
    pub relation: Mutable<Relation>,
    /// Specifies whether the player is currently AFK.
    pub stasis: Mutable<bool>,
    /// Specifies in how many seconds the player will go fully AFK.
    /// This value is not equal to zero only if the player is currently falling asleep.
    pub stasis_incoming_seconds: Mutable<u8>,
    /// Player's current x-axis coordinate relative to the left border of the map.
    pub x: Mutable<u8>,
    /// Player's current y-axis coordinate relative to the top border of the map.
    pub y: Mutable<u8>,
    /// Specifies if the player is currently on the wanted list.
    pub wanted: Mutable<bool>,
    // pub vip: Option<String>,
    // pub rights: Option<u8>,
    // pub is_blessed: Option<u8>,
    // pub dir: Option<u8>,
    // pub icon: Option<String>,
    // pub attr: Option<u8>,
}

impl Other {
    pub(crate) fn new(char_id: OtherId, other_data: OtherData) -> Result<Self, JsValue> {
        if other_data.action.ok_or_else(|| err_code!())? != OtherAction::Create {
            return Err(err_code!());
        }

        let initial_emotions = other_data.parse_emotions(char_id, false);
        let OtherData {
            account,
            clan,
            lvl,
            operational_lvl,
            nick,
            prof,
            relation,
            stasis,
            stasis_incoming_seconds,
            x,
            y,
            wanted,
            ..
        } = other_data;

        Ok(Self {
            account: account.ok_or_else(|| err_code!())?,
            char_id,
            emo: Emotions::new(initial_emotions),
            clan: Mutable::new(clan),
            lvl: Mutable::new(lvl.ok_or_else(|| err_code!())?),
            operational_lvl: Mutable::new(operational_lvl.ok_or_else(|| err_code!())?),
            nick: Mutable::new(nick.ok_or_else(|| err_code!())?),
            prof: Mutable::new(prof.ok_or_else(|| err_code!())?),
            relation: Mutable::new(relation.ok_or_else(|| err_code!())?),
            stasis: Mutable::new(stasis.ok_or_else(|| err_code!())?),
            stasis_incoming_seconds: Mutable::new(
                stasis_incoming_seconds.ok_or_else(|| err_code!())?,
            ),
            x: Mutable::new(x.ok_or_else(|| err_code!())?),
            y: Mutable::new(y.ok_or_else(|| err_code!())?),
            wanted: Mutable::new(wanted.unwrap_or_default()),
        })
    }

    pub(crate) fn update(&self, char_id: OtherId, new_other_data: OtherData) {
        {
            let mut emotions_lock = self.emo.0.lock_mut();
            let emotions = new_other_data.parse_emotions(char_id, self.stasis.get());
            emotions
                .stasis
                .into_iter()
                .chain(emotions.stasis_incoming)
                .for_each(|emotion| match emotion.name {
                    EmotionName::AwayEnd => {
                        if let Some(index) = emotions_lock
                            .iter()
                            .position(|emotion| emotion.name == EmotionName::Away)
                        {
                            emotions_lock.remove(index);
                        }
                    }
                    EmotionName::StasisEnd => {
                        if let Some(index) = emotions_lock
                            .iter()
                            .position(|other_emotion| other_emotion.name == EmotionName::Stasis)
                        {
                            emotions_lock.remove(index);
                        }
                    }
                    _ => emotions_lock.push(emotion),
                })
        }

        if let Some(new_clan) = new_other_data.clan {
            self.clan.set_neq(Some(new_clan));
        }
        if let Some(new_lvl) = new_other_data.lvl {
            self.lvl.set_neq(new_lvl);
        }
        if let Some(new_operational_lvl) = new_other_data.operational_lvl {
            self.operational_lvl.set_neq(new_operational_lvl);
        }
        if let Some(new_nick) = new_other_data.nick {
            self.nick.set_neq(new_nick);
        }
        if let Some(new_prof) = new_other_data.prof {
            self.prof.set_neq(new_prof);
        }
        if let Some(new_relation) = new_other_data.relation {
            self.relation.set_neq(new_relation);
        }
        if let Some(new_stasis) = new_other_data.stasis {
            self.stasis.set_neq(new_stasis);
        }
        if let Some(new_stasis_incoming_seconds) = new_other_data.stasis_incoming_seconds {
            self.stasis_incoming_seconds
                .set_neq(new_stasis_incoming_seconds);
        }
        if let Some(new_x) = new_other_data.x {
            self.x.set_neq(new_x);
        }
        if let Some(new_y) = new_other_data.y {
            self.y.set_neq(new_y);
        }
        if let Some(new_wanted) = new_other_data.wanted {
            self.wanted.set_neq(new_wanted);
        }
    }

    pub(crate) fn init_emotion(&self, emotion: Emotion) {
        let mut emotions_lock = self.emo.0.lock_mut();
        emotions_lock.retain(|old| old.name != emotion.name);

        match emotion.name {
            EmotionName::AwayEnd => {
                if let Some(index) = emotions_lock
                    .iter()
                    .position(|emotion| emotion.name == EmotionName::Away)
                {
                    emotions_lock.remove(index);
                }
            }
            EmotionName::StasisEnd => {
                if let Some(index) = emotions_lock
                    .iter()
                    .position(|other_emotion| other_emotion.name == EmotionName::Stasis)
                {
                    emotions_lock.remove(index);
                }
            }
            EmotionName::Noemo => {
                emotions_lock.retain(|emotion| {
                    !matches!(
                        emotion.name,
                        EmotionName::Noemo
                            | EmotionName::Battle
                            | EmotionName::Logoff
                            | EmotionName::Frnd
                            | EmotionName::PvpProtected
                    )
                });
            }
            EmotionName::Undefined => {}
            _ => emotions_lock.push(emotion),
        };
    }

    /// Determines whether a player is currently fighting something based on the emotion displayed
    /// by him.
    pub fn in_battle(&self) -> bool {
        self.emo
            .0
            .lock_ref()
            .iter()
            .any(|emotion| emotion.name == EmotionName::Battle)
    }

    /// Determines whether a player is a friend | clan member | clan ally | fraction ally.
    pub fn friendly(&self) -> bool {
        matches!(
            self.relation.get(),
            Relation::Friend | Relation::Clan | Relation::ClanAlly | Relation::FractionAlly
        )
    }

    /// Signal identifying whether a player is a friend | clan member | clan ally | fraction ally.
    pub fn friendly_signal(&self) -> impl Signal<Item = bool> + use<> {
        self.relation.signal().map(|relation| {
            matches!(
                relation,
                Relation::Friend | Relation::Clan | Relation::ClanAlly | Relation::FractionAlly
            )
        })
    }

    pub fn coords_signal(&self) -> impl Signal<Item = (u8, u8)> + use<> {
        map_ref! {
            let x = self.x.signal(),
            let y = self.y.signal() => {
                (*x, *y)
            }
        }
    }

    pub fn is_wanted(&self) -> bool {
        self.wanted.get()
    }

    pub(crate) fn init_color_mark(&self, color: Color, addon_name: AddonName) -> JsResult<()> {
        ColorMark::init_with_player_data(color, addon_name, self)
    }
}

impl PartialEq for Other {
    fn eq(&self, other: &Self) -> bool {
        self.char_id == other.char_id
    }
}

#[derive(Debug, Clone)]
pub struct Emotions(MutableVec<Emotion>);

impl Emotions {
    fn new(initial: OtherDataEmotions) -> Self {
        let this = Self(MutableVec::with_capacity(4));
        let future = this.0.signal_vec().for_each({
            let this = this.clone();
            move |vec_diff| {
                let this = this.clone();
                async move {
                    // debug_log!(@f "{vec_diff:?}");
                    if let VecDiff::Push { value } | VecDiff::InsertAt { value, .. } = vec_diff {
                        if value.name.is_removable() {
                            this.enqueue_remove(value.name).await
                        }
                    } else if let VecDiff::Replace { values } = vec_diff {
                        let futures = values
                            .into_iter()
                            .map(|emotion| this.enqueue_remove(emotion.name));
                        futures::future::join_all(futures).await;
                    }
                }
            }
        });

        wasm_bindgen_futures::spawn_local(future);

        {
            let mut emotions_lock = this.0.lock_mut();
            initial
                .stasis
                .into_iter()
                .chain(initial.stasis_incoming)
                .for_each(|emotion| match emotion.name {
                    EmotionName::AwayEnd => {
                        if let Some(index) = emotions_lock
                            .iter()
                            .position(|emotion| emotion.name == EmotionName::Away)
                        {
                            emotions_lock.remove(index);
                        }
                    }
                    EmotionName::StasisEnd => {
                        if let Some(index) = emotions_lock
                            .iter()
                            .position(|other_emotion| other_emotion.name == EmotionName::Stasis)
                        {
                            emotions_lock.remove(index);
                        }
                    }
                    _ => emotions_lock.push(emotion),
                })
        }

        this
    }

    pub fn signal_vec(&self) -> impl SignalVec<Item = Emotion> + use<> {
        self.0.signal_vec()
    }

    pub fn clear(&self) {
        self.0.lock_mut().clear();
    }

    //pub fn apply_vec_diff(&self, vec_diff: VecDiff<Emotion>) {
    //    MutableVecLockMut::apply_vec_diff(&mut self.0.lock_mut(), vec_diff);
    //}

    pub fn lock_ref(&self) -> MutableVecLockRef<'_, Emotion> {
        self.0.lock_ref()
    }

    // TODO: Cancel queued removals when removing duplicates.
    async fn enqueue_remove(&self, emotion_name: EmotionName) {
        if emotion_name == EmotionName::Undefined {
            self.0
                .lock_mut()
                .retain(|emotion| !matches!(emotion.name, EmotionName::Undefined));
            return;
        }
        if let Some(emotion_duration) = emotion_name.get_duration() {
            delay(emotion_duration).await;
        }

        let mut emotions_lock = self.0.lock_mut();
        match emotion_name {
            EmotionName::AwayEnd => {
                if let Some(index) = emotions_lock
                    .iter()
                    .position(|emotion| emotion.name == EmotionName::Away)
                {
                    emotions_lock.remove(index);
                }
                if let Some(index) = emotions_lock
                    .iter()
                    .position(|emotion| emotion.name == EmotionName::AwayEnd)
                {
                    emotions_lock.remove(index);
                }
            }
            EmotionName::StasisEnd => {
                if let Some(index) = emotions_lock
                    .iter()
                    .position(|other_emotion| other_emotion.name == EmotionName::Stasis)
                {
                    emotions_lock.remove(index);
                }
                if let Some(index) = emotions_lock
                    .iter()
                    .position(|other_emotion| other_emotion.name == EmotionName::StasisEnd)
                {
                    emotions_lock.remove(index);
                }
            }
            EmotionName::Noemo => {
                emotions_lock.retain(|emotion| {
                    !matches!(
                        emotion.name,
                        EmotionName::Noemo
                            | EmotionName::Battle
                            | EmotionName::Logoff
                            | EmotionName::Frnd
                            | EmotionName::PvpProtected
                    )
                });
            }
            _ => {
                if let Some(index) = emotions_lock
                    .iter()
                    .position(|emotion| emotion.name == emotion_name)
                {
                    emotions_lock.remove(index);
                }
            }
        }
    }
}
