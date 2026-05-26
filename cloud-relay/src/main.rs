mod auth;
mod db;
mod ws;

use axum::{
    extract::{Json, State, WebSocketUpgrade},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use shared::*;
use ws::{AppState, SharedState};
use std::collections::HashMap;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db = db::init_db("pc_butler.db").expect("Failed to initialize database");
    let state: SharedState = std::sync::Arc::new(AppState::new(db));

    let app = Router::new()
        .route("/api/register", post(register_handler))
        .route("/api/login", post(login_handler))
        .route("/api/devices", get(devices_handler))
        .route("/api/bind", post(bind_handler))
        .route("/api/generate_code", post(generate_code_handler))
        .route("/ws/pc", get(ws_pc_handler))
        .route("/ws/mobile", get(ws_mobile_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9800").await.unwrap();
    info!("PC Butler relay server listening on :9800");
    axum::serve(listener, app).await.unwrap();
}

/// Extract Bearer token from Authorization header.
fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

// ── HTTP Handlers ──────────────────────────────────────────────────────────

async fn register_handler(
    State(state): State<SharedState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let user_id = uuid_simple();
    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST).unwrap();

    match db::create_user(&state.db, &user_id, &req.username, &password_hash) {
        Ok(_) => {
            let token = auth::generate_token(&user_id, &req.username).unwrap();
            (StatusCode::CREATED, Json(LoginResponse { token, user_id }))
        }
        Err(e) => {
            tracing::error!("Register failed: {}", e);
            (
                StatusCode::CONFLICT,
                Json(LoginResponse {
                    token: String::new(),
                    user_id: String::new(),
                }),
            )
        }
    }
}

async fn login_handler(
    State(state): State<SharedState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match db::get_user_by_username(&state.db, &req.username) {
        Ok(Some((user_id, username, hash))) => {
            if bcrypt::verify(&req.password, &hash).unwrap_or(false) {
                let token = auth::generate_token(&user_id, &username).unwrap();
                (StatusCode::OK, Json(LoginResponse { token, user_id }))
            } else {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(LoginResponse {
                        token: String::new(),
                        user_id: String::new(),
                    }),
                )
            }
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(LoginResponse {
                token: String::new(),
                user_id: String::new(),
            }),
        ),
    }
}

async fn devices_handler(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let claims = match extract_token(&headers)
        .and_then(|t| auth::validate_token(&t).ok())
    {
        Some(c) => c,
        None => return (StatusCode::UNAUTHORIZED, Json(DeviceListResponse { devices: vec![] })),
    };

    match db::get_user_devices(&state.db, &claims.sub) {
        Ok(raw_devices) => {
            let devices = raw_devices
                .into_iter()
                .map(|(id, name, bound_at)| {
                    let online = state.pcs.contains_key(&id);
                    Device {
                        id,
                        name,
                        online,
                        bound_at,
                    }
                })
                .collect();
            (StatusCode::OK, Json(DeviceListResponse { devices }))
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(DeviceListResponse { devices: vec![] })),
    }
}

/// PC calls this to generate a pairing code for mobile to bind.
async fn generate_code_handler(
    State(state): State<SharedState>,
    Json(body): Json<HashMap<String, String>>,
) -> impl IntoResponse {
    let device_id = match body.get("device_id") {
        Some(id) => id.clone(),
        None => return (StatusCode::BAD_REQUEST, Json(PairingCodeResponse { pairing_code: String::new(), expires_in: 0 })),
    };

    let code = auth::generate_pairing_code();
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 600;
    let expires_str = format!("{}", expires);

    match db::store_pairing_code(&state.db, &code, &device_id, &expires_str) {
        Ok(_) => (
            StatusCode::OK,
            Json(PairingCodeResponse {
                pairing_code: code,
                expires_in: 600,
            }),
        ),
        Err(e) => {
            tracing::error!("Failed to store pairing code: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PairingCodeResponse { pairing_code: String::new(), expires_in: 0 }),
            )
        }
    }
}

/// Mobile calls this with a pairing code to bind a PC device.
async fn bind_handler(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<BindRequest>,
) -> impl IntoResponse {
    let claims = match extract_token(&headers)
        .and_then(|t| auth::validate_token(&t).ok())
    {
        Some(c) => c,
        None => return (StatusCode::UNAUTHORIZED, Json(BindResponse { success: false, device_id: None, message: "Unauthorized".to_string() })),
    };

    match db::verify_pairing_code(&state.db, &req.pairing_code) {
        Ok(Some(device_id)) => {
            match db::bind_device(&state.db, &claims.sub, &device_id) {
                Ok(_) => (
                    StatusCode::OK,
                    Json(BindResponse {
                        success: true,
                        device_id: Some(device_id),
                        message: "Device bound successfully".to_string(),
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(BindResponse {
                        success: false,
                        device_id: None,
                        message: format!("Bind failed: {}", e),
                    }),
                ),
            }
        }
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(BindResponse {
                success: false,
                device_id: None,
                message: "Invalid or expired pairing code".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(BindResponse {
                success: false,
                device_id: None,
                message: format!("Error: {}", e),
            }),
        ),
    }
}

// ── WebSocket Handlers ─────────────────────────────────────────────────────

async fn ws_pc_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> Response {
    ws.on_upgrade(move |socket| ws::handle_pc_socket(socket, state))
}

async fn ws_mobile_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> Response {
    ws.on_upgrade(move |socket| ws::handle_mobile_socket(socket, state))
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:016x}", now.as_nanos() & 0xFFFF_FFFF_FFFF_FFFF)
}
