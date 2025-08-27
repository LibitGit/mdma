use std::net::SocketAddr;

use axum::{
    body::Bytes,
    extract::ws::{Message as WsMessage, WebSocket},
};
use common::messaging::prelude::*;
use futures::{SinkExt, StreamExt, channel::mpsc, stream::SplitStream};
use oauth2::{AuthorizationCode, TokenResponse};
use serde::Deserialize;
use uuid::fmt::Simple;

use crate::prelude::*;

pub(super) async fn handle_upgrade(socket: WebSocket, who: SocketAddr, state: AppState) {
    let (tx, mut rx) = mpsc::unbounded();
    let (mut socket_tx, socket_rx) = socket.split();

    tokio::spawn(async move {
        while let Some(msg) = rx.next().await {
            if let Err(err) = socket_tx.send(msg).await {
                warn!("{who}: failed to send msg! {err:#?}");
            }
        }
    });

    let (mut rx, uid, cid) = match AuthorizedConnection::new(tx, socket_rx, who, &state).await {
        Ok(Some(details)) => (details.rx, details.uid, details.cid),
        Ok(None) => {
            debug!("{who}: socket connection closed!");
            return;
        }
        Err(err) => {
            warn!("{who}: failed to establish authorized connection! {err:#?}");
            return;
        }
    };

    let _guard = scopeguard::guard(state.clone(), |state| {
        tokio::task::spawn(async move { state.terminate_connection(who, uid, cid).await });
    });

    while let Some(socket_message) = rx.next().await {
        let msg = match socket_message {
            Ok(WsMessage::Close(_)) => return,
            Ok(socket_message) => match Message::try_from(socket_message) {
                Ok(msg) => msg,
                Err(msg) => {
                    warn!("{who}: failed to parse `ws::Message`! {msg:?}");
                    return;
                }
            },
            Err(err) => {
                warn!("{who}: rx error! {err:#?}");
                return;
            }
        };

        if let Err(err) = state.dispatch_socket_message(uid, cid, msg).await {
            warn!("{who}: error while trying to dispatch socket `Message`! {err:#?}");
            break;
        }
        // if let Err(err) = connections
        //     .handle_message(&db, &uuid, &extension_id, msg)
        //     .await
        // {
        //     eprintln!("Error when trying to handle message: {err}");
        //     connections.disconnect(&db, &uuid).await;
        // }
    }

    // if let Err(err) = state.end_connection(who, discord_id,
    // connection_id).await {     warn!("{who}: {err}");
    // }
}

#[derive(Debug, Deserialize)]
struct DiscordUserData {
    id: serenity::UserId,
    #[serde(rename = "verified")]
    email_verified: bool,
    // email: String,
}

impl DiscordUserData {
    async fn fetch(state: &AppState, auth_code: String) -> Result<Self> {
        let token_response = state
            .oauth_client
            .exchange_code(AuthorizationCode::new(auth_code))
            .request_async(&state.http_client)
            .await?;

        let acc_data = state
            .http_client
            .get("https://discord.com/api/users/@me")
            .bearer_auth(token_response.access_token().secret())
            .send()
            .await?
            .json::<Self>()
            .await?;

        Ok(acc_data)
    }
}

/// [`Connection`][connection] details used after establishing an authorized
/// connection.
///
/// # Authorizing
/// For the connection to be authorized the user needs to provide their discord
/// account details (in the form of a valid
/// [`Jwt::<RefreshClaims>`][crate::prelude::Jwt]) in a [handshake
/// task][Task::Handshake].
///
/// If the user fails to do so, the connection is kept in the [`AppState`] as
/// unauthorized until they log in via the extension's popup.
/// Unauthorized meaning only data needed to login is stored - socket's `tx` and
/// `cid`.
///
/// After a successful handshake the user is authorized (granted a
/// `Jwt::<AccessClaims>`). Their account details along with a
/// [`SessionScope`][scope] get stored in the `user` field of a connection until
/// the connection terminates.
///
/// [connection]: crate::app_state::connections::Connection
/// [scope]: crate::app_state::connections::SessionScope
struct AuthorizedConnection {
    /// Receiving side of the WebSocket.
    rx: SplitStream<WebSocket>,
    /// User id.
    uid: serenity::UserId,
    /// Connection id
    cid: Simple,
}

impl AuthorizedConnection {
    const fn _new(rx: SplitStream<WebSocket>, uid: serenity::UserId, cid: Simple) -> Self {
        Self { rx, uid, cid }
    }

    fn new_cid() -> Simple {
        uuid::Uuid::new_v4().simple()
    }

