use diesel::pg::PgConnection;
use diesel::prelude::*;
use lazy_static::lazy_static;
use std::env;

lazy_static! {
    static ref DATABASE_URL: String = env::var("DATABASE_URL").unwrap();
}

pub fn establish_connection() -> PgConnection {
    PgConnection::establish(&DATABASE_URL)
        .unwrap_or_else(|_| panic!("Error connecting to {}", DATABASE_URL.as_str()))
}
