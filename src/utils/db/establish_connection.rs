use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::mobc::Pool;
use std::env;
use std::sync::OnceLock;

static DATABASE_URL: OnceLock<String> = OnceLock::new();

pub fn establish_connection() -> mobc::Pool<AsyncDieselConnectionManager<AsyncPgConnection>> {
    let config =
        AsyncDieselConnectionManager::<AsyncPgConnection>::new(DATABASE_URL.get_or_init(|| {
            env::var("DATABASE_URL").expect("DATABASE_URL env variable must be set.")
        }));
    Pool::new(config)
}
