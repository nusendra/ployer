use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::app_state::SharedState;
use crate::auth::validate_token;
use ployer_core::models::WsEvent;

// Client message types (from browser to server)
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum WsClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe { channel: String },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { channel: String },
    #[serde(rename = "ping")]
    Ping,
}

// Server message types (from server to browser)
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
enum WsServerMessage {
    #[serde(rename = "server_health")]
    ServerHealth {
        server_id: String,
        status: String,
        timestamp: String,
    },
    #[serde(rename = "container_logs")]
    ContainerLogs {
        container_id: String,
        line: String,
        timestamp: String,
    },
    #[serde(rename = "container_stats")]
    ContainerStats {
        container_id: String,
        cpu_usage: f64,
        memory_usage_mb: f64,
        memory_limit_mb: f64,
    },
    #[serde(rename = "deployment_status")]
    DeploymentStatus {
        deployment_id: String,
        status: String,
        message: Option<String>,
    },
    #[serde(rename = "deployment_logs")]
    DeploymentLogs {
        deployment_id: String,
        line: String,
        timestamp: String,
    },
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "error")]
    Error { message: String },
}

// Connection manager to track active WebSocket connections
type Subscriptions = Arc<Mutex<HashMap<String, HashSet<String>>>>;

#[derive(Clone)]
pub struct ConnectionManager {
    subscriptions: Subscriptions,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn subscribe(&self, conn_id: &str, channel: &str) {
        let mut subs = self.subscriptions.lock().await;
        subs.entry(channel.to_string())
            .or_insert_with(HashSet::new)
            .insert(conn_id.to_string());
        info!("Client {} subscribed to channel: {}", conn_id, channel);
    }

    async fn unsubscribe(&self, conn_id: &str, channel: &str) {
        let mut subs = self.subscriptions.lock().await;
        if let Some(channel_subs) = subs.get_mut(channel) {
            channel_subs.remove(conn_id);
            if channel_subs.is_empty() {
                subs.remove(channel);
            }
        }
        info!("Client {} unsubscribed from channel: {}", conn_id, channel);
    }

    async fn cleanup(&self, conn_id: &str) {
        let mut subs = self.subscriptions.lock().await;
        subs.retain(|_, clients| {
            clients.remove(conn_id);
            !clients.is_empty()
        });
        info!("Cleaned up subscriptions for client: {}", conn_id);
    }
}

// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    token: String,
}

// WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<SharedState>,
) -> Response {
    // Validate JWT token
    let user_id = match validate_token(&query.token, &state.config.auth.jwt_secret) {
        Ok(claims) => claims.sub,
        Err(_) => {
            warn!("WebSocket connection denied: invalid token");
            return ws.on_upgrade(|mut socket| async move {
                let error_msg = WsServerMessage::Error {
                    message: "Invalid authentication token".to_string(),
                };
                if let Ok(json) = serde_json::to_string(&error_msg) {
                    let _ = socket.send(Message::Text(json)).await;
                }
                let _ = socket.close().await;
            });
        }
    };

    info!("WebSocket connection established for user: {}", user_id);

    ws.on_upgrade(move |socket| handle_socket(socket, user_id, state))
}

async fn handle_socket(socket: WebSocket, user_id: String, state: SharedState) {
    let (mut sender, mut receiver) = socket.split();
    let conn_id = uuid::Uuid::new_v4().to_string();

    let manager = ConnectionManager::new();

    // Subscribe to broadcast channel
    let mut broadcast_rx = state.ws_broadcast.subscribe();

    // Task to forward broadcast messages to this client
    let manager_clone = manager.clone();
    let conn_id_clone = conn_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = broadcast_rx.recv().await {
            // Convert ployer_core::models::WsEvent to our WsServerMessage
            let message = match event {
                WsEvent::ServerHealth { server_id, status } => {
                    Some(WsServerMessage::ServerHealth {
                        server_id,
                        status: status.as_str().to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    })
                }
                WsEvent::DeploymentLog { deployment_id, line } => {
                    Some(WsServerMessage::DeploymentLogs {
                        deployment_id,
                        line,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    })
                }
                WsEvent::DeploymentStatus { deployment_id, status, .. } => {
                    Some(WsServerMessage::DeploymentStatus {
                        deployment_id,
                        status: status.as_str().to_string(),
                        message: None,
                    })
                }
                WsEvent::ContainerStats { container_id, cpu_percent, memory_mb } => {
                    Some(WsServerMessage::ContainerStats {
                        container_id,
                        cpu_usage: cpu_percent,
                        memory_usage_mb: memory_mb,
                        memory_limit_mb: 0.0, // Not available in this event
                    })
                }
            };

            if let Some(msg) = message {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }

        manager_clone.cleanup(&conn_id_clone).await;
    });

    // Task to handle incoming messages from client
    let manager_clone = manager.clone();
    let conn_id_clone = conn_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<WsClientMessage>(&text) {
                    Ok(WsClientMessage::Subscribe { channel }) => {
                        manager_clone.subscribe(&conn_id_clone, &channel).await;
                    }
                    Ok(WsClientMessage::Unsubscribe { channel }) => {
                        manager_clone.unsubscribe(&conn_id_clone, &channel).await;
                    }
                    Ok(WsClientMessage::Ping) => {
                        // Send pong back
                        let pong = WsServerMessage::Pong;
                        if let Ok(_json) = serde_json::to_string(&pong) {
                            // Note: Can't send here directly, would need a channel
                            // For now, ping/pong is mostly for keepalive
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse WebSocket message: {}", e);
                    }
                }
            } else if let Message::Close(_) = msg {
                break;
            }
        }

        manager_clone.cleanup(&conn_id_clone).await;
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    info!("WebSocket connection closed for user: {}", user_id);
}
