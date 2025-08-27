use std::net::SocketAddr;

use axum::{
    Router,
    extract::{ConnectInfo, State, WebSocketUpgrade},
    http::{Method, header},
    response::IntoResponse,
    routing::any,
};
use axum_extra::{TypedHeader, headers};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, TraceLayer},
};

use crate::prelude::*;

mod callback;
mod login;
mod ws;

pub(super) fn ws() -> Router<AppState> {
    Router::new()
        .route("/ws", any(ws_handler))
        .route_layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::OPTIONS, Method::GET])
                .allow_headers([
                    header::CONNECTION,
                    header::SEC_WEBSOCKET_EXTENSIONS,
                    header::SEC_WEBSOCKET_KEY,
                    header::SEC_WEBSOCKET_VERSION,
                    header::UPGRADE,
                ]),
        )
        .route_layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
}

/// The handler for the HTTP request (this gets called when the HTTP request
/// lands at the start of websocket negotiation). After this completes, the
/// actual switching from HTTP to websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP
/// address of the client as well as things from HTTP headers such as user-agent
/// of the browser etc.
async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = match user_agent {
        Some(TypedHeader(user_agent)) => user_agent.to_string(),
        _ => String::from("Unknown browser"),
    };

    info!("`{user_agent}` at {addr} connected.");
    ws.on_upgrade(move |socket| ws::handle_upgrade(socket, addr, state.clone()))
}

pub(super) fn login() -> Router<AppState> {
    Router::new().route("/login", any(login::route))
}

pub(super) fn callback() -> Router<AppState> {
    Router::new().route("/callback", any(callback::route))
}
