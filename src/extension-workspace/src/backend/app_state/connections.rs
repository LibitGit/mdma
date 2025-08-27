use std::ops::Not;

use axum::extract::ws::Message as WsMessage;
use common::{connection::SessionScope, messaging::prelude::*};
use dashmap::DashMap;
use futures::{SinkExt, channel::mpsc};
use serde_json::Value;
use tokio::task::JoinHandle;
use uuid::fmt::Simple;

use crate::prelude::*;

// TODO: Keep these maps "in sync" - meaning whenever a user gets logs in also
// update the uid_to_cids map accordingly.
#[derive(Debug, Default)]
pub struct Connections {
    // TODO: What is the usecase for this? Needed in the future when u want to send data to all of
    // user's connections.
    /// Mapping a user id (uid) to all their authorized connection ids (cid).
    authorized: DashMap<serenity::UserId, Vec<Simple>>,
    /// All active connections.
    all: DashMap<Simple, Connection>,
    /// Mapping a game account id to the corresponding connection id (cid).
    // TODO: Move this explanation somewhere else? Maybe into the module docs.
    /// If for a game account id there is a corresponding cid the newest
    /// settings are guaranteed to be in the database, otherwise they are to be
    /// taken from the connection corresponding to the cid. Then the session
    /// for that cid should be closed, since no two players can play on one
    /// account.
    field: DashMap<GameAccountId, Simple>,
}

impl Connections {
    /// Authorize a single [`Connection`].
    ///
    /// # Errors
    ///
    /// If this method returns an [`Err`] the connection with `cid` stays
    /// unauthorized.
    pub async fn authorize_one(
        &self,
        cid: Simple,
        discord_acc: DiscordAccount,
        member: serenity::Member,
        premium_details: Option<super::client::Premium>,
    ) -> std::result::Result<(), AuthError> {
        let uid = discord_acc.id;
        let Some(mut connection) = self.all.get_mut(&cid) else {
            return Err(AuthError::MissingConnection);
        };
        
        connection.authorize(discord_acc, member, premium_details)?;
        drop(connection);

        self.authorized.entry(uid).or_default().push(cid);
        info!("Authorized a connection with id '{cid}' for '{uid}'.");

        Ok(())
    }

    pub fn has_unauthorized(&self, cid: &Simple) -> bool {
        self.all
            .get(cid)
            .is_some_and(|connection| !connection.is_authorized())
    }
}

impl Connections {
    /// Remove an unauthorized connection.
    ///
    /// Since the connection is unauthorized no grace period is provided for
    /// moving it's `session` data and it get's removed instantly.
    pub(super) fn remove_unauthorized(&self, cid: &Simple) -> Option<(Simple, Connection)> {
        // Since the connection is unauthorized there is no grace period for it's
        // removal.
        let opt = self.all.remove(cid);

        debug_assert!(
            opt.as_ref()
                .is_some_and(|(_, connection)| !connection.is_authorized()),
            "Could not remove unauthorized connection: {opt:?}",
        );

        opt
    }

    /// Remove an authorized connection.
    ///
    /// This method does not provide a grace period after which session data
    /// should be saved. It only removes the connection details from the
    /// connections list.
    pub(super) fn remove_authorized(
        &self,
        cid: &Simple,
        uid: &serenity::UserId,
    ) -> Result<Connection> {
        let mut err = Some(anyhow!("Missing entry in 'authorized' for uid '{uid}'!"));

        self.authorized.remove_if_mut(uid, |_, cids| {
            let removed = cids
                .iter()
                .position(|id| id == cid)
                .map(|index| cids.swap_remove(index))
                .is_some();

            err = (!removed).then(|| anyhow!("Missing entry in 'authorized' for cid '{cid}'!"));

            cids.is_empty()
        });

        if let Some(err) = err {
            return Err(err);
        }

        let (_, connection) = self
            .all
            .remove_if(cid, |_, connection| !connection.has_active_session())
            .ok_or_else(|| anyhow!("Connection with id '{cid}' for '{uid}' has active session!"))?;

        info!("Terminated authorized connection with id '{cid}' for '{uid}'.");

        Ok(connection)
    }

