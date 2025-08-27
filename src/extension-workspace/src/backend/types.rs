use std::fmt;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
// use async_session::{Result, Session, SessionStore, async_trait, serde_json};
use oauth2::{
    EmptyExtraTokenFields, EndpointNotSet, EndpointSet, RevocationErrorResponseType,
    StandardErrorResponse, StandardRevocableToken, StandardTokenIntrospectionResponse,
    StandardTokenResponse,
    basic::{BasicErrorResponseType, BasicTokenType},
};
// use redis::{AsyncCommands, Client, IntoConnectionInfo, RedisResult,
// aio::Connection};

#[derive(Debug)]
#[repr(transparent)]
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Coś poszło nie tak! {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type Context<'a> = poise::Context<'a, crate::app_state::AppState, AppError>;

pub type Result<T> = std::result::Result<T, AppError>;

pub type OAuth2Client = oauth2::Client<
    StandardErrorResponse<BasicErrorResponseType>,
    StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardTokenIntrospectionResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardRevocableToken,
    StandardErrorResponse<RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

crate::id_u64! {
    #[doc = "Game account user id."]
    GameAccountId;

    #[doc = "Game character user id."]
    GameCharId;
}

pub trait ContextExt<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static;
    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

// TODO: Check if this generates backtrace as the original anyhow::Context trait
// would.
impl<T, E> ContextExt<T> for std::result::Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => Err(AppError(error.into().context(context))),
        }
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => Err(AppError(error.into().context(f()))),
        }
    }
}

impl<T> ContextExt<T> for Result<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(AppError(error)) => Err(AppError(error.context(context))),
        }
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(AppError(error)) => Err(AppError(error.context(f()))),
        }
    }
}

#[repr(transparent)]
struct DisplayError<M>(M);

impl<M> fmt::Debug for DisplayError<M>
where
    M: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<M> fmt::Display for DisplayError<M>
where
    M: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<M> core::error::Error for DisplayError<M> where M: fmt::Display {}

impl<T> ContextExt<T> for Option<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        match self {
            Some(ok) => Ok(ok),
            None => Err(AppError(DisplayError(context).into())),
        }
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        match self {
            Some(ok) => Ok(ok),
            None => Err(AppError(DisplayError(f()).into())),
        }
    }
}

// TODO: Impl this after 0.15.0 for now use MemoryStore.
// /// # RedisSessionStore
// #[derive(Clone, Debug)]
// pub struct RedisSessionStore {
//     client: Client,
//     prefix: Option<String>,
// }

// impl RedisSessionStore {
//     /// creates a redis store from an existing [`redis::Client`]
//     /// ```rust
//     /// # use async_redis_session::RedisSessionStore;
//     /// let client = redis::Client::open("redis://127.0.0.1").unwrap();
//     /// let store = RedisSessionStore::from_client(client);
//     /// ```
//     pub fn from_client(client: Client) -> Self {
//         Self {
//             client,
//             prefix: None,
//         }
//     }

//     /// creates a redis store from a [`redis::IntoConnectionInfo`]
//     /// such as a [`String`], [`&str`](str), or
// [`Url`](../url/struct.Url.html)     /// ```rust
//     /// # use async_redis_session::RedisSessionStore;
//     /// let store = RedisSessionStore::new("redis://127.0.0.1").unwrap();
//     /// ```
//     pub fn new(connection_info: impl IntoConnectionInfo) -> RedisResult<Self>
// {         Ok(Self::from_client(Client::open(connection_info)?))
//     }

//     /// sets a key prefix for this session store
//     ///
//     /// ```rust
//     /// # use async_redis_session::RedisSessionStore;
//     /// let store = RedisSessionStore::new("redis://127.0.0.1").unwrap()
//     ///     .with_prefix("async-sessions/");
//     /// ```
//     /// ```rust
//     /// # use async_redis_session::RedisSessionStore;
//     /// let client = redis::Client::open("redis://127.0.0.1").unwrap();
//     /// let store = RedisSessionStore::from_client(client)
//     ///     .with_prefix("async-sessions/");
//     /// ```
//     pub fn with_prefix(mut self, prefix: impl AsRef<str>) -> Self {
//         self.prefix = Some(prefix.as_ref().to_owned());
//         self
//     }

//     async fn ids(&self) -> Result<Vec<String>> {
//         Ok(self.connection().await?.keys(self.prefix_key("*")).await?)
//     }

//     /// returns the number of sessions in this store
//     pub async fn count(&self) -> Result<usize> {
//         if self.prefix.is_none() {
//             let mut connection = self.connection().await?;
//             Ok(redis::cmd("DBSIZE").query_async(&mut connection).await?)
//         } else {
//             Ok(self.ids().await?.len())
//         }
//     }

//     #[cfg(test)]
//     async fn ttl_for_session(&self, session: &Session) -> Result<usize> {
//         Ok(self
//             .connection()
//             .await?
//             .ttl(self.prefix_key(session.id()))
//             .await?)
//     }

//     fn prefix_key(&self, key: impl AsRef<str>) -> String {
//         if let Some(ref prefix) = self.prefix {
//             format!("{}{}", prefix, key.as_ref())
//         } else {
//             key.as_ref().into()
//         }
//     }

//     async fn connection(&self) -> RedisResult<Connection> {
//         self.client.get_async_std_connection().await
//     }
// }

// #[async_trait]
// impl SessionStore for RedisSessionStore {
//     async fn load_session(&self, cookie_value: String) ->
// Result<Option<Session>> {         let id =
// Session::id_from_cookie_value(&cookie_value)?;         let mut connection =
// self.connection().await?;         let record: Option<String> =
// connection.get(self.prefix_key(id)).await?;         match record {
//             Some(value) => Ok(serde_json::from_str(&value)?),
//             None => Ok(None),
//         }
//     }

//     async fn store_session(&self, session: Session) -> Result<Option<String>>
// {         let id = self.prefix_key(session.id());
//         let string = serde_json::to_string(&session)?;

//         let mut connection = self.connection().await?;

//         match session.expires_in() {
//             None => connection.set(id, string).await?,

//             Some(expiry) => {
//                 connection
//                     .set_ex(id, string, expiry.as_secs() as usize)
//                     .await?
//             }
//         };

//         Ok(session.into_cookie_value())
//     }

//     async fn destroy_session(&self, session: Session) -> Result {
//         let mut connection = self.connection().await?;
//         let key = self.prefix_key(session.id().to_string());
//         connection.del(key).await?;
//         Ok(())
//     }

//     async fn clear_store(&self) -> Result {
//         let mut connection = self.connection().await?;

//         if self.prefix.is_none() {
//             let _: () = redis::cmd("FLUSHDB").query_async(&mut
// connection).await?;         } else {
//             let ids = self.ids().await?;
//             if !ids.is_empty() {
//                 connection.del(ids).await?;
//             }
//         }
//         Ok(())
//     }
// }