    /// Wait for establishing an authorized connection over a given socket.
    ///
    /// If the user doesn't provide valid credentials an unauthorized connection
    /// is stored and kept alive until they log in.
    ///
    /// # Errors
    /// This method returns an [`Err`] if any suspicious activity takes place.
    ///
    /// This includes but is not limited to:
    /// - first message not having a correct [`kind`][MessageKind],
    ///   [`target`][Target] or [`task`][Task],
    /// - providing a fraudulent [`refresh_token`][Jwt],
    /// - in the case of an unauthorized connection, receiving a `task` which
    ///   isn't an instance of `Task::Heartbeat` or `Task::Handshake`.
    async fn new(
        mut tx: mpsc::UnboundedSender<WsMessage>,
        mut socket_rx: SplitStream<WebSocket>,
        who: SocketAddr,
        state: &AppState,
    ) -> Result<Option<Self>> {
        let socket_message = match Self::ping(&mut tx, &mut socket_rx).await? {
            Some(first_msg) => first_msg,
            None => match socket_rx.next().await {
                Some(msg_res) => msg_res?,
                None => return Ok(None),
            },
        };
        let msg = Message::try_from(socket_message)
            .map_err(|msg| anyhow!("Failed to parse message! {msg:?}"))?;

        MessageValidator::new(Target::Background).validate(&msg)?;

        if msg.task == Task::Handshake {
            return Self::from_unauthorized(tx, socket_rx, who, state, Task::Handshake).await;
        }
        if msg.task != Task::Tokens {
            bail!("Incorrect first task!")
        }

        let refresh_token = msg
            .refresh_token
            .ok_or_else(|| anyhow!("Missing refresh token!"))?;

        match state.client.validate_refresh_token(&refresh_token).await {
            Ok(discord_acc) => {
                let cid = Self::new_cid();

                state
                    .establish_connection(tx, who, cid, &discord_acc)
                    .await?;

                Ok(Some(Self::_new(socket_rx, discord_acc.id, cid)))
            }
            Err(AuthError::TokenFraudulent) | Err(AuthError::WrongCredentials) => {
                todo!("Fraudulent token {who}: {refresh_token:?}")
            }
            Err(AuthError::InvalidToken) => {
                Self::from_unauthorized(tx, socket_rx, who, state, Task::Tokens).await
            }
            Err(AuthError::MissingCredentials) => {
                todo!("Secret missing for validation, retry maybe?")
            }
            Err(AuthError::MongoDbError) => todo!("MONGODB ERROR"),
            _ => unreachable!(),
        }
    }

    /// Ping a socket and return the response if it wasn't a [`Pong`][WsMessage]
    /// or a [`Close`][WsMessage].
    async fn ping(
        tx: &mut mpsc::UnboundedSender<WsMessage>,
        socket_rx: &mut SplitStream<WebSocket>,
    ) -> Result<Option<WsMessage>> {
        tx.send(WsMessage::Ping(Bytes::from_static(&[1, 2, 3])))
            .await?;

        let Some(msg) = socket_rx.next().await.transpose()? else {
            return Ok(None);
        };

        match msg {
            WsMessage::Pong(_) | WsMessage::Close(_) => Ok(None),
            _ => Ok(Some(msg)),
        }
    }

    /// Wait for the user to authorize a connection by logging in.
    ///
    /// For the user to be able to log in the `socket`'s sink and the `cid` need
    /// to be stored in the [`Connections`][connections] as part of an
    /// unauthorized [`Connection`][connection]. An instance of
    /// [`AuthorizedConnection`] is returned after the user logs in successfuly.
    ///
    /// [connections]: crate::app_state::connections::Connections
    /// [connection]: crate::app_state::connections::Connection
    async fn from_unauthorized(
        mut tx: mpsc::UnboundedSender<WsMessage>,
        mut socket_rx: SplitStream<WebSocket>,
        who: SocketAddr,
        state: &AppState,
        task: Task,
    ) -> Result<Option<Self>> {
        let cid = Self::new_cid();
        let guard = guard(state.clone(), |state| {
            state.terminate_unauthorized_connection(who, cid)
        });

        state.establish_unauthorized_connection(tx.clone(), who, cid);
        tx.send(Message::new(task, Target::Background, MessageKind::Response).into_ws_message()?)
            .await?;

        while let Some(socket_message) = socket_rx.next().await {
            let msg = match socket_message? {
                WsMessage::Close(_) => return Ok(None),
                socket_message => Message::try_from(socket_message)
                    .map_err(|_| anyhow!("Failed to parse message!"))?,
            };

            MessageValidator::new(Target::Background).validate(&msg)?;

            if msg.task == Task::KeepAlive {
                continue;
            }
            if msg.task != Task::Tokens {
                bail!("Incorrect task `{task:?}`! {msg:?}");
            }

            let code = msg.code.ok_or_else(|| anyhow!("Incorrect handshake!"))?;
            let user = DiscordUserData::fetch(state, code).await?;
            let Err(err) = state
                .authorize_one_connection(who, cid, user.id, user.email_verified)
                .await
            else {
                // After authorization the connection terminates via `terminate_connection`.
                ScopeGuard::into_inner(guard);

                return Ok(Some(Self::_new(socket_rx, user.id, cid)));
            };
            let err_msg = match err {
                AuthError::MissingCredentials => "",
                _ => "Failed to authorize user.",
            };

            tx.send(
                Message::builder(Task::Tokens, Target::Background, MessageKind::Response)
                    .error(err_msg)
                    .build()
                    .into_ws_message()?,
            )
            .await?;
        }

        Ok(None)
    }
}
