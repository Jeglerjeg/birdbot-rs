use crate::models::questions::{NewQuestion, Question};
use crate::schema::questions;
use diesel::dsl::sql;
use diesel::prelude::*;

pub fn count_entries() -> i64 {
    let connection = &mut crate::utils::db::establish_connection::establish_connection();

    questions::table
        .count()
        .get_result(connection)
        .expect("Failed to count questions.")
}

pub fn update_choice(id: i32, choice: i8) {
    let connection = &mut crate::utils::db::establish_connection::establish_connection();

    let question: Question = questions::table
        .find(id)
        .first(connection)
        .expect("Failed to get question.");
    match choice {
        1 => {
            let new_count = &question.choice1_answers + 1;
            diesel::update(questions::table.find(id))
                .set(questions::choice1_answers.eq(new_count))
                .execute(connection)
                .expect("Failed to update question choice")
        }
        2 => {
            let new_count = &question.choice2_answers + 1;
            diesel::update(questions::table.find(id))
                .set(questions::choice2_answers.eq(new_count))
                .execute(connection)
                .expect("Failed to update question choice")
        }
        _ => {
            return;
        }
    };
}

pub fn add_question(choice_1: &str, choice_2: &str) {
    let connection = &mut crate::utils::db::establish_connection::establish_connection();

    let new_question = NewQuestion {
        choice1: choice_1,
        choice2: choice_2,
    };

    diesel::insert_into(questions::table)
        .values(&new_question)
        .execute(connection)
        .expect("Failed to insert prefix");
}

pub fn get_question(choice_1: String, choice_2: String) -> Option<Question> {
    let connection = &mut crate::utils::db::establish_connection::establish_connection();

    let question = questions::table
        .filter(questions::choice1.eq(choice_1))
        .filter(questions::choice2.eq(choice_2))
        .first(connection);

    match question {
        Ok(question) => Some(question),
        Err(_) => None,
    }
}

pub fn get_random_question() -> Option<Question> {
    let connection = &mut crate::utils::db::establish_connection::establish_connection();

    let question = questions::table
        .order(sql::<diesel::sql_types::Integer>("RANDOM()"))
        .first::<Question>(connection);

    match question {
        Ok(question) => Some(question),
        Err(_) => None,
    }
}
