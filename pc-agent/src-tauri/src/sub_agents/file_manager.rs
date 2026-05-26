use shared::{TaskRequest, TaskResponse};
use std::path::Path;

/// Dangerous system paths that should not be modified.
const BLOCKED_PATHS: &[&str] = &[
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "/bin",
    "/sbin",
    "/usr",
    "/etc",
    "/System",
];

/// Execute file management tasks.
pub fn execute(task: &TaskRequest) -> TaskResponse {
    match task.action.as_str() {
        "list" => list_directory(task),
        "copy" => copy_file(task),
        "move" => move_file(task),
        "delete" => delete_file(task),
        "create_file" => create_file(task),
        "create_dir" => create_dir(task),
        "info" => file_info(task),
        _ => TaskResponse::failure(&task.task_id, &format!("Unknown file action: {}", task.action)),
    }
}

fn is_blocked(path: &str) -> bool {
    let p = Path::new(path);
    for blocked in BLOCKED_PATHS {
        if p.starts_with(blocked) {
            return true;
        }
    }
    false
}

fn list_directory(task: &TaskRequest) -> TaskResponse {
    let path = match task.parameters.get("path") {
        Some(p) => p,
        None => return TaskResponse::failure(&task.task_id, "Missing 'path' parameter"),
    };

    match std::fs::read_dir(path) {
        Ok(entries) => {
            let mut items = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let size = if !is_dir {
                    entry.metadata().map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };
                items.push(format!(
                    "{}{} [{} bytes]",
                    name,
                    if is_dir { "/" } else { "" },
                    size
                ));
            }
            let listing = items.join("\n");
            TaskResponse::success(&task.task_id, &format!("Directory listing:\n{}", listing))
                .with_data(&listing)
        }
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Cannot read directory: {}", e)),
    }
}

fn copy_file(task: &TaskRequest) -> TaskResponse {
    let src = match task.parameters.get("source") {
        Some(s) => s,
        None => return TaskResponse::failure(&task.task_id, "Missing 'source' parameter"),
    };
    let dst = match task.parameters.get("target") {
        Some(d) => d,
        None => return TaskResponse::failure(&task.task_id, "Missing 'target' parameter"),
    };

    match std::fs::copy(src, dst) {
        Ok(_) => TaskResponse::success(&task.task_id, &format!("Copied {} → {}", src, dst)),
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Copy failed: {}", e)),
    }
}

fn move_file(task: &TaskRequest) -> TaskResponse {
    let src = match task.parameters.get("source") {
        Some(s) => s,
        None => return TaskResponse::failure(&task.task_id, "Missing 'source' parameter"),
    };
    let dst = match task.parameters.get("target") {
        Some(d) => d,
        None => return TaskResponse::failure(&task.task_id, "Missing 'target' parameter"),
    };

    match std::fs::rename(src, dst) {
        Ok(_) => TaskResponse::success(&task.task_id, &format!("Moved {} → {}", src, dst)),
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Move failed: {}", e)),
    }
}

fn delete_file(task: &TaskRequest) -> TaskResponse {
    let path = match task.parameters.get("path") {
        Some(p) => p,
        None => return TaskResponse::failure(&task.task_id, "Missing 'path' parameter"),
    };

    if is_blocked(path) {
        return TaskResponse::failure(&task.task_id, &format!("Cannot delete system path: {}", path));
    }

    let p = Path::new(path);
    let result = if p.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };

    match result {
        Ok(_) => TaskResponse::success(&task.task_id, &format!("Deleted: {}", path)),
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Delete failed: {}", e)),
    }
}

fn create_file(task: &TaskRequest) -> TaskResponse {
    let path = match task.parameters.get("path") {
        Some(p) => p,
        None => return TaskResponse::failure(&task.task_id, "Missing 'path' parameter"),
    };
    let content = task.parameters.get("content").map(|s| s.as_str()).unwrap_or("");

    match std::fs::write(path, content) {
        Ok(_) => TaskResponse::success(&task.task_id, &format!("Created file: {}", path)),
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Create file failed: {}", e)),
    }
}

fn create_dir(task: &TaskRequest) -> TaskResponse {
    let path = match task.parameters.get("path") {
        Some(p) => p,
        None => return TaskResponse::failure(&task.task_id, "Missing 'path' parameter"),
    };

    match std::fs::create_dir_all(path) {
        Ok(_) => TaskResponse::success(&task.task_id, &format!("Created directory: {}", path)),
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Create directory failed: {}", e)),
    }
}

fn file_info(task: &TaskRequest) -> TaskResponse {
    let path = match task.parameters.get("path") {
        Some(p) => p,
        None => return TaskResponse::failure(&task.task_id, "Missing 'path' parameter"),
    };

    match std::fs::metadata(path) {
        Ok(meta) => {
            let info = format!(
                "Path: {}\nType: {}\nSize: {} bytes\nReadonly: {}\nModified: {:?}",
                path,
                if meta.is_dir() { "Directory" } else { "File" },
                meta.len(),
                meta.permissions().readonly(),
                meta.modified().ok(),
            );
            TaskResponse::success(&task.task_id, &info).with_data(&info)
        }
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Cannot read metadata: {}", e)),
    }
}
