use crate::{Context, Error};
use poise::builtins::HelpConfiguration;

use poise::{serenity_prelude, Command, ContextMenuCommandAction, PartialContext};
use rand::Rng;
use serenity::utils::colours::roles::BLUE;
use serenity_prelude::User;

use serenity::utils::Color;
use std::fmt::Write;
use std::time::Instant;
use std::writeln;

pub struct OrderedMap<K, V>(pub Vec<(K, V)>);

impl<K, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self(Vec::default())
    }
}

impl<K: Eq, V> OrderedMap<K, V> {
    /// Creates a new [`OrderedMap`]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Finds a value in the map by the given key, or inserts it if it doesn't exist
    pub fn get_or_insert_with(&mut self, k: K, v: impl FnOnce() -> V) -> &mut V {
        if let Some(i) = self.0.iter().position(|entry| entry.0 == k) {
            &mut self.0[i].1
        } else {
            self.0.push((k, v()));
            &mut self.0.last_mut().expect("we just inserted").1
        }
    }
}

impl<K, V> IntoIterator for OrderedMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

async fn help_single_command<U, E>(
    ctx: poise::Context<'_, U, E>,
    command_name: &str,
    config: HelpConfiguration<'_>,
) -> Result<(), Error> {
    let command = ctx.framework().options().commands.iter().find(|command| {
        if command.name.eq_ignore_ascii_case(command_name) {
            return true;
        }
        if let Some(context_menu_name) = command.context_menu_name {
            if context_menu_name.eq_ignore_ascii_case(command_name) {
                return true;
            }
        }

        false
    });

    let reply = if let Some(command) = command {
        match command.help_text {
            Some(f) => f(),
            None => command
                .description
                .as_deref()
                .unwrap_or("No help available")
                .to_owned(),
        }
    } else {
        format!("No such command `{}`", command_name)
    };

    ctx.send(|b| b.content(reply).ephemeral(config.ephemeral))
        .await?;
    Ok(())
}

fn format_args<U, E>(arguments: &[poise::CommandParameter<U, E>]) -> String {
    return arguments.iter().fold(String::new(), |acc, arg| {
        acc + &*if arg.required {
            arg.name.clone()
        } else {
            format!("<{}>", arg.name)
        }
    });
}

async fn format_command<U, E>(
    ctx: poise::Context<'_, U, E>,
    command: &Command<U, E>,
    parent: Option<String>,
) -> String {
    let prefix = if command.prefix_action.is_some() {
        let options = &ctx.framework().options().prefix_options;

        match &options.prefix {
            Some(fixed_prefix) => fixed_prefix.clone(),
            None => match options.dynamic_prefix {
                Some(dynamic_prefix_callback) => {
                    match dynamic_prefix_callback(PartialContext::from(ctx)).await {
                        Ok(Some(dynamic_prefix)) => dynamic_prefix,
                        Err(_) | Ok(None) => String::from(""),
                    }
                }
                None => String::from(""),
            },
        }
    } else if command.slash_action.is_some() {
        String::from("/")
    } else {
        // This is not a prefix or slash command, i.e. probably a context menu only command
        // which we will only show later
        return String::from("");
    };
    format!(
        "  {}{} {}",
        prefix,
        if parent.is_some() {
            format!("{} {}", parent.unwrap(), &command.name)
        } else {
            command.name.clone()
        },
        format_args(&command.parameters)
    )
}

async fn help_all_commands<U, E>(
    ctx: poise::Context<'_, U, E>,
    config: HelpConfiguration<'_>,
) -> Result<(), serenity::Error> {
    let mut categories = OrderedMap::<Option<&str>, Vec<&Command<U, E>>>::new();
    for cmd in &ctx.framework().options().commands {
        categories
            .get_or_insert_with(cmd.category, Vec::new)
            .push(cmd);
    }

    let mut menu = String::from("```\n");
    for (category_name, commands) in categories {
        menu += category_name.unwrap_or("Commands");
        menu += ":\n";
        for command in commands {
            if command.hide_in_help {
                continue;
            }
            let _ = writeln!(menu, "{}", format_command(ctx, command, None).await);
            if !command.subcommands.is_empty() {
                for subcommand in &command.subcommands {
                    if subcommand.hide_in_help {
                        continue;
                    }
                    let _ = writeln!(
                        menu,
                        "{}",
                        format_command(ctx, subcommand, Option::from(command.name.clone())).await
                    );
                }
            }
        }
    }

    if config.show_context_menu_commands {
        menu += "\nContext menu commands:\n";

        for command in &ctx.framework().options().commands {
            let kind = match command.context_menu_action {
                Some(ContextMenuCommandAction::User(_)) => "user",
                Some(ContextMenuCommandAction::Message(_)) => "message",
                None => continue,
            };
            let name = command.context_menu_name.unwrap_or(&command.name);
            let _ = writeln!(menu, "  {} (on {})", name, kind);
        }
    }

    menu += "\n```";
    menu += config.extra_text_at_bottom;

    let color = match ctx.author_member().await {
        Some(member) => member.colour(ctx.discord()).unwrap_or(BLUE),
        _ => BLUE,
    };

    ctx.send(|b| b.embed(|e| e.title("Commands:").description(menu).colour(color)))
        .await?;
    Ok(())
}

