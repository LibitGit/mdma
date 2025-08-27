// TODO: Changing the setting for per character/account settings should be done
// via a separate ws task.       Changes per account by default.
//       The changes should take place immediatelly ?
//! Implementation of mdma web server.

#![forbid(unsafe_code)]
// #![deny(
//     bad_style,
//     dead_code,
//     improper_ctypes,
//     non_shorthand_field_patterns,
//     no_mangle_generic_items,
//     overflowing_literals,
//     path_statements,
//     patterns_in_fns_without_body,
//     private_interfaces,
//     private_bounds,
//     unconditional_recursion,
//     unused,
//     unused_allocation,
//     unused_comparisons,
//     unused_parens,
//     while_true,
//     missing_debug_implementations,
//     missing_copy_implementations,
//     missing_docs,
//     trivial_casts,
//     trivial_numeric_casts,
//     unused_import_braces,
//     unused_results
// )]
use std::net::Ipv4Addr;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod prelude {
    pub use crate::{
        app_state::{
            AppState,
            client::{Client, DiscordAccount},
        },
        auth::{
            AuthError,
            jwt::{AccessClaims, AccessLevel, Jwt, RefreshClaims},
        },
        to_u64,
        types::{ContextExt, Context, GameAccountId, GameCharId, OAuth2Client, Result},
    };
    // pub use anyhow::Context as AnyhowContext;
    pub use poise::serenity_prelude as serenity;
    pub use scopeguard::{ScopeGuard, guard};
    pub use tracing::{debug, info, warn};
}

#[macro_use(defer)]
extern crate scopeguard;

/// Module containing macro_rules macro implementations.
#[macro_use]
mod macros;
/// Module containing the state of the app shared between requests and web
/// socket connections.
mod app_state;

mod auth;
mod discord_bot;
mod routes;
mod types;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::builder()
                .parse(format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                ))
                .unwrap(),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let ip = match cfg!(target_os = "windows") {
        true => Ipv4Addr::LOCALHOST,
        false => Ipv4Addr::UNSPECIFIED,
    };
    let port = 3000;
    let listener = tokio::net::TcpListener::bind((ip, port)).await.unwrap();

    tracing::debug!("Listening on {}", listener.local_addr().unwrap());

    let client = app_state::client::Client::connect().await.unwrap();
    let (tx, rx) = futures::channel::oneshot::channel();

    tokio::spawn(discord_bot::start(client, tx));

    let app_state = rx.await.unwrap();

    // app_state.get_member_data(serenity::UserId::new(419459580335095808)).await.
    // unwrap();

    let app = axum::Router::new()
        .merge(routes::ws())
        .merge(routes::login())
        .merge(routes::callback())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(app_state)
        .into_make_service_with_connect_info::<std::net::SocketAddr>();

    axum::serve(listener, app).await.unwrap();
}

pub const fn to_u64(s: &str) -> u64 {
    let mut res = 0;
    let mut i = 0;
    while i < s.len() {
        let b = s.as_bytes()[i];
        res = 10 * res + (b - b'0') as u64;
        i += 1;
    }
    res
}
