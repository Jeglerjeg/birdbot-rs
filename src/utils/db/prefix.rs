use crate::{Context, Error, PartialContext};
use entities::prefix::{Entity as Prefix, Model};
use sea_orm::{entity::*, sea_query};

pub async fn add_guild_prefix(
    ctx: Context<'_>,
    guild_id: i64,
    prefix: String,
) -> Result<(), Error> {
    let table = entities::prefix::ActiveModel {
        guild_id: Set(guild_id.to_owned()),
        prefix: Set(prefix),
    };

    Prefix::insert(table)
        .on_conflict(
            sea_query::OnConflict::column(entities::prefix::Column::GuildId)
                .update_column(entities::prefix::Column::Prefix)
                .to_owned(),
        )
        .exec(&ctx.data().db)
        .await?;

    Ok(())
}

pub async fn get_guild_prefix(
    ctx: PartialContext<'_>,
    default_prefix: String,
) -> Result<Option<String>, Error> {
    let guild_id = match ctx.guild_id {
        Some(guild) => guild.0 as i64,
        _ => return Ok(Some(default_prefix)),
    };
    let db_prefix: Option<Model> = Prefix::find_by_id(guild_id)
        .one(&ctx.data.db)
        .await
        .expect("");
    let prefix = match db_prefix {
        Some(model) => model.prefix,
        _ => default_prefix,
    };
    Ok(Some(prefix))
}
