use futures_util::{SinkExt, StreamExt};
use shared::WsMessage;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{info, warn};

type WsSplitSink = futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

pub struct WsState {
    pub sink: Mutex<Option<WsSplitSink>>,
    pub connected: Mutex<bool>,
}

impl WsState {
    pub fn new() -> Self {
        Self {
            sink: Mutex::new(None),
            connected: Mutex::new(false),
        }
    }
}

/// Connect to the relay server as a mobile client.
pub async fn connect(
    app: AppHandle,
    state: Arc<WsState>,
    server_url: &str,
    user_token: &str,
) -> Result<(), String> {
    let ws_url = format!("{}/ws/mobile", server_url.replace("http", "ws"));
    info!("Mobile connecting to relay: {}", ws_url);

    let (ws_stream, _) = connect_async(&ws_url)
        .await
        .map_err(|e| format!("WebSocket connect failed: {}", e))?;

    let (write, mut read) = ws_stream.split();

    // Send bind message
    {
        let mut temp_sink = write;
        let bind_msg = WsMessage::MobileBind {
            user_token: user_token.to_string(),
        };
        let bind_json = serde_json::to_string(&bind_msg).unwrap();
        temp_sink.send(Message::Text(bind_json.into())).await
            .map_err(|e| format!("Failed to send bind: {}", e))?;

        // Wait for bind ack
        if let Some(Ok(msg)) = read.next().await {
            if let Ok(text) = msg.to_text() {
                if let Ok(WsMessage::BindAck { success, message, .. }) = serde_json::from_str::<WsMessage>(text) {
                    if !success {
                        return Err(format!("Bind rejected: {}", message));
                    }
                    info!("Mobile bind successful");
                }
            }
        }

        *state.sink.lock().await = Some(temp_sink);
    }

    *state.connected.lock().await = true;

    // Heartbeat via channel
    let (hb_tx, mut hb_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let hb = serde_json::to_string(&WsMessage::Heartbeat).unwrap();
            if hb_tx.send(Message::Text(hb.into())).is_err() {
                break;
            }
        }
    });

    let state_hb = state.clone();
    tokio::spawn(async move {
        while let Some(msg) = hb_rx.recv().await {
            let mut guard = state_hb.sink.lock().await;
            if let Some(ref mut sink) = *guard {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
        }
    });

    // Message reader
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        handle_message(&app_clone, ws_msg).await;
                    }
                }
                Message::Close(_) => {
                    warn!("Relay connection closed");
                    let _ = app_clone.emit("ws-disconnected", ());
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(())
}

async fn handle_message(app: &AppHandle, msg: WsMessage) {
    match msg {
        WsMessage::TaskResponse(response) => {
            let json = serde_json::to_string(&response).unwrap();
            let _ = app.emit("task-response", json);
        }
        WsMessage::Error { code, message } => {
            let _ = app.emit("ws-error", format!("{}: {}", code, message));
        }
        WsMessage::DeviceStatus { device_id, online } => {
            let _ = app.emit("device-status", serde_json::json!({
                "device_id": device_id,
                "online": online
            }).to_string());
        }
        WsMessage::HeartbeatAck => {}
        _ => {
            info!("Mobile received: {:?}", msg);
        }
    }
}

/// Send a command to a PC through the relay.
pub async fn send_command(state: &WsState, target_pc_id: &str, message: &str) -> Result<(), String> {
    let cmd = WsMessage::Command {
        target_pc_id: target_pc_id.to_string(),
        message: message.to_string(),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let mut guard = state.sink.lock().await;
    if let Some(ref mut sink) = *guard {
        sink.send(Message::Text(json.into())).await
            .map_err(|e| format!("Send failed: {}", e))?;
    }
    Ok(())
}

/// Fetch devices list via HTTP.
pub async fn fetch_devices(server_url: &str, token: &str) -> Result<Vec<shared::Device>, String> {
    let url = format!("{}/api/devices", server_url);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let body: shared::DeviceListResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;

    Ok(body.devices)
}

/// Login to the relay server.
pub async fn login(server_url: &str, username: &str, password: &str) -> Result<shared::LoginResponse, String> {
    let url = format!("{}/api/login", server_url);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&shared::LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        })
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err("Login failed: invalid credentials".to_string());
    }

    resp.json().await.map_err(|e| format!("Parse failed: {}", e))
}

/// Register a new user.
pub async fn register(server_url: &str, username: &str, password: &str) -> Result<shared::LoginResponse, String> {
    let url = format!("{}/api/register", server_url);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&shared::RegisterRequest {
            username: username.to_string(),
            password: password.to_string(),
        })
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err("Registration failed".to_string());
    }

    resp.json().await.map_err(|e| format!("Parse failed: {}", e))
}

/// Bind a device with a pairing code.
pub async fn bind_device(server_url: &str, token: &str, pairing_code: &str) -> Result<shared::BindResponse, String> {
    let url = format!("{}/api/bind", server_url);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&shared::BindRequest {
            pairing_code: pairing_code.to_string(),
        })
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    resp.json().await.map_err(|e| format!("Parse failed: {}", e))
}
