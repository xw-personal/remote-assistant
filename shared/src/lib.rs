use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Task Protocol ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_id: String,
    pub task_type: TaskType,
    pub action: String,
    pub parameters: HashMap<String, String>,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub status: TaskStatus,
    pub result: TaskResult,
    pub screenshot_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub message: String,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    FileManagement,
    SystemControl,
    Simulation,
    Document,
    Browser,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Success,
    Failure,
    Pending,
}

// ── WebSocket Messages ─────────────────────────────────────────────────────

/// Envelope for all WebSocket communication between components.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    /// PC agent registers with relay server
    PcBind {
        device_id: String,
        token: String,
    },
    /// Mobile app registers with relay server
    MobileBind {
        user_token: String,
    },
    /// Mobile sends a command to a specific PC
    Command {
        target_pc_id: String,
        message: String,
    },
    /// PC sends a result back (relay routes to originating mobile)
    CommandResult {
        task_response: TaskResponse,
    },
    /// Heartbeat keepalive
    Heartbeat,
    /// Heartbeat acknowledgement
    HeartbeatAck,
    /// Task request (relay → PC)
    TaskRequest(TaskRequest),
    /// Task response (PC → relay → mobile)
    TaskResponse(TaskResponse),
    /// Error message
    Error { code: u16, message: String },
    /// Acknowledgement for bind
    BindAck {
        success: bool,
        device_id: Option<String>,
        message: String,
    },
    /// Device status update broadcast
    DeviceStatus {
        device_id: String,
        online: bool,
    },
}

// ── API Models ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindRequest {
    pub pairing_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindResponse {
    pub success: bool,
    pub device_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub online: bool,
    pub bound_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceListResponse {
    pub devices: Vec<Device>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingCodeResponse {
    pub pairing_code: String,
    pub expires_in: u64,
}

// ── Helpers ────────────────────────────────────────────────────────────────

impl TaskRequest {
    pub fn new(task_type: TaskType, action: &str, parameters: HashMap<String, String>) -> Self {
        Self {
            task_id: uuid_v4(),
            task_type,
            action: action.to_string(),
            parameters,
            context: String::new(),
        }
    }

    pub fn with_context(mut self, context: &str) -> Self {
        self.context = context.to_string();
        self
    }
}

impl TaskResponse {
    pub fn success(task_id: &str, message: &str) -> Self {
        Self {
            task_id: task_id.to_string(),
            status: TaskStatus::Success,
            result: TaskResult {
                message: message.to_string(),
                data: None,
            },
            screenshot_base64: None,
        }
    }

    pub fn failure(task_id: &str, message: &str) -> Self {
        Self {
            task_id: task_id.to_string(),
            status: TaskStatus::Failure,
            result: TaskResult {
                message: message.to_string(),
                data: None,
            },
            screenshot_base64: None,
        }
    }

    pub fn with_screenshot(mut self, screenshot: String) -> Self {
        self.screenshot_base64 = Some(screenshot);
        self
    }

    pub fn with_data(mut self, data: &str) -> Self {
        self.result.data = Some(data.to_string());
        self
    }
}

/// Simple UUID v4 generator (no external dependency).
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let ts = now.as_nanos();
    // Use timestamp + random for reasonably unique IDs
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (ts >> 96) as u32 & 0xFFFF_FFFF,
        (ts >> 80) as u16 & 0xFFFF,
        (ts >> 64) as u16 & 0x0FFF,
        (ts >> 48) as u16 & 0xFFFF,
        ts & 0x0000_FFFF_FFFF_FFFF,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_request_serialization() {
        let mut params = HashMap::new();
        params.insert("url".to_string(), "https://example.com".to_string());
        let req = TaskRequest::new(TaskType::Browser, "open_url", params);
        let json = serde_json::to_string(&req).unwrap();
        let back: TaskRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.action, back.action);
        assert_eq!(req.task_type as u8, back.task_type as u8);
    }

    #[test]
    fn ws_message_roundtrip() {
        let msg = WsMessage::Heartbeat;
        let json = serde_json::to_string(&msg).unwrap();
        let back: WsMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, WsMessage::Heartbeat));
    }

    #[test]
    fn task_response_builder() {
        let resp = TaskResponse::success("abc-123", "opened browser")
            .with_screenshot("base64data".to_string())
            .with_data("extra");
        assert_eq!(resp.status, TaskStatus::Success);
        assert!(resp.screenshot_base64.is_some());
        assert_eq!(resp.result.data.as_deref(), Some("extra"));
    }
}
