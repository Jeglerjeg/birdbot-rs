pub mod models;
mod plugins;
pub mod schema;
mod utils;

use chrono::{DateTime, Utc};
use poise::serenity_prelude;
use songbird::SerenityInit;
use std::env;
use tracing::{error, info};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub struct Data {
    time_started: DateTime<Utc>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

type Context<'a> = poise::Context<'a, Data, Error>;

type PartialContext<'a> = poise::PartialContext<'a, Data, Error>;

async fn event_listener(
    ctx: &serenity_prelude::Context,
    event: &poise::Event<'_>,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _user_data: &Data,
) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => {
            info!("{} is connected!", data_about_bot.user.name);
        }
        poise::Event::VoiceStateUpdate { old, new: _new } => {
            let voice = match old {
                Some(old) => old.clone(),
                _ => return Ok(()),
            };
            plugins::music::check_for_empty_channel(ctx.clone(), voice.guild_id).await;
        }
        _ => {}
    }

    Ok(())
}

async fn pre_command(ctx: Context<'_>) {
    info!(
        "@{}#{} ({}) -> {}",
        ctx.author().name,
        ctx.author().discriminator,
        match ctx.guild() {
            Some(guild) => guild.name,
            _ => {
                match ctx.channel_id().name(ctx.discord()).await {
                    Some(channel_name) => channel_name,
                    _ => "Direct Message".into(),
                }
            }
        },
        ctx.command().name
    );
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx } => {
            error!(
                "Command '{}' returned error {:?}",
                ctx.command().name,
                error
            );
        }
        poise::FrameworkError::Listener { error, event, .. } => {
            error!(
                "Listener returned error during {:?} event: {:?}",
                event.name(),
                error
            );
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // This will load the environment variables located at `./.env`, relative to
    // the CWD. See `./.env.example` for an example on how to structure this.
    dotenv::dotenv().expect("Failed to load .env file");

    let connection = &mut utils::db::establish_connection::establish_connection();

    connection.run_pending_migrations(MIGRATIONS).unwrap();

    let options = poise::FrameworkOptions {
        commands: vec![
            plugins::basic::help(),
            plugins::basic::ping(),
            plugins::basic::info(),
            plugins::basic::roll(),
            plugins::basic::avatar(),
            plugins::basic::prefix(),
            plugins::basic::stop(),
            plugins::music::music(),
            plugins::wyr::wyr(),
            // This function registers slash commands on Discord. When you change something about a
            // command signature, for example by changing its name, adding or removing parameters, or
            // changing a parameter type, you should call this function.
            plugins::basic::register(),
        ],
        listener: |ctx, event, framework, user_data| {
            Box::pin(event_listener(ctx, event, framework, user_data))
        },
        on_error: |error| Box::pin(on_error(error)),
        // Set a function to be called prior to each command execution. This
        // provides all context of the command that would also be passed to the actual command code
        pre_command: |ctx| Box::pin(pre_command(ctx)),
        // Similar to `pre_command`, except will be called directly _after_
        // command execution.

        // Options specific to prefix.rs commands, i.e. commands invoked via chat messages
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: None,
            dynamic_prefix: Some(|c| {
                Box::pin(utils::db::prefix::get_guild_prefix(
                    c,
                    env::var("PREFIX").unwrap_or_else(|_| String::from(">")),
                ))
            }),
            mention_as_prefix: true,
            ..Default::default()
        },
        ..Default::default()
    };

    // Initialize the logger to use environment variables.
    //
    // In this case, a good default is setting the environment variable
    // `RUST_LOG` to `debug`.

    let file_appender = tracing_appender::rolling::daily("logs", "info.log");

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(non_blocking)
        .init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = serenity_prelude::GatewayIntents::all();

    let framework = poise::Framework::builder()
        .client_settings(SerenityInit::register_songbird)
        .token(token.clone())
        .intents(intents)
        .options(options)
        .user_data_setup(|_ctx, _data_about_bot, _framework| {
            Box::pin(async move {
                Ok(Data {
                    time_started: Utc::now(),
                })
            })
        })
        .build()
        .await
        .unwrap();

    let shard_manager = framework.shard_manager().clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = framework.start().await {
        error!("Client error: {:?}", why);
    }
}
