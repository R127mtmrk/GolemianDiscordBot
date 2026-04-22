use serenity::prelude::TypeMapKey;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

pub struct DatabaseKey;

impl TypeMapKey for DatabaseKey {
    type Value = SqlitePool;
}

#[derive(Debug, sqlx::FromRow)]
pub struct Warning {
    pub id: i64,
    pub guild_id: String,
    pub user_id: String,
    pub moderator_id: String,
    pub reason: String,
    pub created_at: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Giveaway {
    pub id: i64,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub prize: String,
    pub end_time: i64,
    pub winner_count: i64,
    pub ended: i64,
    pub created_by: String,
}

pub async fn create_pool(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let opts = SqliteConnectOptions::from_str(url)
        .expect("URL SQLite invalide")
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    init_tables(&pool).await?;
    Ok(pool)
}

async fn init_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS warnings (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            guild_id     TEXT NOT NULL,
            user_id      TEXT NOT NULL,
            moderator_id TEXT NOT NULL,
            reason       TEXT NOT NULL,
            created_at   INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS giveaways (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            guild_id     TEXT NOT NULL,
            channel_id   TEXT NOT NULL,
            message_id   TEXT NOT NULL,
            prize        TEXT NOT NULL,
            end_time     INTEGER NOT NULL,
            winner_count INTEGER NOT NULL DEFAULT 1,
            ended        INTEGER NOT NULL DEFAULT 0,
            created_by   TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS giveaway_entries (
            giveaway_id INTEGER NOT NULL,
            user_id     TEXT NOT NULL,
            PRIMARY KEY (giveaway_id, user_id)
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}
