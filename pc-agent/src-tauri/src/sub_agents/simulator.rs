use shared::{TaskRequest, TaskResponse};
use enigo::{Keyboard, Mouse};

/// Execute simulation/automation tasks (keyboard, mouse, screen capture).
pub fn execute(task: &TaskRequest) -> TaskResponse {
    match task.action.as_str() {
        "click" => click(task),
        "double_click" => double_click(task),
        "right_click" => right_click(task),
        "type_text" => type_text(task),
        "hotkey" => hotkey(task),
        "screenshot" => screenshot(task),
        "open_app" => open_app(task),
        _ => TaskResponse::failure(&task.task_id, &format!("Unknown simulation action: {}", task.action)),
    }
}

fn click(task: &TaskRequest) -> TaskResponse {
    let x: i32 = match task.parameters.get("x").and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return TaskResponse::failure(&task.task_id, "Missing or invalid 'x' parameter"),
    };
    let y: i32 = match task.parameters.get("y").and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return TaskResponse::failure(&task.task_id, "Missing or invalid 'y' parameter"),
    };

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();
    let _ = enigo.move_mouse(x, y, enigo::Coordinate::Abs);
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = enigo.button(enigo::Button::Left, enigo::Direction::Click);

    let screenshot = super::browser::capture_screenshot_base64();
    TaskResponse::success(&task.task_id, &format!("Clicked at ({}, {})", x, y))
        .with_screenshot(screenshot)
}

fn double_click(task: &TaskRequest) -> TaskResponse {
    let x: i32 = match task.parameters.get("x").and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return TaskResponse::failure(&task.task_id, "Missing or invalid 'x' parameter"),
    };
    let y: i32 = match task.parameters.get("y").and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return TaskResponse::failure(&task.task_id, "Missing or invalid 'y' parameter"),
    };

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();
    let _ = enigo.move_mouse(x, y, enigo::Coordinate::Abs);
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = enigo.button(enigo::Button::Left, enigo::Direction::Click);
    std::thread::sleep(std::time::Duration::from_millis(50));
    let _ = enigo.button(enigo::Button::Left, enigo::Direction::Click);

    let screenshot = super::browser::capture_screenshot_base64();
    TaskResponse::success(&task.task_id, &format!("Double-clicked at ({}, {})", x, y))
        .with_screenshot(screenshot)
}

fn right_click(task: &TaskRequest) -> TaskResponse {
    let x: i32 = match task.parameters.get("x").and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return TaskResponse::failure(&task.task_id, "Missing or invalid 'x' parameter"),
    };
    let y: i32 = match task.parameters.get("y").and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return TaskResponse::failure(&task.task_id, "Missing or invalid 'y' parameter"),
    };

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();
    let _ = enigo.move_mouse(x, y, enigo::Coordinate::Abs);
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = enigo.button(enigo::Button::Right, enigo::Direction::Click);

    let screenshot = super::browser::capture_screenshot_base64();
    TaskResponse::success(&task.task_id, &format!("Right-clicked at ({}, {})", x, y))
        .with_screenshot(screenshot)
}

fn type_text(task: &TaskRequest) -> TaskResponse {
    let text = match task.parameters.get("text") {
        Some(t) => t,
        None => return TaskResponse::failure(&task.task_id, "Missing 'text' parameter"),
    };

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();
    let _ = enigo.text(text);

    let screenshot = super::browser::capture_screenshot_base64();
    TaskResponse::success(&task.task_id, &format!("Typed: {}", text))
        .with_screenshot(screenshot)
}

fn hotkey(task: &TaskRequest) -> TaskResponse {
    let keys = match task.parameters.get("keys") {
        Some(k) => k,
        None => return TaskResponse::failure(&task.task_id, "Missing 'keys' parameter"),
    };

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();

    // Parse key combo like "ctrl+c", "alt+f4", "ctrl+shift+s"
    let key_names: Vec<&str> = keys.split('+').collect();
    let mut held_keys = Vec::new();

    for name in &key_names {
        if let Some(key) = parse_key(name.trim()) {
            let _ = enigo.key(key, enigo::Direction::Press);
            held_keys.push(key);
        }
    }

    // Release in reverse order
    for key in held_keys.iter().rev() {
        let _ = enigo.key(*key, enigo::Direction::Release);
    }

    std::thread::sleep(std::time::Duration::from_millis(300));
    let screenshot = super::browser::capture_screenshot_base64();
    TaskResponse::success(&task.task_id, &format!("Pressed hotkey: {}", keys))
        .with_screenshot(screenshot)
}

fn screenshot(task: &TaskRequest) -> TaskResponse {
    let screenshot = super::browser::capture_screenshot_base64();
    if screenshot.is_empty() {
        TaskResponse::failure(&task.task_id, "Failed to capture screenshot")
    } else {
        TaskResponse::success(&task.task_id, "Screenshot captured")
            .with_screenshot(screenshot)
    }
}

fn open_app(task: &TaskRequest) -> TaskResponse {
    let app_name = match task.parameters.get("app_name") {
        Some(a) => a,
        None => return TaskResponse::failure(&task.task_id, "Missing 'app_name' parameter"),
    };

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/c", "start", "", app_name])
        .spawn();

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .args(["-a", app_name])
        .spawn();

    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("sh")
        .args(["-c", app_name])
        .spawn();

    match result {
        Ok(_) => {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            let screenshot = super::browser::capture_screenshot_base64();
            TaskResponse::success(&task.task_id, &format!("Opened app: {}", app_name))
                .with_screenshot(screenshot)
        }
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Failed to open app: {}", e)),
    }
}

fn parse_key(name: &str) -> Option<enigo::Key> {
    match name.to_lowercase().as_str() {
        "ctrl" | "control" => Some(enigo::Key::Control),
        "alt" => Some(enigo::Key::Alt),
        "shift" => Some(enigo::Key::Shift),
        "meta" | "super" | "win" | "cmd" => Some(enigo::Key::Meta),
        "enter" | "return" => Some(enigo::Key::Return),
        "tab" => Some(enigo::Key::Tab),
        "space" => Some(enigo::Key::Space),
        "backspace" => Some(enigo::Key::Backspace),
        "delete" | "del" => Some(enigo::Key::Delete),
        "escape" | "esc" => Some(enigo::Key::Escape),
        "up" => Some(enigo::Key::UpArrow),
        "down" => Some(enigo::Key::DownArrow),
        "left" => Some(enigo::Key::LeftArrow),
        "right" => Some(enigo::Key::RightArrow),
        "f1" => Some(enigo::Key::F1),
        "f2" => Some(enigo::Key::F2),
        "f3" => Some(enigo::Key::F3),
        "f4" => Some(enigo::Key::F4),
        "f5" => Some(enigo::Key::F5),
        "f6" => Some(enigo::Key::F6),
        "f7" => Some(enigo::Key::F7),
        "f8" => Some(enigo::Key::F8),
        "f9" => Some(enigo::Key::F9),
        "f10" => Some(enigo::Key::F10),
        "f11" => Some(enigo::Key::F11),
        "f12" => Some(enigo::Key::F12),
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            Some(enigo::Key::Unicode(ch))
        }
        _ => None,
    }
}
