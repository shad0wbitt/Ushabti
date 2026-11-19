//! Sessions persisted in SQLite.

use async_trait::async_trait;
use kestrel_protocol::{Message, Usage};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("{0}")]
    Msg(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub created_at: i64,
    pub recipe_hash: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub meta: SessionMeta,
    pub messages: Vec<Message>,
    pub usage: Usage,
    pub cost_usd: f64,
}

pub struct SessionStore {
    pool: SqlitePool,
}

impl SessionStore {
    pub async fn new(path: &str) -> Result<Self, SessionError> {
        let url = format!("sqlite:{}?mode=rwc", path);
        let pool = SqlitePool::connect(&url).await.map_err(|e| SessionError::Msg(e.to_string()))?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, created_at INTEGER, recipe_hash TEXT, title TEXT, messages TEXT, usage TEXT, cost REAL)",
        )
        .execute(&pool)
        .await
        .map_err(|e| SessionError::Msg(e.to_string()))?;
        Ok(Self { pool })
    }

    pub async fn save(&self, rec: &SessionRecord) -> Result<(), SessionError> {
        let messages = serde_json::to_string(&rec.messages).map_err(|e| SessionError::Msg(e.to_string()))?;
        let usage = serde_json::to_string(&rec.usage).map_err(|e| SessionError::Msg(e.to_string()))?;
        sqlx::query(
            "INSERT OR REPLACE INTO sessions (id, created_at, recipe_hash, title, messages, usage, cost) VALUES (?,?,?,?,?,?,?)",
        )
        .bind(&rec.meta.id)
        .bind(rec.meta.created_at)
        .bind(&rec.meta.recipe_hash)
        .bind(&rec.meta.title)
        .bind(messages)
        .bind(usage)
        .bind(rec.cost_usd)
        .execute(&self.pool)
        .await
        .map_err(|e| SessionError::Msg(e.to_string()))?;
        Ok(())
    }

    pub async fn load(&self, id: &str) -> Result<SessionRecord, SessionError> {
        let row = sqlx::query_as::<_, (i64, Option<String>, Option<String>, String, String, f64)>(
            "SELECT created_at, recipe_hash, title, messages, usage, cost FROM sessions WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SessionError::Msg(e.to_string()))?;
        let (created_at, recipe_hash, title, messages, usage, cost) = row;
        let messages: Vec<Message> = serde_json::from_str(&messages).map_err(|e| SessionError::Msg(e.to_string()))?;
        let usage: Usage = serde_json::from_str(&usage).map_err(|e| SessionError::Msg(e.to_string()))?;
        Ok(SessionRecord {
            meta: SessionMeta { id: id.to_string(), created_at, recipe_hash, title },
            messages,
            usage,
            cost_usd: cost,
        })
    }

    pub async fn list(&self) -> Result<Vec<SessionMeta>, SessionError> {
        let rows = sqlx::query_as::<_, (String, i64, Option<String>, Option<String>)>(
            "SELECT id, created_at, recipe_hash, title FROM sessions ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SessionError::Msg(e.to_string()))?;
        Ok(rows.into_iter().map(|(id, created_at, recipe_hash, title)| SessionMeta { id, created_at, recipe_hash, title }).collect())
    }
}
