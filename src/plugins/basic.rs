use crate::{Context, Error};
use std::borrow::Cow;

use aformat::{CapStr, aformat};
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::small_fixed_array::FixedString;
use poise::serenity_prelude::{Colour, CreateEmbed, User};
use poise::{Command, ContextMenuCommandAction, CreateReply, PartialContext};
use rand::Rng;
use std::fmt::Write;
use std::time::Instant;
use std::writeln;
use to_arraystring::ToArrayString;

/// Optional configuration for how the help message from [`help()`] looks
#[derive(Default)]
pub struct HelpConfiguration<'a> {
    /// Extra text displayed at the bottom of your message. Can be used for help and tips specific
    /// to your bot
    pub extra_text_at_bottom: &'a str,
    /// Whether to list context menu commands as well
    pub show_context_menu_commands: bool,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

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

async fn help_single_command<U: Send + Sync + 'static, E>(
    ctx: poise::Context<'_, U, E>,
    command_name: &str,
) -> Result<(), Error> {
    let command = ctx.framework().options().commands.iter().find(|command| {
        if command.name.eq_ignore_ascii_case(command_name) {
            return true;
        }
        if let Some(context_menu_name) = &command.context_menu_name {
            if context_menu_name.eq_ignore_ascii_case(command_name) {
                return true;
            }
        }

        false
    });

    let reply = if let Some(command) = command {
        match &command.help_text {
            Some(f) => f,
            None => command
                .description
                .as_deref()
                .unwrap_or("No help available"),
        }
    } else {
        "Command not found."
    };

    ctx.say(reply).await?;
    Ok(())
}

fn format_args<U, E>(arguments: &[poise::CommandParameter<U, E>]) -> String {
    arguments.iter().fold(String::new(), |acc, arg| {
        acc + &*if arg.required {
            format!("{} ", arg.name)
        } else {
            format!("<{}> ", arg.name)
        }
    })
}

async fn format_command<U: Send + Sync + 'static, E>(
    ctx: poise::Context<'_, U, E>,
    command: &Command<U, E>,
    parent: Option<Cow<'_, str>>,
) -> Result<String, Error> {
    let prefix = if command.prefix_action.is_some() {
        let options = &ctx.framework().options().prefix_options;

        match &options.prefix {
            Some(fixed_prefix) => fixed_prefix.clone(),
            None => match options.dynamic_prefix {
                Some(dynamic_prefix_callback) => {
                    match dynamic_prefix_callback(PartialContext::from(ctx)).await {
                        Ok(Some(dynamic_prefix)) => dynamic_prefix,
                        Err(_) | Ok(None) => Cow::Borrowed(""),
                    }
                }
                None => Cow::Borrowed(""),
            },
        }
    } else if command.slash_action.is_some() {
        Cow::Borrowed("/")
    } else {
        // This is not a prefix or slash command, i.e. probably a context menu only command
        // which we will only show later
        return Ok(String::new());
    };
    Ok(aformat!(
        "  {}{} {}",
        CapStr::<4>(&prefix),
        if parent.is_some() {
            aformat!(
                "{} {}",
                CapStr::<16>(&parent.ok_or("Failed to unwrap parent in format_command")?),
                CapStr::<24>(&command.name)
            )
        } else {
            CapStr::<41>(&command.name).to_arraystring()
        },
        CapStr::<82>(&format_args(&command.parameters))
    )
    .to_string())
}