/// Registers slash commands in this guild or globally
#[poise::command(prefix_command, hide_in_help, category = "Basic", owners_only)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;

    Ok(())
}

/// Displays basic information about the bot
#[poise::command(prefix_command, slash_command, category = "Basic")]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    let information = ctx
        .discord()
        .http
        .get_current_application_info()
        .await
        .unwrap();
    let content = format!(
        "```elm\n\
        Owner   : {}#{}\n\
        Up      : {} UTC\n\
        Members : {}\n\
        Guilds  : {}```\
        {}",
        information.owner.name,
        information.owner.discriminator,
        ctx.data().time_started.format("%d-%m-%Y %H:%M:%S"),
        ctx.discord().cache.users().len(),
        ctx.discord().cache.guilds().len(),
        information.description
    );

    let color = match ctx.guild() {
        Some(guild) => match guild.member(ctx.discord(), ctx.framework().bot_id).await {
            Ok(member) => member.colour(ctx.discord()).unwrap_or(BLUE),
            Err(_error) => BLUE,
        },
        _ => BLUE,
    };

    ctx.send(|b| b.embed(|e| e.title(information.name).description(content).colour(color)))
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, hide_in_help = true, category = "Basic")]
pub async fn help(
    ctx: Context<'_>,
    #[rest]
    #[description = "Specific command to show help about"]
    command: Option<String>,
) -> Result<(), Error> {
    let config = HelpConfiguration {
        extra_text_at_bottom: &*format!(
            "Type {}help command for more info on a command. <> around arguments mean they are optional.",
            ctx.prefix()
        ),
        ..Default::default()
    };
    match command.as_deref() {
        Some(command) => help_single_command(ctx, command, config).await?,
        None => help_all_commands(ctx, config).await?,
    }
    Ok(())
}

/// Sets the guild prefix
#[poise::command(prefix_command, slash_command, category = "Basic", guild_only = true)]
pub async fn prefix(
    ctx: Context<'_>,
    #[description = "Prefix to use in guild"] new_prefix: String,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get()?;
    crate::utils::db::prefix::add_guild_prefix(
        connection,
        ctx.guild_id().unwrap().0 as i64,
        &*new_prefix,
    );

    ctx.say(format!("Set guild prefix to {}", new_prefix))
        .await?;

    Ok(())
}

/// Rolls a number between 1-100, or a specified range
#[poise::command(prefix_command, slash_command, category = "Basic")]
pub async fn roll(
    ctx: Context<'_>,
    #[description = "Range of numbers to roll"] range: Option<u128>,
) -> Result<(), Error> {
    let random_number = rand::thread_rng().gen_range(0..range.unwrap_or(100));

    ctx.say(random_number.to_string()).await?;

    Ok(())
}

#[poise::command(prefix_command, category = "Basic", hide_in_help = true, owners_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Shutting down the bot.").await?;

    ctx.framework()
        .shard_manager
        .lock()
        .await
        .shutdown_all()
        .await;

    Ok(())
}

/// Pings the bot
#[poise::command(prefix_command, slash_command, category = "Basic")]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let start = Instant::now();
    let message = ctx.say("Pong!").await?;
    let duration = start.elapsed();
    message
        .edit(ctx, |f| {
            f.content(format!("Pong! `{}ms`", duration.as_millis()))
        })
        .await?;
    Ok(())
}

/// Displays your or a specified member's avatar
#[poise::command(prefix_command, slash_command, category = "Basic")]
pub async fn avatar(
    ctx: Context<'_>,
    #[rest]
    #[description = "User to get avatar for"]
    user: Option<User>,
) -> Result<(), Error> {
    let color: Color;
    let name: String;
    let avatar: String;

    if let Some(guild) = ctx.guild() {
        if let Ok(member) = guild
            .member(
                ctx.discord(),
                user.as_ref().unwrap_or_else(|| ctx.author()).id,
            )
            .await
        {
            color = member.colour(ctx.discord()).unwrap_or(BLUE);
            name = member.nick.as_ref().unwrap_or(&member.user.name).clone();
            avatar = member.face();
        } else {
            color = BLUE;
            if let Some(user) = user {
                name = user.name.clone();
                avatar = user.face();
            } else {
                name = ctx.author().name.clone();
                avatar = ctx.author().face();
            }
        }
    } else {
        color = BLUE;
        if let Some(user) = user {
            name = user.name.clone();
            avatar = user.face();
        } else {
            name = ctx.author().name.clone();
            avatar = ctx.author().face();
        }
    };

    ctx.send(|m| m.embed(|e| e.title(name).image(avatar).color(color)))
        .await?;

    Ok(())
}