    /// Insert an unauthorized connection **only** into the active connections
    /// map.
    ///
    /// *This method does not update the authorized connections map. Make sure
    /// to update it accordingly when the user logs in*.
    pub(super) fn new_unauthorized(&self, cid: Simple, tx: mpsc::UnboundedSender<WsMessage>) {
        let connection = Connection::new_unauthorized(tx, cid);

        if let Some(old) = self.all.insert(cid, connection) {
            warn!("Duplicate connection with id `{cid}`! {old:?}")
        }
    }

    /// Insert an authorized connection.
    pub(super) fn new_authorized(
        &self,
        cid: Simple,
        tx: mpsc::UnboundedSender<WsMessage>,
        user: impl Into<User>,
    ) {
        let user = user.into();
        let uid = user.id;
        let connection = Connection::new(tx, user, cid);

        self.authorized.entry(uid).or_default().push(cid);

        if let Some(old) = self.all.insert(cid, connection) {
            warn!("Duplicate connection with id `{cid}`! {old:?}")
        }
    }

    pub(super) fn has_active_session(&self, cid: &Simple) -> bool {
        self.all
            .get(cid)
            .is_some_and(|connection| connection.has_active_session())
    }

    // TODO: Incomplete functionality.
    /// Withdraw access for an authorized session, by removing the current user
    /// data.
    pub(super) fn revoke(
        &self,
        uid: &serenity::UserId,
        cid: &Simple,
        all_devices: bool,
    ) -> Result<User> {
        let mut err = Some(anyhow!("Missing entry in 'authorized' for uid '{uid}'!"));

        self.authorized.remove_if_mut(uid, |_, cids| {
            let removed = cids
                .iter()
                .position(|id| id == cid)
                .map(|index| cids.swap_remove(index))
                .is_some();

            err = (!removed).then(|| anyhow!("Missing entry in 'authorized' for cid '{cid}'!"));

            cids.is_empty()
        });

        if let Some(err) = err {
            return Err(err);
        }

        self.all
            .get_mut(cid)
            .ok_or_else(|| anyhow!("Missing `Connection` with id `{cid}`!"))?
            .user
            .take()
            .ok_or_else(|| anyhow!("Missing `User` with id `{uid}`!"))
    }
}

/// Connection data of a single user connected via web socket.
/// There can be multiple sessions played in the lifetime of one connection.
///
/// Connections are kept alive for 1min after a disconnect, since
/// someone might e.g. be quickly relogging from another profile. This way we
/// reduce the db usage.
#[derive(Debug)]
pub struct Connection {
    /// User details available after connection authorization.
    user: Option<User>,
    /// Sending side of a WebSocket used for transmitting data back to the user.
    pub tx: mpsc::UnboundedSender<WsMessage>,
    /// Id of the connection.
    pub id: Simple,
}

impl Connection {
    pub(super) const fn new(tx: mpsc::UnboundedSender<WsMessage>, user: User, cid: Simple) -> Self {
        Self {
            tx,
            user: Some(user),
            id: cid,
        }
    }

    pub(super) const fn new_unauthorized(
        tx: mpsc::UnboundedSender<WsMessage>,
        cid: Simple,
    ) -> Self {
        Self {
            tx,
            user: None,
            id: cid,
        }
    }

    const fn is_authorized(&self) -> bool {
        self.user.is_some()
    }

