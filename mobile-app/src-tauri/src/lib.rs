mod ws;

use std::sync::Arc;
use tauri::{AppHandle, State};
use ws::WsState;

#[tauri::command]
async fn login(
    server_url: String,
    username: String,
    password: String,
) -> Result<String, String> {
    let resp = ws::login(&server_url, &username, &password).await?;
    serde_json::to_string(&resp).map_err(|e| e.to_string())
}

#[tauri::command]
async fn register(
    server_url: String,
    username: String,
    password: String,
) -> Result<String, String> {
    let resp = ws::register(&server_url, &username, &password).await?;
    serde_json::to_string(&resp).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_devices(
    server_url: String,
    token: String,
) -> Result<String, String> {
    let devices = ws::fetch_devices(&server_url, &token).await?;
    serde_json::to_string(&devices).map_err(|e| e.to_string())
}

#[tauri::command]
async fn bind_device(
    server_url: String,
    token: String,
    pairing_code: String,
) -> Result<String, String> {
    let resp = ws::bind_device(&server_url, &token, &pairing_code).await?;
    serde_json::to_string(&resp).map_err(|e| e.to_string())
}

#[tauri::command]
async fn connect_ws(
    server_url: String,
    user_token: String,
    ws_state: State<'_, Arc<WsState>>,
    app: AppHandle,
) -> Result<String, String> {
    ws::connect(app, ws_state.inner().clone(), &server_url, &user_token)
        .await
        .map(|_| "Connected".to_string())
}

#[tauri::command]
async fn send_command(
    target_pc_id: String,
    message: String,
    ws_state: State<'_, Arc<WsState>>,
) -> Result<(), String> {
    ws::send_command(&ws_state, &target_pc_id, &message).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(WsState::new()))
        .invoke_handler(tauri::generate_handler![
            login,
            register,
            get_devices,
            bind_device,
            connect_ws,
            send_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running mobile app");
}
