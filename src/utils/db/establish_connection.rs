use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use lazy_static::lazy_static;
use std::env;

lazy_static! {
    static ref DATABASE_URL: String = env::var("DATABASE_URL").unwrap();
}

pub fn establish_connection() -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(DATABASE_URL.clone());
    // Refer to the `r2d2` documentation for more methods to use
    // when building a connection pool
    Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .expect("Could not build connection pool")
}