async fn help_all_commands<U: Send + Sync + 'static, E>(
    ctx: poise::Context<'_, U, E>,
    config: HelpConfiguration<'_>,
) -> Result<(), Error> {
    let mut categories = OrderedMap::<&Option<Cow<'_, str>>, Vec<&Command<U, E>>>::new();
    for cmd in &ctx.framework().options().commands {
        categories
            .get_or_insert_with(&cmd.category, Vec::new)
            .push(cmd);
    }

    let mut menu = String::from("```\n");
    for (category_name, commands) in categories {
        menu += category_name.as_ref().unwrap_or(&Cow::from("Commands"));
        menu += ":\n";
        for command in commands {
            if command.hide_in_help {
                continue;
            }
            writeln!(menu, "{}", format_command(ctx, command, None).await?)?;
            if !command.subcommands.is_empty() {
                for subcommand in &command.subcommands {
                    if subcommand.hide_in_help {
                        continue;
                    }
                    writeln!(
                        menu,
                        "{}",
                        format_command(ctx, subcommand, Option::from(command.name.clone())).await?
                    )?;
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
                _ => continue,
            };
            let name = command
                .context_menu_name
                .clone()
                .unwrap_or(command.name.clone());
            writeln!(menu, "  {name} (on {kind})")?;
        }
    }

    menu += "\n```";
    menu += config.extra_text_at_bottom;

    let color = match ctx.author_member().await {
        Some(member) => member.colour(ctx.cache()).unwrap_or(BLUE),
        _ => BLUE,
    };

    let embed = CreateEmbed::new()
        .title("Commands")
        .description(menu)
        .colour(color);

    let builder = CreateReply::default().embed(embed);

    ctx.send(builder).await?;
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
    let information = ctx.http().get_current_application_info().await?;
    let owner = information.owner.ok_or("No application owner registered")?;
    let username = if let Some(discriminator) = owner.discriminator {
        aformat!(
            "@{}#{}",
            CapStr::<16>(&owner.name),
            discriminator.to_arraystring()
        )
    } else {
        CapStr::<23>(&owner.name).to_arraystring()
    };

    let content = aformat!(
        "```elm\n\
        Owner   : {}\n\
        Up      : {} UTC\n\
        Guilds  : {}```\
        {}",
        username,
        CapStr::<16>(&ctx.data().time_started.to_rfc2822()),
        ctx.cache().guilds().len().to_arraystring(),
        CapStr::<64>(&information.description)
    );

    let color = match ctx.guild() {
        Some(guild) => match guild.members.get(&ctx.framework().bot_id()) {
            Some(member) => member.colour(ctx.cache()).unwrap_or(BLUE),
            _ => BLUE,
        },
        _ => BLUE,
    };

    let embed = CreateEmbed::new()
        .title(information.name)
        .description(content.as_str())
        .colour(color);

    let builder = CreateReply::default().embed(embed);

    ctx.send(builder).await?;

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
        extra_text_at_bottom: &format!(
            "Type {}help command for more info on a command. <> around arguments mean they are optional.",
            ctx.prefix()
        ),
        ..Default::default()
    };
    match command.as_deref() {
        Some(command) => help_single_command(ctx, command).await?,
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
    let connection = &mut ctx.data().db_pool.get().await?;
    crate::utils::db::prefix::add_guild_prefix(
        connection,
        i64::try_from(
            ctx.guild_id()
                .ok_or("Failed to get guild ID in prefix")?
                .get(),
        )?,
        new_prefix.clone(),
    )
    .await?;

    ctx.say(aformat!("Set guild prefix to {}", CapStr::<4>(&new_prefix)).as_str())
        .await?;

    Ok(())
}

/// Rolls a number between 1-100, or a specified range
#[poise::command(prefix_command, slash_command, category = "Basic")]
pub async fn roll(
    ctx: Context<'_>,
    #[description = "Range of numbers to roll"] range: Option<u128>,
) -> Result<(), Error> {
    let random_number = rand::rng().random_range(0..range.unwrap_or(100));

    ctx.say(random_number.to_string()).await?;

    Ok(())
}

#[poise::command(prefix_command, category = "Basic", hide_in_help = true, owners_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Shutting down the bot.").await?;

    ctx.serenity_context().shutdown_all();

    Ok(())
}

/// Pings the bot
#[poise::command(prefix_command, slash_command, category = "Basic")]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let start = Instant::now();
    let message = ctx.say("Pong!").await?;
    let duration = start.elapsed();
    message
        .edit(
            ctx,
            CreateReply::default()
                .content(aformat!("Pong! `{}ms`", duration.as_millis().to_arraystring()).as_str()),
        )
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
    let color: Colour;
    let name: FixedString<u8>;
    let avatar: String;

    if let Some(guild) = ctx.guild()
        && let Some(member) = guild
            .members
            .get(&user.as_ref().unwrap_or_else(|| ctx.author()).id)
    {
        {
            color = member.colour(ctx.cache()).unwrap_or(BLUE);
            name = member.nick.as_ref().unwrap_or(&member.user.name).clone();
            avatar = member.face();
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
    }

    let embed = CreateEmbed::new().title(name).image(avatar).color(color);

    let builder = CreateReply::default().embed(embed);

    ctx.send(builder).await?;

    Ok(())
}
