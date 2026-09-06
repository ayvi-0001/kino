extern crate kino_db;

pub use kino_db::Watchlist;
#[cfg(feature = "postgres")]
pub use kino_db_postgres::Database;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub use kino_db_sqlite::Database;

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!("either the `sqlite` or `postgres` feature must be enabled");
