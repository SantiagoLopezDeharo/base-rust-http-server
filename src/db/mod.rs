use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::{PgPoolOptions, PgRow};
use std::env;
use std::sync::OnceLock;

static POOL: OnceLock<PgPool> = OnceLock::new();

fn build_database_url() -> String {
    let host = env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string());
    let pass = env::var("DB_PASS").unwrap_or_else(|_| "postgres".to_string());
    let name = env::var("DB_NAME").unwrap_or_else(|_| "postgres".to_string());
    format!("postgres://{}:{}@{}:{}/{}", user, pass, host, port, name)
}

pub async fn init_pool() -> Result<&'static PgPool, sqlx::Error> {
    // ANSI color codes
    const CYAN: &str = "\x1b[36m";
    const GREEN: &str = "\x1b[32m";
    const YELLOW: &str = "\x1b[33m";
    const RESET: &str = "\x1b[0m";

    if let Some(pool) = POOL.get() {
        println!("{CYAN}DB pool already initialized.{RESET}");
        return Ok(pool);
    }

    let database_url = build_database_url();
    let max_connections = env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(10);

    println!("{CYAN}Connecting to database...{RESET}");

    println!("{GREEN}Max pool connections:{RESET} {YELLOW}{max_connections}{RESET}");

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&database_url)
        .await?;

    println!("{CYAN}DB pool initialized successfully!{RESET}");

    let _ = POOL.set(pool);
    Ok(POOL.get().expect("DB pool initialized"))
}

#[allow(dead_code)]
pub fn pool() -> &'static PgPool {
    POOL.get().expect("DB pool not initialized")
}

#[allow(dead_code)]
pub async fn ensure_migrations_tables() -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (\n  id TEXT PRIMARY KEY,\n  name TEXT NOT NULL,\n  applied_at TIMESTAMP NOT NULL DEFAULT NOW()\n);",
    )
    .execute(pool())
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _seeders (\n  id TEXT PRIMARY KEY,\n  name TEXT NOT NULL,\n  applied_at TIMESTAMP NOT NULL DEFAULT NOW()\n);",
    )
    .execute(pool())
    .await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn mark_migration_applied(id: &str, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO _migrations (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(name)
        .execute(pool())
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn unmark_migration_applied(id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM _migrations WHERE id = $1")
        .bind(id)
        .execute(pool())
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn mark_seed_applied(id: &str, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO _seeders (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(name)
        .execute(pool())
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn unmark_seed_applied(id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM _seeders WHERE id = $1")
        .bind(id)
        .execute(pool())
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn applied_migration_ids() -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT id FROM _migrations")
        .fetch_all(pool())
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("id").ok())
        .collect())
}

#[allow(dead_code)]
pub async fn applied_seed_ids() -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT id FROM _seeders")
        .fetch_all(pool())
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("id").ok())
        .collect())
}

#[allow(dead_code)]
pub async fn execute_sql(sql: &str) -> Result<(), sqlx::Error> {
    sqlx::query(sql).execute(pool()).await?;
    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum DbParam {
    Int32(i32),
    Int64(i64),
    Float64(f64),
    Bool(bool),
    Text(String),
}

#[allow(dead_code)]
pub async fn query(sql: &str, params: Vec<DbParam>) -> Result<Vec<PgRow>, sqlx::Error> {
    let mut q = sqlx::query(sql);
    for param in params {
        q = match param {
            DbParam::Int32(v) => q.bind(v),
            DbParam::Int64(v) => q.bind(v),
            DbParam::Float64(v) => q.bind(v),
            DbParam::Bool(v) => q.bind(v),
            DbParam::Text(v) => q.bind(v),
        };
    }
    q.fetch_all(pool()).await
}
