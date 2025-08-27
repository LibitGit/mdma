use async_session::SessionStore;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use axum_extra::{TypedHeader, headers};
use oauth2::CsrfToken;
use serde::Deserialize;

use crate::prelude::*;

use super::login::{COOKIE_NAME, CSRF_TOKEN};

#[derive(Debug, Deserialize)]
pub(super) struct AuthRequest {
    state: String,
    code: String,
}

pub(super) async fn route(
    State(state): State<AppState>,
    Query(query): Query<AuthRequest>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> Result<impl IntoResponse> {
    csrf_token_validation_workflow(&query, &cookies, &state).await?;

    Ok(Redirect::to(&format!(
        "https://{}.chromiumapp.org/?code={}",
        std::env::var("EXTENSION_ID")?,
        query.code
    )))
}

async fn csrf_token_validation_workflow(
    auth_request: &AuthRequest,
    cookies: &headers::Cookie,
    state: &AppState,
) -> Result<()> {
    // Extract the cookie from the request
    let cookie = cookies
        .get(COOKIE_NAME)
        .context("unexpected error getting cookie name")?
        .to_string();

    // Load the session
    let session = match state
        .store
        .load_session(cookie)
        .await
        .context("failed to load session")?
    {
        Some(session) => session,
        None => return Err(anyhow!("Session not found").into()),
    };

    // Extract the CSRF token from the session
    let stored_csrf_token = session
        .get::<CsrfToken>(CSRF_TOKEN)
        .context("CSRF token not found in session")?;

    // Cleanup the CSRF token session
    state
        .store
        .destroy_session(session)
        .await
        .context("Failed to destroy old session")?;

    // Validate CSRF token is the same as the one in the auth request
    if *stored_csrf_token.secret() != auth_request.state {
        return Err(anyhow!("CSRF token mismatch").into());
    }

    Ok(())
}
