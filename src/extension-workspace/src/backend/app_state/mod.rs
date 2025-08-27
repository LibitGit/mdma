use std::{net::SocketAddr, sync::Arc};

use async_session::MemoryStore;
use axum::extract::ws::Message as WsMessage;
use common::messaging::prelude::*;
use futures::{SinkExt, channel::mpsc};
use oauth2::{
    AuthType, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl, basic::BasicClient,
};
use uuid::fmt::Simple;

use crate::prelude::*;

/// Module containing the API used for manipulating the MongoDb client
/// instance's contents.
pub mod client;
use client::Client;

/// In game web socket connection session.
pub mod connections;
use connections::Connections;

const GUILD_ID: serenity::GuildId = serenity::GuildId::new(to_u64(env!("GUILD_ID")));

/// State of the app shared between the discord bot, requests and web socket
/// connections.
#[derive(Debug, Clone)]
pub struct AppState {
    /// MongoDb Client instance.
    pub client: Client,
    /// All handshake verified (authorized or not) socket connections.
    pub connections: Arc<Connections>,
    /// Discord bot context allowing for actions outside a discord command.
    pub cache_http: serenity::Context,
    pub oauth_client: OAuth2Client,
    pub store: MemoryStore,
    pub http_client: oauth2::reqwest::Client,
}

impl AppState {
    pub(super) fn new(client: Client, cache_http: serenity::Context) -> Result<Self> {
        let oauth_client = BasicClient::new(ClientId::new(env!("CLIENT_ID").to_string()))
            .set_client_secret(ClientSecret::new(env!("CLIENT_SECRET").to_string()))
            .set_auth_uri(AuthUrl::new(
                "https://discord.com/api/oauth2/authorize".to_string(),
            )?)
            .set_token_uri(TokenUrl::new(
                "https://discord.com/api/oauth2/token".to_string(),
            )?)
            .set_redirect_uri(RedirectUrl::new(match cfg!(target_os = "windows") {
                true => "http://localhost:3000/callback".to_string(),
                false => "https://libit.ovh/callback".to_string(),
            })?)
            .set_auth_type(AuthType::RequestBody);
        let store = MemoryStore::new();
        let http_client = oauth2::reqwest::Client::new();

        Ok(Self {
            client,
            connections: Default::default(),
            cache_http,
            oauth_client,
            store,
            http_client,
        })
    }

    pub async fn get_member_data(
        &self,
        uid: impl Into<serenity::UserId>,
    ) -> Result<serenity::Member> {
        let member = serenity::PartialGuild::get(&self.cache_http, GUILD_ID)
            .await?
            .member(&self.cache_http, uid)
            .await?;

        Ok(member)
    }

    /// Establish an authorized connection for the provided user.
    ///
    /// # Errors
    ///
    /// If this method returns an [`Err`] the connection does not get added into
    /// the connections list.
    pub async fn establish_connection(
        &self,
        mut tx: mpsc::UnboundedSender<WsMessage>,
        who: SocketAddr,
        cid: Simple,
        discord_acc: &DiscordAccount,
    ) -> Result<()> {
        let uid = discord_acc.id;
        let member = self.get_member_data(uid).await?;
        let access_level = AccessLevel::new(discord_acc.email_verified, &member.roles);
        let access_token = Jwt::<AccessClaims>::new(access_level, cid)?;
        let refresh_token = Jwt::<RefreshClaims>::new(uid, discord_acc.version)?;
        let maybe_premium = self.client.get_premium_details(uid).await?.map(|premium| {
            Premium::new(
                (premium.exp.timestamp_millis() / 1000) as u64,
                premium.neon,
                premium.animation,
                access_level.antyduch(),
            )
        });
        let response = Message::builder(Task::Tokens, Target::Background, MessageKind::Response)
            .access_token(access_token.into())
            .refresh_token(refresh_token.into())
            .username(member.user.name)
            .session_scope(discord_acc.session_scope)
            .maybe_premium(maybe_premium)
            .build()
            .into_ws_message()?;

        tx.send(response).await?;

        self.client.update_discord_account_login(uid).await?;
        self.connections.new_authorized(cid, tx, discord_acc);

        info!("{who}: established authorized connection with id '{cid}' for '{uid}'.");

        Ok(())
    }

