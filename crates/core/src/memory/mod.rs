//! Memory trait and impls.

use async_trait::async_trait;
use kestrel_protocol::Message;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("{0}")]
    Msg(String),
}

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn load(&self, session_id: &str) -> Result<Vec<Message>, MemoryError>;
    async fn save(&self, session_id: &str, messages: &[Message]) -> Result<(), MemoryError>;
    async fn search(&self, query: &str, k: usize) -> Result<Vec<Message>, MemoryError>;
}

/// In-memory short-term memory.
pub struct InMemoryMemory {
    data: tokio::sync::RwLock<std::collections::BTreeMap<String, Vec<Message>>>,
}

impl InMemoryMemory {
    pub fn new() -> Self {
        Self { data: tokio::sync::RwLock::new(std::collections::BTreeMap::new()) }
    }
}

#[async_trait]
impl MemoryStore for InMemoryMemory {
    async fn load(&self, session_id: &str) -> Result<Vec<Message>, MemoryError> {
        Ok(self.data.read().await.get(session_id).cloned().unwrap_or_default())
    }
    async fn save(&self, session_id: &str, messages: &[Message]) -> Result<(), MemoryError> {
        self.data.write().await.insert(session_id.to_string(), messages.to_vec());
        Ok(())
    }
    async fn search(&self, query: &str, k: usize) -> Result<Vec<Message>, MemoryError> {
        let q = query.to_lowercase();
        let mut out = Vec::new();
        for msgs in self.data.read().await.values() {
            for m in msgs {
                if m.content.to_lowercase().contains(&q) {
                    out.push(m.clone());
                    if out.len() >= k { return Ok(out); }
                }
            }
        }
        Ok(out)
    }
}

impl Default for InMemoryMemory {
    fn default() -> Self { Self::new() }
}

/// None memory (no persistence).
pub struct NoneMemory;
#[async_trait]
impl MemoryStore for NoneMemory {
    async fn load(&self, _session_id: &str) -> Result<Vec<Message>, MemoryError> { Ok(Vec::new()) }
    async fn save(&self, _session_id: &str, _messages: &[Message]) -> Result<(), MemoryError> { Ok(()) }
    async fn search(&self, _query: &str, _k: usize) -> Result<Vec<Message>, MemoryError> { Ok(Vec::new()) }
}

/// SQLite-backed long-term memory with keyword recall.
pub struct SqliteMemory {
    pool: sqlx::SqlitePool,
}

impl SqliteMemory {
    pub async fn new(path: &str) -> Result<Self, MemoryError> {
        let url = format!("sqlite:{}?mode=rwc", path);
        let pool = sqlx::SqlitePool::connect(&url).await.map_err(|e| MemoryError::Msg(e.to_string()))?;
        sqlx::query("CREATE TABLE IF NOT EXISTS mem (session TEXT, role TEXT, content TEXT)")
            .execute(&pool).await.map_err(|e| MemoryError::Msg(e.to_string()))?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl MemoryStore for SqliteMemory {
    async fn load(&self, session_id: &str) -> Result<Vec<Message>, MemoryError> {
        let rows = sqlx::query_as::<_, (String, String)>("SELECT role, content FROM mem WHERE session = ?")
            .bind(session_id).fetch_all(&self.pool).await.map_err(|e| MemoryError::Msg(e.to_string()))?;
        let mut out = Vec::new();
        for (role, content) in rows {
            let r = match role.as_str() {
                "system" => kestrel_protocol::Role::System,
                "user" => kestrel_protocol::Role::User,
                "assistant" => kestrel_protocol::Role::Assistant,
                _ => kestrel_protocol::Role::Tool,
            };
            out.push(Message { role: r, content, tool_calls: Vec::new(), tool_results: Vec::new() });
        }
        Ok(out)
    }
    async fn save(&self, session_id: &str, messages: &[Message]) -> Result<(), MemoryError> {
        // replace
        sqlx::query("DELETE FROM mem WHERE session = ?").bind(session_id).execute(&self.pool).await.map_err(|e| MemoryError::Msg(e.to_string()))?;
        for m in messages {
            let role = format!("{:?}", m.role).to_lowercase();
            sqlx::query("INSERT INTO mem (session, role, content) VALUES (?,?,?)")
                .bind(session_id).bind(role).bind(&m.content).execute(&self.pool).await.map_err(|e| MemoryError::Msg(e.to_string()))?;
        }
        Ok(())
    }
    async fn search(&self, query: &str, k: usize) -> Result<Vec<Message>, MemoryError> {
        let like = format!("%{}%", query);
        let rows = sqlx::query_as::<_, (String, String)>("SELECT role, content FROM mem WHERE content LIKE ? LIMIT ?")
            .bind(like).bind(k as i64).fetch_all(&self.pool).await.map_err(|e| MemoryError::Msg(e.to_string()))?;
        let mut out = Vec::new();
        for (role, content) in rows {
            let r = match role.as_str() {
                "system" => kestrel_protocol::Role::System,
                "user" => kestrel_protocol::Role::User,
                "assistant" => kestrel_protocol::Role::Assistant,
                _ => kestrel_protocol::Role::Tool,
            };
            out.push(Message { role: r, content, tool_calls: Vec::new(), tool_results: Vec::new() });
        }
        Ok(out)
    }
}
