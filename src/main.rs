pub mod models;
mod plugins;
pub mod schema;
mod utils;

use crate::utils::osu::tracking::OsuTracker;
use chrono::{DateTime, Utc};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use poise::serenity_prelude::FullEvent;
use rosu_v2::prelude::Osu;
use songbird::SerenityInit;
use std::env;
use std::sync::Arc;
use tracing::{error, info};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub struct Data {
    time_started: DateTime<Utc>,
    osu_client: Arc<Osu>,
    db_pool: Pool<ConnectionManager<PgConnection>>,
    http_client: reqwest::Client,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

type Context<'a> = poise::Context<'a, Data, Error>;

type PartialContext<'a> = poise::PartialContext<'a, Data, Error>;

async fn event_listener(
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    user_data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready {
            ctx,
            data_about_bot,
        } => {
            info!("{} is connected!", data_about_bot.user.name);

            let mut osu_tracker = OsuTracker {
                ctx: ctx.clone(),
                osu_client: user_data.osu_client.clone(),
                pool: user_data.db_pool.clone(),
                shut_down: false,
            };

            tokio::spawn(async move {
                osu_tracker.tracking_loop().await?;

                Ok::<(), Error>(())
            });
        }
        FullEvent::VoiceStateUpdate {
            ctx,
            old,
            new: _new,
        } => {
            let Some(voice) = old else { return Ok(()) };
            plugins::music::check_for_empty_channel(ctx, voice.guild_id).await?;
        }
        _ => {}
    }

    Ok(())
}

async fn pre_command(ctx: Context<'_>) {
    info!(
        "@{} ({}) -> {}",
        ctx.author().name,
        match ctx.guild() {
            Some(guild) => guild.name.clone(),
            _ => {
                "Direct Message".into()
            }
        },
        ctx.invocation_string()
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
            if let Err(why) = ctx
                .say("The command returned an error. Try again later.")
                .await
            {
                error!("Error while handling error: {}", why);
            }
        }
        poise::FrameworkError::Listener { error, event, .. } => {
            error!(
                "Listener returned error during {:?} event: {:?}",
                event.snake_case_name(),
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

    let connection = utils::db::establish_connection::establish_connection();

    let mut migration_connection = match connection.get() {
        Ok(connection) => connection,
        Err(why) => {
            panic!("Couldn't get db connection: {why:?}");
        }
    };

    if let Err(why) = migration_connection.run_pending_migrations(MIGRATIONS) {
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
            plugins::osu::score(),
            plugins::osu::scores(),
            plugins::osu::firsts(),
            plugins::osu::pins(),
            // This function registers slash commands on Discord. When you change something about a
            // command signature, for example by changing its name, adding or removing parameters, or
            // changing a parameter type, you should call this function.
            plugins::basic::register(),
        ],
        listener: |event, framework, user_data| {
            Box::pin(event_listener(event, framework, user_data))
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
            dynamic_prefix: Some(|c| Box::pin(utils::db::prefix::get_guild_prefix(c))),
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

    let framework = poise::Framework::new(options, move |_ctx, _data_about_bot, _framework| {
        Box::pin(async move {
            Ok(Data {
                time_started: Utc::now(),
                osu_client,
                db_pool: connection,
                http_client: reqwest::Client::new(),
            })
        })
    });

    let mut client = serenity::Client::builder(token, intents)
        .framework(framework)
        .register_songbird()
        .await
        .unwrap();

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start_autosharded().await {
        error!("Client error: {:?}", why);
    }
}