    /// # Errors
    ///
    /// If this method returns an [`Err`] the connection does not get added into
    /// the connections list.
    // TODO: Rename since it no longer sends data to the background.
    pub fn establish_unauthorized_connection(
        &self,
        tx: mpsc::UnboundedSender<WsMessage>,
        who: SocketAddr,
        cid: Simple,
    ) {
        self.connections.new_unauthorized(cid, tx);

        info!("{who}: established unauthorized connection with id '{cid}'.",);
    }

    pub async fn authorize_one_connection(
        &self,
        who: SocketAddr,
        cid: Simple,
        uid: serenity::UserId,
        email_verified: bool,
    ) -> std::result::Result<(), AuthError> {
        let member = self.get_member_data(uid).await.map_err(|err| {
            warn!("{who}: could not get member data! {err:#?}");
            AuthError::MissingCredentials
        })?;
        let discord_acc = self
            .client
            .upsert_discord_account_login(uid, email_verified)
            .await
            .map_err(|err| {
                warn!("{who}: could not upsert discord account with id '{uid}'! {err:#?}");
                AuthError::MongoDbError
            })?;
        let premium_details = self.client.get_premium_details(uid).await.map_err(|err| {
            warn!("{who}: could not get premium details for id '{uid}'! {err:#?}");
            AuthError::MongoDbError
        })?;

        self.connections
            .authorize_one(cid, discord_acc, member, premium_details)
            .await
    }

    /// Terminate an authorized connection.
    pub async fn terminate_connection(
        &self,
        who: SocketAddr,
        uid: serenity::UserId,
        cid: Simple,
    ) -> Result<()> {
        if !self.connections.has_active_session(&cid) {
            self.connections
                .remove_authorized(&cid, &uid)
                .with_context(|| format!("{who}: could not remove authorized connection!"))?;

            return Ok(());
        }

        todo!("Implement terminate connection with the 1min grace period etc");
        // let mut sessions_lock = self
        //     .connections
        //     .get_mut(&discord_id)
        //     .ok_or_else(|| anyhow!("there is no connection list for
        // {discord_id}!"))?; let index = sessions_lock
        //     .iter()
        //     .position(|connection| connection.id == connection_id)
        //     .ok_or_else(|| anyhow!("no connection with id {connection_id} for
        // {discord_id}!"))?;

        // let connection = sessions_lock.remove(index);

        // info!("{who}: ended connection with id {connection_id} for
        // {discord_id}");

        // let is_empty = sessions_lock.is_empty();

        // drop(sessions_lock);

        // if is_empty {
        //     self.connections.remove(&discord_id);
        // }

        // if let Some(user) = connection.user {
        //     user.terminate_session().await?;
        // }

        // Ok(())
    }

    pub fn terminate_unauthorized_connection(&self, who: SocketAddr, cid: Simple) {
        self.connections.remove_unauthorized(&cid);

        info!("{who}: terminated unauthorized connection, cid: '{cid}'")
    }

    // TODO: Docs.
    /// Dispatches a single message from an authorized connection.
    pub async fn dispatch_socket_message(
        &self,
        uid: serenity::UserId,
        cid: Simple,
        msg: Message,
    ) -> Result<()> {
        if msg.target != Target::Backend {
            bail!("Incorrect target: `{:?}`! `{msg:?}`!", msg.target);
        }
        if msg.kind == MessageKind::Response {
            bail!("Incorrect kind: `{:?}`! `{msg:?}`!", msg.kind); // Temporary, we might want some responses in the future.
        }

        match msg.task {
            Task::LogOut => {
                let all_devices = msg
                    .log_out
                    .ok_or_else(|| anyhow!("`Message` missing `LogOutDetails`!"))?
                    .all_devices;
            }
            _ => bail!("Incorrect task: `{:?}`! `{msg:?}`", msg.task),
        }

        Ok(())
    }

    /// Withdraw access for an authorized session, by removing the current user
    /// data.
    fn revoke_connection(
        &self,
        uid: serenity::UserId,
        cid: Simple,
        all_devices: bool,
    ) -> Result<()> {
        self.connections.revoke(&uid, &cid, all_devices)?;
        Ok(())
    }
}
