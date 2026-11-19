//! Daemon binary.

use anyhow::Result;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Json, Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::info;

#[derive(Clone)]
struct AppState {
    token: String,
    events_tx: mpsc::Sender<kestrel_protocol::Event>,
    events_rx: broadcast::Sender<kestrel_protocol::Event>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let token = std::env::var("HARNESSD_TOKEN").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
    let (tx, _rx) = mpsc::channel(256);
    let (btx, _) = broadcast::channel(256);
    let state = Arc::new(AppState { token, events_tx: tx, events_rx: btx });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/health", get(health))
        .route("/api/modules", get(modules))
        .route("/api/recipes", get(list_recipes).post(create_recipe))
        .route("/api/recipes/:id/versions", get(recipe_versions))
        .route("/api/recipes/:id/restore", post(restore_recipe))
        .route("/api/sessions", get(list_sessions).post(create_session))
        .route("/api/sessions/:id", get(get_session).delete(delete_session))
        .route("/api/sessions/:id/trace", get(session_trace))
        .with_state(state);

    let addr: SocketAddr = "127.0.0.1:7878".parse().unwrap();
    info!("harnessd listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status":"ok","protocol_version": kestrel_protocol::PROTOCOL_VERSION}))
}

async fn modules() -> Json<serde_json::Value> {
    Json(serde_json::json!({"modules": []}))
}

async fn list_recipes() -> Json<serde_json::Value> { Json(serde_json::json!({"recipes": []})) }
async fn create_recipe() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
async fn recipe_versions(Path(_id): Path<String>) -> Json<serde_json::Value> { Json(serde_json::json!({"versions": []})) }
async fn restore_recipe(Path(_id): Path<String>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
async fn list_sessions() -> Json<serde_json::Value> { Json(serde_json::json!({"sessions": []})) }
async fn create_session() -> StatusCode { StatusCode::CREATED }
async fn get_session(Path(_id): Path<String>) -> Json<serde_json::Value> { Json(serde_json::json!({})) }
async fn delete_session(Path(_id): Path<String>) -> StatusCode { StatusCode::NO_CONTENT }
async fn session_trace(Path(_id): Path<String>) -> Json<serde_json::Value> { Json(serde_json::json!({"events": []})) }

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    // hello
    let hello = serde_json::json!({"type":"hello","protocol_version": kestrel_protocol::PROTOCOL_VERSION});
    let _ = socket.send(Message::Text(hello.to_string())).await;
    let mut rx = state.events_rx.subscribe();
    loop {
        tokio::select! {
            Ok(ev) = rx.recv() => {
                if let Ok(s) = serde_json::to_string(&ev) {
                    if socket.send(Message::Text(s)).await.is_err() { break; }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(t))) => {
                        // parse Command; handle approval etc. (stub)
                        let _ = t;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
