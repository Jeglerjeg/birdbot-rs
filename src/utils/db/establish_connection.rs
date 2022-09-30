use diesel::prelude::*;

pub fn establish_connection() -> SqliteConnection {
    SqliteConnection::establish("sqlite://bot.db?mode=rwc")
        .unwrap_or_else(|_| panic!("Error connecting to {}", "sqlite://bot.db?mode=rwc"))
}
