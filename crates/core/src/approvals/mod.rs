//! Approvals in core: broker with timeout and deny-by-default.

use kestrel_protocol::{Command, Event};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub approval_id: String,
    pub session_id: String,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub reason: String,
}

pub struct ApprovalBroker {
    events_tx: mpsc::Sender<Event>,
    pending: tokio::sync::Mutex<BTreeMap<String, oneshot::Sender<bool>>>,
    timeout: Duration,
}

impl ApprovalBroker {
    pub fn new(events_tx: mpsc::Sender<Event>, timeout: Duration) -> Self {
        Self { events_tx, pending: tokio::sync::Mutex::new(BTreeMap::new()), timeout }
    }

    /// Request approval; emits ApprovalRequest event and waits for response or timeout (deny).
    pub async fn request(&self, req: ApprovalRequest) -> bool {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(req.approval_id.clone(), tx);
        let _ = self.events_tx.send(Event::ApprovalRequest {
            approval_id: req.approval_id.clone(),
            session_id: req.session_id.clone(),
            tool: req.tool.clone(),
            arguments: req.arguments.clone(),
            reason: req.reason.clone(),
        }).await;
        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(approved)) => approved,
            _ => false, // deny by default on timeout
        }
    }

    /// Resolve an approval response.
    pub async fn respond(&self, approval_id: &str, approved: bool) {
        if let Some(tx) = self.pending.lock().await.remove(approval_id) {
            let _ = tx.send(approved);
        }
    }

    /// Handle incoming command.
    pub async fn handle_command(&self, cmd: Command) {
        if let Command::ApprovalResponse { approval_id, approved } = cmd {
            self.respond(&approval_id, approved).await;
        }
    }
}
EOF