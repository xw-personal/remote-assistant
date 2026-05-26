import { useState, useEffect, useCallback } from "react";
import Login from "./components/Login";
import DeviceList from "./components/DeviceList";
import ChatView from "./components/ChatView";
import BindDevice from "./components/BindDevice";
import {
  loginApi,
  registerApi,
  getDevices,
  bindDevice,
  connectWs,
  sendCommand,
  onTaskResponse,
  onDeviceStatus,
  Device,
  TaskResponse,
} from "./services/tauri";
import "./App.css";

type Screen = "login" | "devices" | "bind" | "chat";

interface Message {
  id: string;
  role: "user" | "bot" | "system";
  content: string;
  screenshot?: string;
  status?: "success" | "error" | "loading";
  timestamp: number;
}

function App() {
  const [screen, setScreen] = useState<Screen>("login");
  const [serverUrl, setServerUrl] = useState("http://localhost:9800");
  const [token, setToken] = useState("");
  const [userId, setUserId] = useState("");
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<Device | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(false);
  const [wsConnected, setWsConnected] = useState(false);

  // Listen for task responses from PC
  useEffect(() => {
    const setup = async () => {
      await onTaskResponse((response: TaskResponse) => {
        setMessages((prev) => {
          // Update the last loading message
          const updated = [...prev];
          for (let i = updated.length - 1; i >= 0; i--) {
            if (updated[i].status === "loading") {
              updated[i] = {
                ...updated[i],
                content: response.result.message,
                screenshot: response.screenshot_base64
                  ? `data:image/png;base64,${response.screenshot_base64}`
                  : undefined,
                status:
                  response.status === "success" ? "success" : "error",
              };
              break;
            }
          }
          return updated;
        });
        setLoading(false);
      });

      await onDeviceStatus((info) => {
        setDevices((prev) =>
          prev.map((d) =>
            d.id === info.device_id ? { ...d, online: info.online } : d
          )
        );
      });
    };
    setup();
  }, []);

  const handleLogin = useCallback(
    async (url: string, username: string, password: string) => {
      const resp = await loginApi(url, username, password);
      setServerUrl(url);
      setToken(resp.token);
      setUserId(resp.user_id);

      // Connect WebSocket
      await connectWs(url, resp.token);
      setWsConnected(true);

      // Fetch devices
      const devs = await getDevices(url, resp.token);
      setDevices(devs);
      setScreen("devices");
    },
    []
  );

  const handleRegister = useCallback(
    async (url: string, username: string, password: string) => {
      const resp = await registerApi(url, username, password);
      setServerUrl(url);
      setToken(resp.token);
      setUserId(resp.user_id);

      await connectWs(url, resp.token);
      setWsConnected(true);

      setDevices([]);
      setScreen("devices");
    },
    []
  );

  const handleRefreshDevices = useCallback(async () => {
    try {
      const devs = await getDevices(serverUrl, token);
      setDevices(devs);
    } catch (e) {
      console.error("Refresh failed:", e);
    }
  }, [serverUrl, token]);

  const handleBind = useCallback(
    async (code: string) => {
      const resp = await bindDevice(serverUrl, token, code);
      if (resp.success) {
        await handleRefreshDevices();
        setScreen("devices");
      } else {
        throw new Error(resp.message);
      }
    },
    [serverUrl, token, handleRefreshDevices]
  );

  const handleSelectDevice = useCallback((device: Device) => {
    setSelectedDevice(device);
    setMessages([
      {
        id: "welcome",
        role: "bot",
        content: `已连接到 ${device.name}。输入指令开始控制电脑。\n\n示例指令：\n• 打开哔哩哔哩\n• 截个图\n• 查看系统信息\n• 打开记事本`,
        timestamp: Date.now(),
      },
    ]);
    setScreen("chat");
  }, []);

  const handleSendMessage = useCallback(
    async (text: string) => {
      if (!selectedDevice) return;

      const userMsg: Message = {
        id: `${Date.now()}-user`,
        role: "user",
        content: text,
        timestamp: Date.now(),
      };

      const loadingMsg: Message = {
        id: `${Date.now()}-loading`,
        role: "bot",
        content: "正在执行...",
        status: "loading",
        timestamp: Date.now() + 1,
      };

      setMessages((prev) => [...prev, userMsg, loadingMsg]);
      setLoading(true);

      try {
        await sendCommand(selectedDevice.id, text);
      } catch (err: any) {
        setMessages((prev) => {
          const updated = [...prev];
          const last = updated[updated.length - 1];
          if (last.status === "loading") {
            last.content = `发送失败: ${err}`;
            last.status = "error";
          }
          return updated;
        });
        setLoading(false);
      }
    },
    [selectedDevice]
  );

  return (
    <div className="app">
      {screen === "login" && (
        <Login
          onLogin={(url, t, uid) => {
            setServerUrl(url);
            setToken(t);
            setUserId(uid);
            setScreen("devices");
          }}
          onLoginSubmit={handleLogin}
          onRegister={handleRegister}
        />
      )}

      {screen === "devices" && (
        <DeviceList
          devices={devices}
          onSelect={handleSelectDevice}
          onBindClick={() => setScreen("bind")}
          onRefresh={handleRefreshDevices}
        />
      )}

      {screen === "bind" && (
        <BindDevice
          onBind={handleBind}
          onCancel={() => setScreen("devices")}
        />
      )}

      {screen === "chat" && selectedDevice && (
        <ChatView
          deviceName={selectedDevice.name}
          messages={messages}
          onSend={handleSendMessage}
          onBack={() => {
            setScreen("devices");
            setSelectedDevice(null);
          }}
          loading={loading}
        />
      )}
    </div>
  );
}

export default App;
