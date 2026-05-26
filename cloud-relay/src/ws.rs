use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use shared::WsMessage;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Shared state holding all active connections.
pub struct AppState {
    /// PC connections: device_id → sender channel
    pub pcs: DashMap<String, mpsc::UnboundedSender<Message>>,
    /// Mobile connections: user_id → sender channel
    pub mobiles: DashMap<String, mpsc::UnboundedSender<Message>>,
    /// Which mobile user is targeting which PC: user_id → device_id
    pub active_targets: DashMap<String, String>,
    pub db: crate::db::DbPool,
}

impl AppState {
    pub fn new(db: crate::db::DbPool) -> Self {
        Self {
            pcs: DashMap::new(),
            mobiles: DashMap::new(),
            active_targets: DashMap::new(),
            db,
        }
    }
}

pub type SharedState = Arc<AppState>;

/// Handle a PC agent WebSocket connection.
pub async fn handle_pc_socket(socket: WebSocket, state: SharedState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Wait for the bind message
    let device_id = match receiver.next().await {
        Some(Ok(msg)) => {
            let text = msg.to_text().unwrap_or("");
            match serde_json::from_str::<WsMessage>(text) {
                Ok(WsMessage::PcBind { device_id, token }) => {
                    // Verify token against DB
                    let valid = {
                        match crate::db::get_device_token(&state.db, &device_id) {
                            Ok(Some(db_token)) => db_token == token,
                            _ => false,
                        }
                    };
                    if !valid {
                        let ack = serde_json::to_string(&WsMessage::BindAck {
                            success: false,
                            device_id: None,
                            message: "Invalid device token".to_string(),
                        }).unwrap();
                        let _ = sender.send(Message::Text(ack.into())).await;
                        return;
                    }
                    info!("PC bound: {}", device_id);
                    let ack = serde_json::to_string(&WsMessage::BindAck {
                        success: true,
                        device_id: Some(device_id.clone()),
                        message: "Connected".to_string(),
                    }).unwrap();
                    let _ = sender.send(Message::Text(ack.into())).await;
                    device_id
                }
                _ => {
                    warn!("PC connected without valid bind message");
                    return;
                }
            }
        }
        _ => return,
    };

    // Register the PC connection
    state.pcs.insert(device_id.clone(), tx.clone());

    // Notify mobiles that this PC is online
    broadcast_device_status(&state, &device_id, true).await;

    // Forward messages from rx channel to the WebSocket sender
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Read messages from PC and route them
    let state_clone = state.clone();
    let did = device_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Ok(text) = msg.to_text() {
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(text) {
                    route_pc_message(&state_clone, &did, ws_msg).await;
                }
            }
        }
    });

    // Wait for either task to finish (connection lost)
    let _ = tokio::select! {
        _ = send_task => (),
        _ = recv_task => (),
    };

    // Cleanup
    state.pcs.remove(&device_id);
    broadcast_device_status(&state, &device_id, false).await;
    info!("PC disconnected: {}", device_id);
}

/// Handle a mobile app WebSocket connection.
pub async fn handle_mobile_socket(socket: WebSocket, state: SharedState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Wait for bind message
    let user_id = match receiver.next().await {
        Some(Ok(msg)) => {
            let text = msg.to_text().unwrap_or("");
            match serde_json::from_str::<WsMessage>(text) {
                Ok(WsMessage::MobileBind { user_token }) => {
                    match crate::auth::validate_token(&user_token) {
                        Ok(claims) => {
                            info!("Mobile bound: {} ({})", claims.username, claims.sub);
                            let ack = serde_json::to_string(&WsMessage::BindAck {
                                success: true,
                                device_id: None,
                                message: "Connected".to_string(),
                            }).unwrap();
                            let _ = sender.send(Message::Text(ack.into())).await;
                            claims.sub
                        }
                        Err(e) => {
                            warn!("Invalid mobile token: {}", e);
                            let ack = serde_json::to_string(&WsMessage::BindAck {
                                success: false,
                                device_id: None,
                                message: "Invalid token".to_string(),
                            }).unwrap();
                            let _ = sender.send(Message::Text(ack.into())).await;
                            return;
                        }
                    }
                }
                _ => {
                    warn!("Mobile connected without valid bind message");
                    return;
                }
            }
        }
        _ => return,
    };

    state.mobiles.insert(user_id.clone(), tx.clone());

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let uid = user_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Ok(text) = msg.to_text() {
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(text) {
                    route_mobile_message(&state_clone, &uid, ws_msg).await;
                }
            }
        }
    });

    let _ = tokio::select! {
        _ = send_task => (),
        _ = recv_task => (),
    };

    state.mobiles.remove(&user_id);
    state.active_targets.remove(&user_id);
    info!("Mobile disconnected: {}", user_id);
}

/// Route messages from a PC agent.
async fn route_pc_message(state: &SharedState, _pc_id: &str, msg: WsMessage) {
    match msg {
        WsMessage::Heartbeat => {
            // Send ack back to PC — find the PC's sender
            if let Some(pc_tx) = state.pcs.get(_pc_id) {
                let ack = serde_json::to_string(&WsMessage::HeartbeatAck).unwrap();
                let _ = pc_tx.send(Message::Text(ack.into()));
            }
        }
        WsMessage::TaskResponse(response) => {
            // Forward to whichever mobile user requested this task
            // We broadcast to all mobiles that have this PC as active target
            let resp_json = serde_json::to_string(&WsMessage::TaskResponse(response)).unwrap();
            for entry in state.active_targets.iter() {
                if entry.value() == _pc_id {
                    if let Some(mobile_tx) = state.mobiles.get(entry.key()) {
                        let _ = mobile_tx.send(Message::Text(resp_json.clone().into()));
                    }
                }
            }
        }
        _ => {
            warn!("Unexpected message from PC {}: {:?}", _pc_id, msg);
        }
    }
}

/// Route messages from a mobile app.
async fn route_mobile_message(state: &SharedState, user_id: &str, msg: WsMessage) {
    match msg {
        WsMessage::Command { target_pc_id, message } => {
            // Set active target
            state.active_targets.insert(user_id.to_string(), target_pc_id.clone());

            // Forward to PC
            if let Some(pc_tx) = state.pcs.get(&target_pc_id) {
                let forward = serde_json::to_string(&WsMessage::Command {
                    target_pc_id: target_pc_id.clone(),
                    message,
                }).unwrap();
                let _ = pc_tx.send(Message::Text(forward.into()));
            } else {
                // PC not online
                if let Some(mobile_tx) = state.mobiles.get(user_id) {
                    let err = serde_json::to_string(&WsMessage::Error {
                        code: 404,
                        message: format!("PC {} is not online", target_pc_id),
                    }).unwrap();
                    let _ = mobile_tx.send(Message::Text(err.into()));
                }
            }
        }
        WsMessage::Heartbeat => {
            if let Some(mobile_tx) = state.mobiles.get(user_id) {
                let ack = serde_json::to_string(&WsMessage::HeartbeatAck).unwrap();
                let _ = mobile_tx.send(Message::Text(ack.into()));
            }
        }
        _ => {
            warn!("Unexpected message from mobile {}: {:?}", user_id, msg);
        }
    }
}

/// Broadcast device online/offline status to all connected mobiles.
async fn broadcast_device_status(state: &SharedState, device_id: &str, online: bool) {
    let status = serde_json::to_string(&WsMessage::DeviceStatus {
        device_id: device_id.to_string(),
        online,
    }).unwrap();
    for mobile_tx in state.mobiles.iter() {
        let _ = mobile_tx.send(Message::Text(status.clone().into()));
    }
}
