use shared::{TaskRequest, TaskResponse};

/// Execute browser-related tasks.
pub fn execute(task: &TaskRequest) -> TaskResponse {
    match task.action.as_str() {
        "open_url" => open_url(task),
        "screenshot" => screenshot(task),
        _ => TaskResponse::failure(&task.task_id, &format!("Unknown browser action: {}", task.action)),
    }
}

fn open_url(task: &TaskRequest) -> TaskResponse {
    let url = match task.parameters.get("url") {
        Some(u) => u.as_str(),
        None => return TaskResponse::failure(&task.task_id, "Missing 'url' parameter"),
    };

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/c", "start", "", url])
        .spawn();

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .arg(url)
        .spawn();

    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open")
        .arg(url)
        .spawn();

    match result {
        Ok(_) => {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            let screenshot = capture_screenshot_base64();
            TaskResponse::success(&task.task_id, &format!("Opened URL: {}", url))
                .with_screenshot(screenshot)
        }
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Failed to open URL: {}", e)),
    }
}

fn screenshot(task: &TaskRequest) -> TaskResponse {
    let screenshot = capture_screenshot_base64();
    if screenshot.is_empty() {
        TaskResponse::failure(&task.task_id, "Failed to capture screenshot")
    } else {
        TaskResponse::success(&task.task_id, "Screenshot captured")
            .with_screenshot(screenshot)
    }
}

pub fn capture_screenshot_base64() -> String {
    use base64::Engine;
    match screenshots::Screen::all() {
        Ok(screens) => {
            if let Some(screen) = screens.first() {
                match screen.capture() {
                    Ok(image) => {
                        // Encode to PNG
                        let width = image.width();
                        let height = image.height();
                        // ImageBuffer<Rgba<u8>> already has RGBA data
                        let raw = image.as_raw();

                        use std::io::Cursor;
                        let mut cursor = Cursor::new(Vec::new());
                        {
                            let mut encoder = png::Encoder::new(&mut cursor, width, height);
                            encoder.set_color(png::ColorType::Rgba);
                            encoder.set_depth(png::BitDepth::Eight);
                            if let Ok(mut writer) = encoder.write_header() {
                                let _ = writer.write_image_data(raw);
                            }
                        }
                        let png_data = cursor.into_inner();
                        base64::engine::general_purpose::STANDARD.encode(png_data)
                    }
                    Err(_) => String::new(),
                }
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}