    fn authorize(
        &mut self,
        discord_acc: DiscordAccount,
        member: serenity::Member,
        premium_details: Option<super::client::Premium>,
    ) -> std::result::Result<(), AuthError> {
        let access_level = AccessLevel::new(discord_acc.email_verified, &member.roles);
        let access_token = Jwt::<AccessClaims>::new(access_level, self.id)?;
        let refresh_token = Jwt::<RefreshClaims>::new(discord_acc.id, discord_acc.version)?;
        let maybe_premium = premium_details.map(|premium| {
            Premium::new(
                (premium.exp.timestamp_millis() / 1000) as u64,
                premium.neon,
                premium.animation,
                access_level.antyduch(),
            )
        });
        // TODO: Add premium data to response.
        let response = Message::builder(Task::Tokens, Target::Background, MessageKind::Response)
            .access_token(access_token.into())
            .refresh_token(refresh_token.into())
            .username(member.user.name)
            .session_scope(discord_acc.session_scope)
            .maybe_premium(maybe_premium)
            .build()
            .into_ws_message()
            .map_err(|err| {
                warn!("{err:#?}");
                AuthError::MessagingError
            })?;

        self.tx.unbounded_send(response).map_err(|err| {
            warn!("{err:#?}");
            AuthError::MessagingError
        })?;

        self.user = Some(discord_acc.into());

        Ok(())
    }

    fn has_active_session(&self) -> bool {
        self.user.as_ref().is_some_and(User::has_active_session)
    }
}

/// User details available after connection authorization.
#[derive(Debug)]
pub struct User {
    /// Determines session persistance.
    pub scope: SessionScope,
    /// Current game session data.
    pub session: Option<Session>,
    /// Discord identifier (discord user id).
    pub id: serenity::UserId,
}

impl User {
    pub(super) const fn new(scope: SessionScope, uid: serenity::UserId) -> Self {
        Self {
            scope,
            id: uid,
            session: None,
        }
    }

    // pub(super) const fn new_with_session(

    pub async fn terminate_session(self) -> Result<()> {
        let Some(_session) = self.session else {
            return Ok(());
        };

        // let Value::Object(settings) = connection.settings else {
        //     // TODO: Verify this is correct.
        //     return Ok(()); // No settings to save
        // };

        // for (account_id, value) in settings {
        //     let Value::Object(char_or_acc_settings_map) = value else {
        //         // TODO: Verify this is correct.
        //         return Ok(()); // No settings to save
        //     };
        //     for (key, value) in char_or_acc_settings_map {
        //         if key.chars().any(|c| !c.is_ascii_digit()) {

        //         }
        //     }
        // }

        Ok(())
    }

    fn has_active_session(&self) -> bool {
        self.session.is_some()
    }
}

impl From<DiscordAccount> for User {
    fn from(value: DiscordAccount) -> Self {
        Self {
            scope: value.session_scope,
            id: value.id,
            session: None,
        }
    }
}

impl From<&DiscordAccount> for User {
    fn from(value: &DiscordAccount) -> Self {
        Self {
            scope: value.session_scope,
            id: value.id,
            session: None,
        }
    }
}

/// Current manager state for the player.
///
/// # Addon Settings
///
/// [`Session`]'s `addon_settings` field stores data valid only for the given
/// scope.
///
/// Settings are to be stored in a [`Map::<AddonName,
/// AddonData>`][serde_json::Map] format.
///
/// Depending on the scope settings are applied:
/// 1. `SessionScope::GameCharacter` - only for the game character,
/// 2. `SessionScope::GameAccount` - to the whole game account,
/// 3. `SessionScope::DiscordAccount` - to all the game accounts used by the
///    discord user.
///
/// Game characters/accounts can be used by multiple discord users, so
/// each instance of them has a discord user id associated with it in the db.
/// This allows each discord user to have different settings for a specific game
/// character or account.
///
/// # Persistance
///
/// Session persistance for a given scope is as follows:
/// 1. `SessionScope::GameCharacter` the session terminates on every relog.
/// 2. `SessionScope::GameAccount` the session terminates on every log out.
/// 3. `SessionScope::DiscordAccount` the session terminates with the
///    connection.
#[derive(Debug)]
pub struct Session {
    /// Id of the account the user is currently logged in to.
    pub account_id: GameAccountId,
    /// Id of the character the user is currently playing as.
    pub char_id: GameCharId,
    /// Addon settings of the session.
    pub addon_settings: Value,
}
