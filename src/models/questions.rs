use crate::schema::questions;
use diesel::prelude::*;

#[derive(Queryable)]
#[diesel(table_name = questions)]
pub struct Question {
    pub id: i32,
    pub choice1: String,
    pub choice2: String,
    pub choice1_answers: i32,
    pub choice2_answers: i32,
}

#[derive(Insertable)]
#[diesel(table_name = questions)]
pub struct NewQuestion<'a> {
    pub choice1: &'a str,
    pub choice2: &'a str,
}
