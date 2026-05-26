import { useState, useRef, useEffect } from "react";

interface Message {
  id: string;
  role: "user" | "bot" | "system";
  content: string;
  screenshot?: string;
  status?: "success" | "error" | "loading";
  timestamp: number;
}

interface Props {
  deviceName: string;
  messages: Message[];
  onSend: (message: string) => void;
  onBack: () => void;
  loading: boolean;
}

export default function ChatView({
  deviceName,
  messages,
  onSend,
  onBack,
  loading,
}: Props) {
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleSend = () => {
    const text = input.trim();
    if (!text || loading) return;
    setInput("");
    onSend(text);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="chat-container">
      <div className="chat-header">
        <button className="back-btn" onClick={onBack}>
          ← 返回
        </button>
        <div className="chat-title">
          <span className="device-name">{deviceName}</span>
          <span className="online-badge">在线</span>
        </div>
      </div>

      <div className="messages-area">
        {messages.map((msg) => (
          <div key={msg.id} className={`msg ${msg.role}`}>
            <div className="msg-label">
              {msg.role === "user" ? "我" : msg.role === "system" ? "系统" : "PC"}
            </div>
            <div
              className={`msg-bubble ${
                msg.status === "success"
                  ? "success"
                  : msg.status === "error"
                  ? "error"
                  : ""
              }`}
            >
              {msg.content}
              {msg.status === "loading" && (
                <div className="loading-dots">
                  <span />
                  <span />
                  <span />
                </div>
              )}
            </div>
            {msg.screenshot && (
              <div className="msg-screenshot">
                <img src={msg.screenshot} alt="执行结果截图" />
              </div>
            )}
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>

      <div className="input-bar">
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="输入指令..."
          rows={1}
          disabled={loading}
        />
        <button
          className="send-btn"
          onClick={handleSend}
          disabled={loading || !input.trim()}
        >
          发送
        </button>
      </div>
    </div>
  );
}
