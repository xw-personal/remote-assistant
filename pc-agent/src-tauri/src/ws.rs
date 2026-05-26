use futures_util::{SinkExt, StreamExt};
use shared::WsMessage;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{info, warn};

type WsSplitSink = futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

/// Global WebSocket connection state.
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

/// Connect to the relay server and start message loop.
pub async fn connect(
    app: AppHandle,
    state: Arc<WsState>,
    server_url: &str,
    device_id: &str,
    device_token: &str,
) -> Result<(), String> {
    let ws_url = format!("{}/ws/pc", server_url.replace("http", "ws"));
    info!("Connecting to relay: {}", ws_url);

    let (ws_stream, _) = connect_async(&ws_url)
        .await
        .map_err(|e| format!("WebSocket connect failed: {}", e))?;

    let (write, mut read) = ws_stream.split();

    // Send bind message directly before storing sink
    {
        let mut temp_sink = write;
        let bind_msg = WsMessage::PcBind {
            device_id: device_id.to_string(),
            token: device_token.to_string(),
        };
        let bind_json = serde_json::to_string(&bind_msg).unwrap();
        temp_sink.send(Message::Text(bind_json.into())).await
            .map_err(|e| format!("Failed to send bind: {}", e))?;

        // Wait for bind ack
        if let Some(Ok(msg)) = read.next().await {
            if let Ok(text) = msg.to_text() {
                if let Ok(WsMessage::BindAck { success, device_id: id, message }) = serde_json::from_str::<WsMessage>(text) {
                    if success {
                        info!("Bind successful: {:?}", id);
                    } else {
                        return Err(format!("Bind rejected: {}", message));
                    }
                }
            }
        }

        // Store the sink
        *state.sink.lock().await = Some(temp_sink);
    }

    *state.connected.lock().await = true;

    // Spawn heartbeat task using a channel approach
    let (hb_tx, mut hb_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // Heartbeat sender
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

    // Forward heartbeat messages to the sink
    let state_for_hb = state.clone();
    tokio::spawn(async move {
        while let Some(msg) = hb_rx.recv().await {
            let mut guard = state_for_hb.sink.lock().await;
            if let Some(ref mut sink) = *guard {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
        }
    });

    // Spawn message reader
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        handle_relay_message(&app_clone, ws_msg).await;
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

/// Handle messages from the relay server.
async fn handle_relay_message(app: &AppHandle, msg: WsMessage) {
    match msg {
        WsMessage::Command { target_pc_id: _, message } => {
            info!("Received command: {}", message);
            let _ = app.emit("remote-command", message);
        }
        WsMessage::TaskRequest(task) => {
            info!("Received task: {:?}", task);
            let json = serde_json::to_string(&task).unwrap();
            let _ = app.emit("task-request", json);
        }
        WsMessage::HeartbeatAck => {}
        WsMessage::Error { code, message } => {
            warn!("Relay error {}: {}", code, message);
            let _ = app.emit("ws-error", format!("{}: {}", code, message));
        }
        _ => {
            info!("Received: {:?}", msg);
        }
    }
}

/// Send a task response back through the relay.
pub async fn send_response(state: &WsState, response: &WsMessage) -> Result<(), String> {
    let json = serde_json::to_string(response).unwrap();
    let mut guard = state.sink.lock().await;
    if let Some(ref mut sink) = *guard {
        sink.send(Message::Text(json.into())).await
            .map_err(|e| format!("Send failed: {}", e))?;
    }
    Ok(())
}
