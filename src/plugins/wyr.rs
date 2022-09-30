use crate::models::questions::Question;
use crate::serenity_prelude as serenity;
use crate::{Context, Error};
use lazy_static::lazy_static;
use poise::futures_util::StreamExt;
use poise::serenity_prelude;
use rand::seq::SliceRandom;
use serenity_prelude::Mentionable;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{Mutex, MutexGuard};

pub struct PreviousServerQuestions {
    pub previous_questions: Mutex<HashMap<serenity::GuildId, Mutex<Vec<i32>>>>,
}

lazy_static! {
    static ref PREVIOUS_SERVER_QUESTIONS: Mutex<PreviousServerQuestions> =
        Mutex::from(PreviousServerQuestions {
            previous_questions: Mutex::from(HashMap::new()),
        });
}

fn format_results(question: &Question) -> String {
    format!(
        "A total of {} would **{}**, while {} would **{}**",
        question.choice1_answers, question.choice1, question.choice2_answers, question.choice2
    )
}

fn format_response(user: &serenity::User, choice: &String) -> String {
    format!("**{}** would **{}**!", user.mention(), choice)
}

fn format_question(question: &Question, responses: &[String]) -> String {
    format!(
        "Would you rather **{}** or **{}**?\n\n{}",
        question.choice1,
        question.choice2,
        responses.join("\n")
    )
}

async fn create_wyr_message(ctx: Context<'_>, mut question: Question) -> Result<(), Error> {
    let mut responses: Vec<String> = vec![];

    let reply = ctx
        .send(|m| {
            m.embed(|e| e.description(format_question(&question, &responses)))
                .components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|b| {
                            b.custom_id("choice_1")
                                .label("1")
                                .style(serenity::ButtonStyle::Success)
                        })
                        .create_button(|b| {
                            b.custom_id("choice_2")
                                .label("2")
                                .style(serenity::ButtonStyle::Danger)
                        })
                    })
                })
        })
        .await?;

    // Wait for multiple interactions
    let mut interaction_stream = reply
        .message()
        .await?
        .await_component_interactions(ctx.discord())
        .timeout(Duration::from_secs(30))
        .build();

    let mut replies: Vec<u64> = vec![];
    while let Some(interaction) = interaction_stream.next().await {
        if replies.contains(&interaction.user.id.0) {
            interaction
                .create_interaction_response(ctx.discord(), |r| {
                    // This time we dont edit the message but reply to it
                    r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|d| {
                            // Make the message hidden for other users by setting `ephemeral(true)`.
                            d.ephemeral(true).content("You have already answered.")
                        })
                })
                .await
                .unwrap();
            continue;
        };

        let choice = &interaction.data.custom_id;
        match &**choice {
            "choice_1" => {
                interaction.defer(ctx.discord()).await?;
                replies.push(interaction.user.id.0);
                responses.push(format_response(&interaction.user, &question.choice1));
                question.choice1_answers += 1;
                crate::utils::db::questions::update_choice(question.id, 1);

                interaction
                    .message
                    .clone()
                    .edit(ctx.discord(), |b| {
                        b.embed(|e| e.description(format_question(&question, &responses)))
                    })
                    .await?;
            }
            "choice_2" => {
                interaction.defer(ctx.discord()).await?;
                replies.push(interaction.user.id.0);
                responses.push(format_response(&interaction.user, &question.choice2));
                question.choice2_answers += 1;
                crate::utils::db::questions::update_choice(question.id, 2);

                interaction
                    .message
                    .clone()
                    .edit(ctx.discord(), |b| {
                        b.embed(|e| e.description(format_question(&question, &responses)))
                    })
                    .await?;
            }
            _ => {
                interaction
                    .create_interaction_response(ctx.discord(), |r| {
                        // This time we dont edit the message but reply to it
                        r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|d| {
                                // Make the message hidden for other users by setting `ephemeral(true)`.
                                d.ephemeral(true).content("Something went wrong.")
                            })
                    })
                    .await
                    .unwrap();
            }
        }
    }

    reply
        .into_message()
        .await?
        .edit(ctx.discord(), |b| {
            b.components(|b| b);
            b.embed(|e| {
                e.description(format!(
                    "{}\n\n{}",
                    format_question(&question, &responses),
                    format_results(&question)
                ))
            })
        })
        .await?;

    Ok(())
}

async fn add_recent_question(lock: &mut MutexGuard<'_, Vec<i32>>, id: i32) {
    lock.push(id);

    let previous_len = crate::utils::db::questions::count_entries();
    if lock.len() > (previous_len / 2) as usize {
        lock.remove(0);
    }
}

fn check_for_duplicates(choice_1: String, choice_2: String) -> bool {
    if (crate::utils::db::questions::get_question(choice_1.clone(), choice_2.clone())).is_some() {
        return false;
    };

    if (crate::utils::db::questions::get_question(choice_2, choice_1)).is_some() {
        return false;
    };

    true
}

#[poise::command(prefix_command, slash_command, category = "Would You Rather")]
pub async fn wyr(
    ctx: Context<'_>,
    #[description = "Question to ask. Must be in in format: <choice_1> or <choice_2>"]
    question: Option<String>,
) -> Result<(), Error> {
    let mut choice_1: Option<String> = None;
    let mut choice_2: Option<String> = None;
    if let Some(question) = question {
        let split_question: Vec<&str> = question.split(" or ").collect();
        choice_1 = Some(String::from(split_question[0]));
        choice_2 = Some(String::from(split_question[1]));
    };

    if let (Some(choice_1), Some(choice_2)) = (choice_1, choice_2) {
        if !check_for_duplicates(choice_1.clone(), choice_2.clone()) {
            ctx.say("That question already exists").await?;
            return Ok(());
        }

        crate::utils::db::questions::add_question(&*choice_1, &*choice_2);

        let choices = vec![choice_1, choice_2];

        let choice: Vec<_> = choices
            .choose_multiple(&mut rand::thread_rng(), 1)
            .collect();

        ctx.say(format!("I would {}!", choice[0])).await?;
    } else {
        let db_question = crate::utils::db::questions::get_random_question();

        let mut db_question = match db_question {
            Some(db_question) => db_question,
            _ => {
                ctx.say("No questions added! Ask me one!").await?;
                return Ok(());
            }
        };

        let previous_questions_lock = PREVIOUS_SERVER_QUESTIONS.lock().await;
        let mut previous_hash_lock = previous_questions_lock.previous_questions.lock().await;

        if let std::collections::hash_map::Entry::Vacant(e) =
            previous_hash_lock.entry(ctx.guild_id().unwrap())
        {
            e.insert(Mutex::from(vec![db_question.id]));
        } else {
            let mut previous_vec = previous_hash_lock
                .get(&ctx.guild_id().unwrap())
                .unwrap()
                .lock()
                .await;

            while previous_vec.contains(&db_question.id) {
                db_question = crate::utils::db::questions::get_random_question().unwrap();
            }
            add_recent_question(&mut previous_vec, db_question.id).await;
            drop(previous_vec);
        }
        drop(previous_hash_lock);
        drop(previous_questions_lock);

        create_wyr_message(ctx, db_question).await?;
    }

    Ok(())
}