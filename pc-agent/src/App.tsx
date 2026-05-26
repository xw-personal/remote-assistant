import { useState, useRef, useEffect, useCallback } from "react";
import { parseUserCommand, LLMConfig, TaskJson } from "./services/llm";
import {
  executeTask,
  connectToServer,
  onRemoteCommand,
  onWsDisconnected,
  TaskResponse,
} from "./services/tauri";

interface Message {
  id: string;
  role: "user" | "bot" | "system";
  content: string;
  screenshot?: string;
  status?: "success" | "error" | "loading";
  timestamp: number;
}

const DEFAULT_LLM_CONFIG: LLMConfig = {
  apiBaseUrl: "https://api.openai.com/v1",
  apiKey: "",
  model: "gpt-4o-mini",
};

export default function App() {
  const [messages, setMessages] = useState<Message[]>([
    {
      id: "welcome",
      role: "bot",
      content:
        "PC Butler 已启动。我可以帮您控制电脑 — 打开网页、管理文件、查看系统状态、模拟操作等。\n\n请配置LLM API后输入指令，例如：\n• 打开哔哩哔哩\n• 截个图\n• 查看系统信息\n• 打开记事本",
      timestamp: Date.now(),
    },
  ]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [connected, setConnected] = useState(false);
  const [llmConfig, setLlmConfig] = useState<LLMConfig>(DEFAULT_LLM_CONFIG);
  const [serverUrl, setServerUrl] = useState("ws://localhost:9800");
  const [deviceId, setDeviceId] = useState("");
  const [deviceToken, setDeviceToken] = useState("");
  const [showSettings, setShowSettings] = useState(false);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Listen for remote commands from relay
  useEffect(() => {
    const setup = async () => {
      await onRemoteCommand((message) => {
        addMessage("user", `[远程] ${message}`, undefined, "loading");
        processCommand(message, true);
      });

      await onWsDisconnected(() => {
        setConnected(false);
        addMessage("system", "与服务器的连接已断开");
      });
    };
    setup();
  }, []);

  const addMessage = useCallback(
    (
      role: "user" | "bot" | "system",
      content: string,
      screenshot?: string,
      status?: "success" | "error" | "loading"
    ) => {
      const msg: Message = {
        id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
        role,
        content,
        screenshot,
        status,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, msg]);
      return msg.id;
    },
    []
  );

  const updateMessage = useCallback(
    (id: string, updates: Partial<Message>) => {
      setMessages((prev) =>
        prev.map((m) => (m.id === id ? { ...m, ...updates } : m))
      );
    },
    []
  );

  const processCommand = async (text: string, isRemote = false) => {
    if (!llmConfig.apiKey) {
      addMessage(
        "bot",
        "请先在设置中配置 LLM API Key 和 API 地址。",
        undefined,
        "error"
      );
      return;
    }

    const loadingId = addMessage("bot", "正在分析指令...", undefined, "loading");
    setLoading(true);

    try {
      // Step 1: Parse user intent with LLM
      const task: TaskJson = await parseUserCommand(text, llmConfig);
      updateMessage(loadingId, {
        content: `已解析指令: ${task.task_type} / ${task.action}\n正在执行...`,
      });

      // Step 2: Execute the task
      const response: TaskResponse = await executeTask(task);

      // Step 3: Display result
      const statusEmoji = response.status === "success" ? "" : "❌";
      updateMessage(loadingId, {
        content: `${statusEmoji} ${response.result.message}`,
        screenshot: response.screenshot_base64
          ? `data:image/png;base64,${response.screenshot_base64}`
          : undefined,
        status: response.status === "success" ? "success" : "error",
      });
    } catch (err: any) {
      updateMessage(loadingId, {
        content: `执行出错: ${err.message || err}`,
        status: "error",
      });
    } finally {
      setLoading(false);
    }
  };

  const handleSend = async () => {
    const text = input.trim();
    if (!text || loading) return;

    setInput("");
    addMessage("user", text);
    await processCommand(text);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleConnect = async () => {
    try {
      await connectToServer(serverUrl, deviceId, deviceToken);
      setConnected(true);
      addMessage("system", "已连接到服务器");
    } catch (err: any) {
      addMessage("system", `连接失败: ${err}`);
    }
  };

  return (
    <div className="app-container">
      {/* Sidebar */}
      <div className="sidebar">
        <div className="sidebar-header">
          <h2>PC Butler</h2>
          <div className="subtitle">对话式电脑管家</div>
        </div>

        {/* Connection */}
        <div className="connection-form">
          <input
            placeholder="服务器地址 (ws://...)"
            value={serverUrl}
            onChange={(e) => setServerUrl(e.target.value)}
          />
          <input
            placeholder="设备 ID"
            value={deviceId}
            onChange={(e) => setDeviceId(e.target.value)}
          />
          <input
            placeholder="设备 Token"
            value={deviceToken}
            onChange={(e) => setDeviceToken(e.target.value)}
            type="password"
          />
          <button onClick={handleConnect} disabled={connected}>
            {connected ? "已连接" : "连接服务器"}
          </button>
        </div>

        <div className="status-indicator">
          <div className={`status-dot ${connected ? "connected" : ""}`} />
          {connected ? "在线" : "离线"}
        </div>

        {/* LLM Settings */}
        <div className="settings-panel">
          <h4
            style={{ cursor: "pointer" }}
            onClick={() => setShowSettings(!showSettings)}
          >
            {showSettings ? "▼" : "▶"} LLM 配置
          </h4>
          {showSettings && (
            <div className="llm-config">
              <label>API 地址</label>
              <input
                placeholder="https://api.openai.com/v1"
                value={llmConfig.apiBaseUrl}
                onChange={(e) =>
                  setLlmConfig({ ...llmConfig, apiBaseUrl: e.target.value })
                }
              />
              <label>API Key</label>
              <input
                placeholder="sk-..."
                type="password"
                value={llmConfig.apiKey}
                onChange={(e) =>
                  setLlmConfig({ ...llmConfig, apiKey: e.target.value })
                }
              />
              <label>模型</label>
              <input
                placeholder="gpt-4o-mini"
                value={llmConfig.model}
                onChange={(e) =>
                  setLlmConfig({ ...llmConfig, model: e.target.value })
                }
              />
            </div>
          )}
        </div>
      </div>

      {/* Chat Area */}
      <div className="chat-area">
        <div className="chat-header">
          <h3>对话</h3>
          <span className="mode-badge">
            {llmConfig.model || "未配置模型"}
          </span>
        </div>

        <div className="messages-container">
          {messages.map((msg) => (
            <div key={msg.id} className={`message ${msg.role}`}>
              <div className="message-label">
                {msg.role === "user"
                  ? "你"
                  : msg.role === "system"
                  ? "系统"
                  : "PC Butler"}
              </div>
              <div
                className={`message-bubble ${
                  msg.status === "success"
                    ? "success"
                    : msg.status === "error"
                    ? "error"
                    : ""
                }`}
              >
                {msg.content}
                {msg.status === "loading" && (
                  <div className="loading">
                    <div className="loading-dot" />
                    <div className="loading-dot" />
                    <div className="loading-dot" />
                  </div>
                )}
              </div>
              {msg.screenshot && (
                <div className="message-screenshot">
                  <img
                    src={msg.screenshot}
                    alt="Screenshot"
                    onClick={() => window.open(msg.screenshot, "_blank")}
                  />
                </div>
              )}
            </div>
          ))}
          <div ref={messagesEndRef} />
        </div>

        <div className="input-area">
          <textarea
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="输入指令，例如：打开哔哩哔哩、截个图、查看系统信息..."
            rows={1}
            disabled={loading}
          />
          <button
            className="send-btn"
            onClick={handleSend}
            disabled={loading || !input.trim()}
          >
            {loading ? "执行中..." : "发送"}
          </button>
        </div>
      </div>
    </div>
  );
}
