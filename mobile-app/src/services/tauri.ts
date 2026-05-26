import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export interface LoginResponse {
  token: string;
  user_id: string;
}

export interface Device {
  id: string;
  name: string;
  online: boolean;
  bound_at: string;
}

export interface TaskResponse {
  task_id: string;
  status: "success" | "failure" | "pending";
  result: {
    message: string;
    data?: string;
  };
  screenshot_base64?: string;
}

export interface BindResponse {
  success: boolean;
  device_id?: string;
  message: string;
}

export async function loginApi(
  serverUrl: string,
  username: string,
  password: string
): Promise<LoginResponse> {
  const result = await invoke<string>("login", {
    serverUrl,
    username,
    password,
  });
  return JSON.parse(result);
}

export async function registerApi(
  serverUrl: string,
  username: string,
  password: string
): Promise<LoginResponse> {
  const result = await invoke<string>("register", {
    serverUrl,
    username,
    password,
  });
  return JSON.parse(result);
}

export async function getDevices(
  serverUrl: string,
  token: string
): Promise<Device[]> {
  const result = await invoke<string>("get_devices", { serverUrl, token });
  return JSON.parse(result);
}

export async function bindDevice(
  serverUrl: string,
  token: string,
  pairingCode: string
): Promise<BindResponse> {
  const result = await invoke<string>("bind_device", {
    serverUrl,
    token,
    pairingCode,
  });
  return JSON.parse(result);
}

export async function connectWs(
  serverUrl: string,
  userToken: string
): Promise<string> {
  return invoke<string>("connect_ws", { serverUrl, userToken });
}

export async function sendCommand(
  targetPcId: string,
  message: string
): Promise<void> {
  return invoke<void>("send_command", { targetPcId, message });
}

export function onTaskResponse(
  callback: (response: TaskResponse) => void
): Promise<UnlistenFn> {
  return listen<string>("task-response", (event) => {
    callback(JSON.parse(event.payload));
  });
}

export function onDeviceStatus(
  callback: (info: { device_id: string; online: boolean }) => void
): Promise<UnlistenFn> {
  return listen<string>("device-status", (event) => {
    callback(JSON.parse(event.payload));
  });
}

export function onWsError(callback: (error: string) => void): Promise<UnlistenFn> {
  return listen<string>("ws-error", (event) => {
    callback(event.payload);
  });
}

export function onWsDisconnected(callback: () => void): Promise<UnlistenFn> {
  return listen("ws-disconnected", callback);
}
