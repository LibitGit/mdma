use std::{sync::Arc, time::Duration};

use futures::channel::oneshot::Sender;

use crate::prelude::*;

mod commands;

/// Start the discord bot and send the app state after serenity::Context is
/// available.
pub(super) async fn start(client: Client, tx: Sender<AppState>) {
    let token = const { env!("DISCORD_TOKEN") };
    let intents = serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::DIRECT_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::helpers::doc()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("?".into()),
                additional_prefixes: vec![
                    poise::Prefix::Literal("🦀 "),
                    poise::Prefix::Literal("🦀"),
                ],
                edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                    Duration::from_secs(60 * 5), // 5 minutes
                ))),
                ..Default::default()
            },
            // This code is run after a command if it was successful (returned Ok)
            post_command: |ctx| {
                Box::pin(async move {
                    info!("Executed command {}!", ctx.command().qualified_name);
                })
            },
            // Every command invocation must pass this check to continue execution
            command_check: Some(|_ctx| Box::pin(async move { Ok(true) })),
            // Enforce command checks even for owners (enforced by default)
            // Set to true to bypass checks, which is useful for testing
            skip_checks_for_owners: false,
            event_handler: |ctx, event, _framework, data| {
                Box::pin(async move { event_handler(ctx, event, data).await })
            },
            // Disallow all mentions (except those to the replied user) by default
            allowed_mentions: Some(serenity::CreateAllowedMentions::new().replied_user(true)),
            ..Default::default()
        })
        .setup(|cache_http, _ready, framework| {
            Box::pin(async move {
                let my_state = AppState::new(client, cache_http.clone())?;
                tx.send(my_state.clone()).unwrap();
                poise::builtins::register_globally(cache_http, &framework.options().commands)
                    .await?;
                Ok(my_state)
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap()
}

async fn event_handler(
    _ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _data: &AppState,
) -> Result<()> {
    debug!(
        "Got an event in event handler: {:?}",
        event.snake_case_name()
    );

    Ok(())
}
