pub mod models;
mod plugins;
pub mod schema;
mod utils;

#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use crate::utils::osu::tracking::OsuTracker;
use chrono::{DateTime, Utc};
use diesel::Connection;
use diesel_async::AsyncPgConnection;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use mobc::Pool;
use poise::serenity_prelude::{EventHandler, FullEvent, Token, async_trait};
use rosu_v2::prelude::Osu;
use std::env;
use std::sync::Arc;
use tracing::{error, info};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub struct Data {
    time_started: DateTime<Utc>,
    osu_client: Arc<Osu>,
    db_pool: Pool<AsyncDieselConnectionManager<AsyncPgConnection>>,
    http_client: reqwest::Client,
    songbird: Arc<songbird::Songbird>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

type Context<'a> = poise::Context<'a, Data, Error>;

type PartialContext<'a> = poise::PartialContext<'a, Data, Error>;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn dispatch(&self, ctx: &serenity::all::Context, event: &FullEvent) {
        match event {
            FullEvent::Ready { data_about_bot, .. } => {
                info!("{} is connected!", data_about_bot.user.name);
                let mut osu_tracker = OsuTracker {
                    cache: ctx.cache.clone(),
                    http: ctx.http.clone(),
                    osu_client: ctx.data::<Data>().osu_client.clone(),
                    pool: ctx.data::<Data>().db_pool.clone(),
                };

                tokio::spawn(async move {
                    match osu_tracker.tracking_loop().await {
                        Ok(()) => {}
                        Err(why) => error!("{why}"),
                    };
                });

                let cloned_ctx = ctx.clone();
                tokio::spawn(async move {
                    tokio::signal::ctrl_c()
                        .await
                        .expect("Could not register ctrl+c handler");

                    cloned_ctx.shutdown_all();
                });
            }
            FullEvent::CacheReady { guilds, .. } => {
                info!("Cache ready: {} guilds cached.", guilds.len());
            }
            FullEvent::Message { new_message, .. } => {
                match plugins::summary::add_message(new_message, &ctx.data::<Data>(), &ctx.cache)
                    .await
                {
                    Ok(()) => {}
                    Err(e) => error!("{e}"),
                }
            }
            FullEvent::VoiceStateUpdate { old, .. } => {
                let Some(voice) = old else { return };
                match plugins::music::check_for_empty_channel(ctx, voice.guild_id).await {
                    Ok(()) => {}
                    Err(e) => error!("{e}"),
                }
            }
            _ => {}
        }
    }
}

async fn pre_command(ctx: Context<'_>) {
    info!(
        "@{} ({}) -> {}",
        ctx.author().name,
        match ctx.guild() {
            Some(guild) => guild.name.to_string(),
            _ => {
                "Direct Message".to_string()
            }
        },
        ctx.invocation_string()
    );
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!(
                "Command '{}' returned error {:?}",
                ctx.command().name,
                error
            );
            if let Err(why) = ctx
                .say("The command returned an error. Try again later.")
                .await
            {
                error!("Error while handling error: {}", why);
            }
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
    dotenvy::dotenv().expect("Failed to load .env file");

    let db_pool = utils::db::establish_connection::establish_connection();

    let res = tokio::task::block_in_place(move || {
        let mut migration_connection = AsyncConnectionWrapper::<AsyncPgConnection>::establish(
            &env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        )?;

        migration_connection.run_pending_migrations(MIGRATIONS)?;

        Ok::<(), Error>(())
    });

    if let Err(why) = res {
        panic!("Couldn't run migrations: {why:?}");
    }

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
            plugins::osu::osu(),
            plugins::osu::top(),
            plugins::osu::recent(),
            plugins::osu::recent_best(),
            plugins::osu::recent_list(),
            plugins::osu::score(),
            plugins::osu::scores(),
            plugins::osu::firsts(),
            plugins::osu::pins(),
            // This function registers slash commands on Discord. When you change something about a
            // command signature, for example by changing its name, adding or removing parameters, or
            // changing a parameter type, you should call this function.
            plugins::basic::register(),
            plugins::summary::summary(),
            plugins::summary::summary_enable(),
            plugins::summary::summary_disable(),
        ],
        on_error: |error| Box::pin(on_error(error)),
        // Set a function to be called prior to each command execution. This
        // provides all context of the command that would also be passed to the actual command code
        pre_command: |ctx| Box::pin(pre_command(ctx)),
        // Similar to `pre_command`, except will be called directly _after_
        // command execution.

        // Options specific to prefix.rs commands, i.e. commands invoked via chat messages
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: None,
            dynamic_prefix: Some(|c| Box::pin(utils::db::prefix::get_guild_prefix(c))),
            mention_as_prefix: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let file_appender = tracing_appender::rolling::daily("logs", "info.log");

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(non_blocking)
        .init();

    let token = Token::from_env("DISCORD_TOKEN").unwrap();

    let intents = serenity::prelude::GatewayIntents::GUILD_MEMBERS
        | serenity::prelude::GatewayIntents::GUILD_VOICE_STATES
        | serenity::prelude::GatewayIntents::GUILD_PRESENCES
        | serenity::prelude::GatewayIntents::MESSAGE_CONTENT
        | serenity::prelude::GatewayIntents::GUILD_MESSAGES
        | serenity::prelude::GatewayIntents::GUILDS
        | serenity::prelude::GatewayIntents::DIRECT_MESSAGES;

    let client_id = env::var("OSU_CLIENT_ID")
        .expect("Expected an osu client id in the environment")
        .parse::<u64>()
        .expect("Failed to parse client_id.");

    let client_secret =
        env::var("OSU_CLIENT_SECRET").expect("Expected an osu client secret in the environment");

    let osu_client: Arc<Osu> = match Osu::new(client_id, client_secret).await {
        Ok(client) => Arc::new(client),
        Err(why) => panic!("Failed to create client or make initial osu!api interaction: {why}"),
    };

    let framework = poise::Framework::new(options);

    let manager = songbird::Songbird::serenity();

    let mut client = serenity::Client::builder(token, intents)
        .framework(framework)
        .voice_manager::<songbird::Songbird>(manager.clone())
        .event_handler(Handler)
        .data(Arc::new(Data {
            time_started: Utc::now(),
            osu_client,
            db_pool,
            http_client: reqwest::Client::new(),
            songbird: manager,
        }))
        .await
        .unwrap();

    if let Err(why) = client.start_autosharded().await {
        error!("Client error: {:?}", why);
    }
}
