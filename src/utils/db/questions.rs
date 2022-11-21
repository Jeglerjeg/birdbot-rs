use crate::models::questions::{NewQuestion, Question};
use crate::schema::questions;
use crate::Error;
use diesel::dsl::{count, sql};
use diesel::prelude::*;

pub fn count_entries(db: &mut PgConnection) -> Result<i64, Error> {
    Ok(questions::table
        .select(count(questions::id))
        .get_result(db)?)
}

pub fn update_question_answers(
    db: &mut PgConnection,
    id: i32,
    choice_1_answers: i32,
    choice_2_answers: i32,
) -> Result<(), Error> {
    diesel::update(questions::table.find(id))
        .set((
            questions::choice2_answers.eq(choice_1_answers),
            questions::choice2_answers.eq(choice_2_answers),
        ))
        .execute(db)?;

    Ok(())
}

pub fn add_question(db: &mut PgConnection, choice_1: &str, choice_2: &str) -> Result<(), Error> {
    let new_question = NewQuestion {
        choice1: choice_1,
        choice2: choice_2,
    };

    diesel::insert_into(questions::table)
        .values(&new_question)
        .execute(db)?;

    Ok(())
}

pub fn get_question(
    db: &mut PgConnection,
    choice_1: &str,
    choice_2: &str,
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
