import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export interface TaskResponse {
  task_id: string;
  status: "success" | "failure" | "pending";
  result: {
    message: string;
    data?: string;
  };
  screenshot_base64?: string;
}

/**
 * Execute a task on the Rust backend.
 */
export async function executeTask(taskJson: object): Promise<TaskResponse> {
  const result = await invoke<string>("execute_task", {
    taskJson: JSON.stringify(taskJson),
  });
  return JSON.parse(result) as TaskResponse;
}

/**
 * Open a URL in the system browser and get a screenshot.
 */
export async function openUrl(url: string): Promise<TaskResponse> {
  const result = await invoke<string>("open_url", { url });
  return JSON.parse(result) as TaskResponse;
}

/**
 * Take a screenshot of the current screen.
 */
export async function takeScreenshot(): Promise<string> {
  return invoke<string>("take_screenshot");
}

/**
 * Connect to the relay server.
 */
export async function connectToServer(
  serverUrl: string,
  deviceId: string,
  deviceToken: string
): Promise<string> {
  return invoke<string>("connect_to_server", {
    serverUrl,
    deviceId,
    deviceToken,
  });
}

/**
 * Get system information.
 */
export async function getSystemInfo(): Promise<TaskResponse> {
  const result = await invoke<string>("get_system_info");
  return JSON.parse(result) as TaskResponse;
}

/**
 * Listen for remote commands from the relay (forwarded from mobile).
 */
export function onRemoteCommand(callback: (message: string) => void): Promise<UnlistenFn> {
  return listen<string>("remote-command", (event) => {
    callback(event.payload);
  });
}

/**
 * Listen for WebSocket disconnection events.
 */
export function onWsDisconnected(callback: () => void): Promise<UnlistenFn> {
  return listen("ws-disconnected", () => {
    callback();
  });
}
