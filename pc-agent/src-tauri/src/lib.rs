mod sub_agents;
mod ws;

use shared::{TaskRequest, TaskResponse, TaskType, WsMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use ws::WsState;

/// Execute a task dispatched by the frontend (LLM-parsed or direct).
#[tauri::command]
async fn execute_task(
    task_json: String,
    ws_state: State<'_, Arc<WsState>>,
) -> Result<String, String> {
    let task: TaskRequest = serde_json::from_str(&task_json)
        .map_err(|e| format!("Invalid task JSON: {}", e))?;

    tracing::info!("Executing task: {:?} / {}", task.task_type, task.action);

    let response = match task.task_type {
        TaskType::Browser => sub_agents::browser::execute(&task),
        TaskType::FileManagement => sub_agents::file_manager::execute(&task),
        TaskType::SystemControl => sub_agents::system::execute(&task),
        TaskType::Simulation => sub_agents::simulator::execute(&task),
        TaskType::Document => {
            TaskResponse::failure(&task.task_id, "Document agent not yet implemented")
        }
    };

    // If this task came from a remote source, send the response back through the relay
    let ws_msg = WsMessage::TaskResponse(response.clone());
    if let Err(e) = ws::send_response(&ws_state, &ws_msg).await {
        tracing::warn!("Failed to send response to relay: {}", e);
    }

    serde_json::to_string(&response).map_err(|e| format!("Serialize error: {}", e))
}

/// Quick command: open a URL and return screenshot.
#[tauri::command]
async fn open_url(url: String) -> Result<String, String> {
    let mut params = HashMap::new();
    params.insert("url".to_string(), url);
    let task = TaskRequest::new(TaskType::Browser, "open_url", params);
    let response = sub_agents::browser::execute(&task);
    serde_json::to_string(&response).map_err(|e| format!("Serialize error: {}", e))
}

/// Quick command: take a screenshot.
#[tauri::command]
async fn take_screenshot() -> Result<String, String> {
    let screenshot = sub_agents::browser::capture_screenshot_base64();
    Ok(screenshot)
}

/// Connect to the relay server.
#[tauri::command]
async fn connect_to_server(
    server_url: String,
    device_id: String,
    device_token: String,
    ws_state: State<'_, Arc<WsState>>,
    app: AppHandle,
) -> Result<String, String> {
    ws::connect(app, ws_state.inner().clone(), &server_url, &device_id, &device_token)
        .await
        .map(|_| "Connected".to_string())
}

/// Get system info as JSON.
#[tauri::command]
async fn get_system_info() -> Result<String, String> {
    let mut params = HashMap::new();
    params.insert("format".to_string(), "json".to_string());
    let task = TaskRequest::new(TaskType::SystemControl, "info", params);
    let response = sub_agents::system::execute(&task);
    serde_json::to_string(&response).map_err(|e| format!("Serialize error: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(WsState::new()))
        .invoke_handler(tauri::generate_handler![
            execute_task,
            open_url,
            take_screenshot,
            connect_to_server,
            get_system_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PC Butler");
}
