use crate::models::prefix::{NewPrefix, Prefix};
use crate::schema::prefix;
use crate::{Error, PartialContext};
use diesel::prelude::*;

pub fn add_guild_prefix(guild_id: i64, prefix: &str) {
    let connection = &mut crate::utils::db::establish_connection::establish_connection();
    let new_prefix = NewPrefix {
        guild_id: &guild_id,
        guild_prefix: prefix,
    };

    diesel::replace_into(prefix::table)
        .values(&new_prefix)
        .execute(connection)
        .expect("Failed to insert prefix");
}

pub async fn get_guild_prefix(
    ctx: PartialContext<'_>,
    default_prefix: String,
) -> Result<Option<String>, Error> {
    let guild_id = match ctx.guild_id {
        Some(guild) => guild.0 as i64,
        _ => return Ok(Some(default_prefix)),
    };

    let connection = &mut crate::utils::db::establish_connection::establish_connection();
    let db_prefix = prefix::table
        .find(guild_id)
        .limit(1)
        .load::<Prefix>(connection)
        .expect("Error loading guild prefix");

    if db_prefix.is_empty() {
        Ok(Some(default_prefix))
    } else {
        Ok(Some(db_prefix[0].guild_prefix.clone()))
    }
}
