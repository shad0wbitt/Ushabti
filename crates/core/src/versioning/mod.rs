//! Recipe versioning in SQLite.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("{0}")]
    Msg(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeVersion {
    pub hash: String,
    pub timestamp: i64,
    pub content: String,
}

pub struct RecipeVersionStore {
    pool: SqlitePool,
}

impl RecipeVersionStore {
    pub async fn new(path: &str) -> Result<Self, VersionError> {
        let url = format!("sqlite:{}?mode=rwc", path);
        let pool = SqlitePool::connect(&url).await.map_err(|e| VersionError::Msg(e.to_string()))?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS recipe_versions (hash TEXT PRIMARY KEY, timestamp INTEGER, content TEXT)",
        )
        .execute(&pool)
        .await
        .map_err(|e| VersionError::Msg(e.to_string()))?;
        Ok(Self { pool })
    }

    pub async fn save(&self, content: &str) -> Result<RecipeVersion, VersionError> {
        let mut h = Sha256::new();
        h.update(content.as_bytes());
        let hash = format!("{:x}", h.finalize());
        let timestamp = chrono::Utc::now().timestamp();
        sqlx::query("INSERT OR IGNORE INTO recipe_versions (hash, timestamp, content) VALUES (?,?,?)")
            .bind(&hash).bind(timestamp).bind(content)
            .execute(&self.pool).await.map_err(|e| VersionError::Msg(e.to_string()))?;
        Ok(RecipeVersion { hash, timestamp, content: content.to_string() })
    }

    pub async fn list(&self) -> Result<Vec<RecipeVersion>, VersionError> {
        let rows = sqlx::query_as::<_, (String, i64, String)>(
            "SELECT hash, timestamp, content FROM recipe_versions ORDER BY timestamp DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| VersionError::Msg(e.to_string()))?;
        Ok(rows.into_iter().map(|(hash, timestamp, content)| RecipeVersion { hash, timestamp, content }).collect())
    }

    pub async fn restore(&self, hash: &str) -> Result<Option<RecipeVersion>, VersionError> {
        let row = sqlx::query_as::<_, (String, i64, String)>(
            "SELECT hash, timestamp, content FROM recipe_versions WHERE hash = ?",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| VersionError::Msg(e.to_string()))?;
        Ok(row.map(|(hash, timestamp, content)| RecipeVersion { hash, timestamp, content }))
    }
}
