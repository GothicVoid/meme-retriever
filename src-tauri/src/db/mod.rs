use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::path::Path;

pub type DbPool = SqlitePool;

pub async fn init(app_data: &Path) -> anyhow::Result<DbPool> {
    let db_path = app_data.join("meme.db");
    let opts = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(opts).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
