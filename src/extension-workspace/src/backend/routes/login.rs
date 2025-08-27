use std::time::Duration;

use async_session::{Session, SessionStore};
use axum::{
    extract::State,
    http::{HeaderMap, header::SET_COOKIE},
    response::{IntoResponse, Redirect},
};
use oauth2::{CsrfToken, Scope};

use crate::prelude::*;

pub const COOKIE_NAME: &str = "SESSION";
pub const CSRF_TOKEN: &str = "csrf_token";
const SESSION_TTL: u64 = 10 * 60; // 10 min in sec

pub(super) async fn route(State(state): State<AppState>) -> Result<impl IntoResponse> {
    // if !state.connections.has_unauthorized(&cid) {
    //     bail!("Nie znaleziono odpowiedniego połączenia o id '{cid}'!")
    // }

    let (auth_url, csrf_token) = state
        .oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .add_scope(Scope::new("guilds.members.read".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .url();
    let mut session = Session::new();
    session.expire_in(Duration::from_secs(SESSION_TTL));
    session.insert(CSRF_TOKEN, &csrf_token)?;

    // Store the session in MemoryStore and retrieve the session cookie
    let cookie = state
        .store
        .store_session(session)
        .await?
        .ok_or_else(|| anyhow!("unexpected error retrieving CSRF cookie value"))?;

    // Attach the session cookie to the response header
    let cookie = format!("{COOKIE_NAME}={cookie}; SameSite=Lax; HttpOnly; Secure; Path=/");
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse()?);

    Ok((headers, Redirect::to(auth_url.as_ref())))
}
