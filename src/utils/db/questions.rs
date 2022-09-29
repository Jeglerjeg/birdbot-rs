use crate::{Context, Error};
use entities::questions::{Entity as Questions, Model};
use sea_orm::{
    entity::{ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, Set},
    query::{QueryFilter, Statement},
    DbBackend,
};

pub struct Question {
    pub id: i32,
    pub choice_1: String,
    pub choice_2: String,
    pub choice_1_answers: i32,
    pub choice_2_answers: i32,
}

pub async fn count_entries(ctx: Context<'_>) -> Result<usize, Error> {
    let question: Vec<_> = Questions::find().all(&ctx.data().db).await?;

    Ok(question.len())
}

pub async fn update_choice(ctx: Context<'_>, id: i32, choice: i8) -> Result<(), Error>{
    let read_only_question: Model = Questions::find_by_id(id).one(&ctx.data().db).await?.unwrap();
    let mut question: entities::questions::ActiveModel = read_only_question.clone().into();

    match choice {
        1 => {
            question.choice1_answers = Set(read_only_question.choice1_answers + 1);
        },
        2 => {
            question.choice2_answers = Set(read_only_question.choice2_answers + 1);
        },
        _ => {
            return Ok(());
        }
    };
    
    question.update(&ctx.data().db).await?;

    Ok(())
}

pub async fn add_question(
    ctx: Context<'_>,
    choice_1: String,
    choice_2: String,
) -> Result<(), Error> {
    let table = entities::questions::ActiveModel {
        id: NotSet,
        choice1: Set(choice_1),
        choice2: Set(choice_2),
        choice1_answers: NotSet,
        choice2_answers: NotSet,
    };

    Questions::insert(table).exec(&ctx.data().db).await?;

    Ok(())
}

pub async fn get_question(
    ctx: Context<'_>,
    choice_1: String,
    choice_2: String,
) -> Result<Option<Question>, Error> {
    let question: Option<Model> = Questions::find()
        .filter(entities::questions::Column::Choice1.eq(choice_1))
        .filter(entities::questions::Column::Choice2.eq(choice_2))
        .one(&ctx.data().db)
        .await?;

    match question {
        Some(question) => Ok(Some(Question {
            id: question.id,
            choice_1: question.choice1,
            choice_2: question.choice2,
            choice_1_answers: question.choice1_answers,
            choice_2_answers: question.choice2_answers,
        })),
        _ => Ok(None),
    }
}

pub async fn get_random_question(ctx: Context<'_>) -> Result<Option<Question>, Error> {
    let question: Option<Model> = Questions::find().from_raw_sql(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"SELECT * FROM questions WHERE id IN (SELECT id FROM questions ORDER BY RANDOM() LIMIT 1)"#,
        vec![1.into()],
    )).one(&ctx.data().db).await?;

    match question {
        Some(question) => Ok(Some(Question {
            id: question.id,
            choice_1: question.choice1,
            choice_2: question.choice2,
            choice_1_answers: question.choice1_answers,
            choice_2_answers: question.choice2_answers,
        })),
        _ => Ok(None),
    }
}
