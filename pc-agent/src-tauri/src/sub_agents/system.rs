use shared::{TaskRequest, TaskResponse};
use sysinfo::{System, Disks};

/// Execute system control tasks.
pub fn execute(task: &TaskRequest) -> TaskResponse {
    match task.action.as_str() {
        "info" => get_system_info(task),
        "processes" => get_process_list(task),
        "kill_process" => kill_process(task),
        "shutdown" => shutdown(task),
        "restart" => restart(task),
        _ => TaskResponse::failure(&task.task_id, &format!("Unknown system action: {}", task.action)),
    }
}

fn get_system_info(task: &TaskRequest) -> TaskResponse {
    let mut sys = System::new_all();
    sys.refresh_all();
    let disks = Disks::new_with_refreshed_list();

    let info = format!(
        "OS: {} {}\nHostname: {}\nCPU: {} ({} cores)\nMemory: {:.1} GB / {:.1} GB ({:.0}%)\nDisk: {} drives",
        System::name().unwrap_or_default(),
        System::os_version().unwrap_or_default(),
        System::host_name().unwrap_or_default(),
        sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default(),
        sys.cpus().len(),
        sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0),
        sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0),
        (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0,
        disks.len(),
    );

    TaskResponse::success(&task.task_id, &info).with_data(&info)
}

fn get_process_list(task: &TaskRequest) -> TaskResponse {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut processes: Vec<_> = sys.processes().values().collect();
    processes.sort_by(|a, b| b.memory().cmp(&a.memory()));

    let top_n: Vec<String> = processes
        .into_iter()
        .take(20)
        .map(|p| {
            format!(
                "{} (PID: {}) - {:.1} MB",
                p.name().to_string_lossy(),
                p.pid(),
                p.memory() as f64 / (1024.0 * 1024.0)
            )
        })
        .collect();

    let listing = top_n.join("\n");
    TaskResponse::success(&task.task_id, &format!("Top processes by memory:\n{}", listing))
        .with_data(&listing)
}

fn kill_process(task: &TaskRequest) -> TaskResponse {
    let pid_str = match task.parameters.get("pid") {
        Some(p) => p,
        None => return TaskResponse::failure(&task.task_id, "Missing 'pid' parameter"),
    };

    let pid: u32 = match pid_str.parse() {
        Ok(p) => p,
        Err(_) => return TaskResponse::failure(&task.task_id, "Invalid PID"),
    };

    let mut sys = System::new_all();
    sys.refresh_all();

    if let Some(process) = sys.processes().get(&(pid as usize).into()) {
        if process.kill() {
            TaskResponse::success(&task.task_id, &format!("Killed process {} ({})", pid, process.name().to_string_lossy()))
        } else {
            TaskResponse::failure(&task.task_id, &format!("Failed to kill process {}", pid))
        }
    } else {
        TaskResponse::failure(&task.task_id, &format!("Process {} not found", pid))
    }
}

fn shutdown(task: &TaskRequest) -> TaskResponse {
    let confirmed = task.parameters.get("confirmed").map(|s| s.as_str()) == Some("true");
    if !confirmed {
        return TaskResponse::failure(&task.task_id, "Shutdown requires confirmation. Set 'confirmed' = 'true'");
    }

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("shutdown")
        .args(["/s", "/t", "0"])
        .spawn();

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("sudo")
        .args(["shutdown", "-h", "now"])
        .spawn();

    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("sudo")
        .args(["shutdown", "-h", "now"])
        .spawn();

    match result {
        Ok(_) => TaskResponse::success(&task.task_id, "Shutdown initiated"),
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Shutdown failed: {}", e)),
    }
}

fn restart(task: &TaskRequest) -> TaskResponse {
    let confirmed = task.parameters.get("confirmed").map(|s| s.as_str()) == Some("true");
    if !confirmed {
        return TaskResponse::failure(&task.task_id, "Restart requires confirmation. Set 'confirmed' = 'true'");
    }

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("shutdown")
        .args(["/r", "/t", "0"])
        .spawn();

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("sudo")
        .args(["shutdown", "-r", "now"])
        .spawn();

    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("sudo")
        .args(["shutdown", "-r", "now"])
        .spawn();

    match result {
        Ok(_) => TaskResponse::success(&task.task_id, "Restart initiated"),
        Err(e) => TaskResponse::failure(&task.task_id, &format!("Restart failed: {}", e)),
    }
}
