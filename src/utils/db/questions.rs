use crate::models::questions::{NewQuestion, Question};
use crate::schema::questions;
use diesel::dsl::{count, sql};
use diesel::prelude::*;

pub fn count_entries(db: &mut PgConnection) -> i64 {
    questions::table
        .select(count(questions::id))
        .get_result(db)
        .expect("Failed to count questions.")
}

pub fn update_choice(db: &mut PgConnection, id: i32, choice: i8) {
    let question: Question = questions::table
        .find(id)
        .first(db)
        .expect("Failed to get question.");
    match choice {
        1 => {
            let new_count = &question.choice1_answers + 1;
            diesel::update(questions::table.find(id))
                .set(questions::choice1_answers.eq(new_count))
                .execute(db)
                .expect("Failed to update question choice");
        }
        2 => {
            let new_count = &question.choice2_answers + 1;
            diesel::update(questions::table.find(id))
                .set(questions::choice2_answers.eq(new_count))
                .execute(db)
                .expect("Failed to update question choice");
        }
        _ => {}
    };
}

pub fn add_question(db: &mut PgConnection, choice_1: &str, choice_2: &str) {
    let new_question = NewQuestion {
        choice1: choice_1,
        choice2: choice_2,
    };

    diesel::insert_into(questions::table)
        .values(&new_question)
        .execute(db)
        .expect("Failed to insert prefix");
}

pub fn get_question(
    db: &mut PgConnection,
    choice_1: String,
    choice_2: String,
) -> QueryResult<Question> {
    questions::table
        .filter(questions::choice1.eq(choice_1))
        .filter(questions::choice2.eq(choice_2))
        .first(db)
}

pub fn get_random_question(db: &mut PgConnection) -> QueryResult<Question> {
    questions::table
        .order(sql::<diesel::sql_types::Integer>("RANDOM()"))
        .first::<Question>(db)
}
