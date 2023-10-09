use crate::models::questions::Question;
use crate::{Context, Error};
use dashmap::mapref::one::RefMut;
use dashmap::DashMap;
use diesel_async::AsyncPgConnection;
use lazy_static::lazy_static;
use poise::futures_util::StreamExt;
use poise::serenity_prelude::ButtonStyle::{Danger, Success};
use poise::serenity_prelude::{
    CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, Mentionable, User,
};
use poise::{CreateReply, ReplyHandle};
use rand::seq::SliceRandom;
use std::time::Duration;

pub struct PreviousServerQuestions {
    pub recent_questions: DashMap<u64, Vec<i32>>,
}

lazy_static! {
    static ref RECENTLY_ASKED_QUESTIONS: PreviousServerQuestions = PreviousServerQuestions {
        recent_questions: DashMap::new(),
    };
}

fn format_results(question: &Question) -> String {
    format!(
        "A total of {} would **{}**, while {} would **{}**",
        question.choice1_answers, question.choice1, question.choice2_answers, question.choice2
    )
}

fn format_response(user: &User, choice: &String) -> String {
    format!("**{}** would **{}**!", user.mention(), choice)
}

fn format_question(question: &Question, responses: &[String]) -> String {
    format!(
        "Would you rather 🟢 **{}** or 🔴 **{}**?\n\n{}",
        question.choice1,
        question.choice2,
        responses.join("\n")
    )
}

async fn create_wyr_message(
    ctx: Context<'_>,
    question: Question,
    connection: &mut AsyncPgConnection,
) -> Result<(), Error> {
    let embed = CreateEmbed::new().description(format_question(&question, &[]));

    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new("choice_1").style(Success).label("1"),
        CreateButton::new("choice_2").style(Danger).label("2"),
    ])];

    let builder = CreateReply::default().embed(embed).components(components);

    let reply = ctx.send(builder).await?;

    handle_interaction_responses(ctx, reply, question, connection).await?;

    Ok(())
}

async fn handle_interaction_responses(
    ctx: Context<'_>,
    reply: ReplyHandle<'_>,
    mut question: Question,
    connection: &mut AsyncPgConnection,
) -> Result<(), Error> {
    let mut responses: Vec<String> = vec![];
    let mut replies: Vec<u64> = vec![];

    // Wait for multiple interactions
    let mut interaction_stream = reply
        .message()
        .await?
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(30))
        .stream();

    while let Some(interaction) = interaction_stream.next().await {
        if replies.contains(&interaction.user.id.get()) {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .ephemeral(true)
                            .content("You have already answered."),
                    ),
                )
                .await?;
            continue;
        };

        let choice = &interaction.data.custom_id;
        match choice.as_str() {
            "choice_1" => {
                interaction.defer(ctx).await?;
                replies.push(interaction.user.id.get());
                responses.push(format_response(&interaction.user, &question.choice1));
                question.choice1_answers += 1;

                let embed = CreateEmbed::new().description(format_question(&question, &responses));

                let builder = CreateReply::default().embed(embed);

                crate::utils::db::questions::update_question_answers(
                    connection,
                    question.id,
                    question.choice1_answers,
                    question.choice2_answers,
                )
                .await?;

                reply.edit(ctx, builder).await?;
            }
            "choice_2" => {
                interaction.defer(ctx).await?;
                replies.push(interaction.user.id.get());
                responses.push(format_response(&interaction.user, &question.choice2));
                question.choice2_answers += 1;

                let embed = CreateEmbed::new().description(format_question(&question, &responses));

                let builder = CreateReply::default().embed(embed);

                crate::utils::db::questions::update_question_answers(
                    connection,
                    question.id,
                    question.choice1_answers,
                    question.choice2_answers,
                )
                .await?;

                reply.edit(ctx, builder).await?;
            }
            _ => {
                interaction
                    .create_response(
                        ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .ephemeral(true)
                                .content("Something went wrong."),
                        ),
                    )
                    .await?;
            }
        }
    }

    let embed = CreateEmbed::new().description(format!(
        "{}\n\n{}",
        format_question(&question, &responses),
        format_results(&question)
    ));

    let builder = CreateReply::default().embed(embed).components(vec![]);

    reply.edit(ctx, builder).await?;

    Ok(())
}

async fn add_recent_question<'a>(
    connection: &mut AsyncPgConnection,
    mut previous_questions: RefMut<'_, u64, Vec<i32>>,
    id: i32,
) -> Result<(), Error> {
    previous_questions.push(id);

    let previous_len = crate::utils::db::questions::count_entries(connection).await?;
    if previous_questions.len() as i64 > (previous_len / 2) {
        previous_questions.remove(0);
    };

    Ok(())
}

async fn check_for_duplicates(
    connection: &mut AsyncPgConnection,
    choice_1: &str,
    choice_2: &str,
) -> bool {
    if (crate::utils::db::questions::get_question(connection, choice_1, choice_2).await).is_ok() {
        return false;
    };

    if (crate::utils::db::questions::get_question(connection, choice_2, choice_1).await).is_ok() {
        return false;
    };

    true
}

///Ask the bot a would you rather question, or have the bot ask you!
#[poise::command(prefix_command, slash_command, category = "Would You Rather")]
pub async fn wyr(
    ctx: Context<'_>,
    #[rest]
    #[description = "Question to ask. Must be in in format: <choice_1> or <choice_2>"]
    question: Option<String>,
) -> Result<(), Error> {
    let mut choice_1: Option<String> = None;
    let mut choice_2: Option<String> = None;
    let connection = &mut ctx.data().db_pool.get().await?;
    if let Some(question) = question {
        let split_question: Vec<&str> = question.split(" or ").collect();
        choice_1 = Some(String::from(split_question[0]));
        choice_2 = Some(String::from(split_question[1]));
    };

    if let (Some(choice_1), Some(choice_2)) = (choice_1, choice_2) {
        if choice_1 == choice_2 {
            ctx.say("Those options are the same.").await?;
            return Ok(());
        }

        if !check_for_duplicates(connection, &choice_1, &choice_2).await {
            ctx.say("That question already exists.").await?;
            return Ok(());
        }

        crate::utils::db::questions::add_question(connection, &choice_1, &choice_2).await?;

        let choices = vec![choice_1, choice_2];

        let choice: Vec<_> = choices
            .choose_multiple(&mut rand::thread_rng(), 1)
            .collect();

        ctx.say(format!("I would {}!", choice[0])).await?;
    } else {
        let db_question = crate::utils::db::questions::get_random_question(connection).await;
        let Ok(mut db_question) = db_question else {
            ctx.say("No questions added! Ask me one!").await?;
            return Ok(());
        };

        let id = if let Some(guild_id) = ctx.guild_id() {
            guild_id.get()
        } else {
            ctx.channel_id().get()
        };

        let recent_vec = RECENTLY_ASKED_QUESTIONS
            .recent_questions
            .entry(id)
            .or_default();
        while recent_vec.contains(&db_question.id) {
            db_question = crate::utils::db::questions::get_random_question(connection).await?;
        }
        add_recent_question(connection, recent_vec, db_question.id).await?;

        create_wyr_message(ctx, db_question, connection).await?;
    }

    Ok(())
}
