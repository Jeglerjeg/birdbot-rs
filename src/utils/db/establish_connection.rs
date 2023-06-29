use diesel_async::pooled_connection::mobc::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use lazy_static::lazy_static;
use std::env;

lazy_static! {
    static ref DATABASE_URL: String =
        env::var("DATABASE_URL").expect("DATABASE_URL env variable must be set.");
}

pub fn establish_connection() -> mobc::Pool<AsyncDieselConnectionManager<AsyncPgConnection>> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(DATABASE_URL.clone());
    Pool::new(config)
}
