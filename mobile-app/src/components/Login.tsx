import { useState } from "react";

interface Props {
  onLogin: (serverUrl: string, token: string, userId: string) => void;
  onRegister: (serverUrl: string, username: string, password: string) => Promise<void>;
  onLoginSubmit: (serverUrl: string, username: string, password: string) => Promise<void>;
}

export default function Login({ onLogin, onRegister, onLoginSubmit }: Props) {
  const [serverUrl, setServerUrl] = useState("http://localhost:9800");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [isRegister, setIsRegister] = useState(false);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);

    try {
      if (isRegister) {
        await onRegister(serverUrl, username, password);
      } else {
        await onLoginSubmit(serverUrl, username, password);
      }
    } catch (err: any) {
      setError(err.toString());
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="login-container">
      <div className="login-card">
        <h2>PC Butler</h2>
        <p className="login-subtitle">远程控制您的电脑</p>

        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label>服务器地址</label>
            <input
              type="text"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              placeholder="http://localhost:9800"
            />
          </div>

          <div className="form-group">
            <label>用户名</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="输入用户名"
              required
            />
          </div>

          <div className="form-group">
            <label>密码</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="输入密码"
              required
            />
          </div>

          {error && <div className="error-msg">{error}</div>}

          <button type="submit" className="login-btn" disabled={loading}>
            {loading ? "处理中..." : isRegister ? "注册" : "登录"}
          </button>
        </form>

        <button
          className="switch-btn"
          onClick={() => setIsRegister(!isRegister)}
        >
          {isRegister ? "已有账号？登录" : "没有账号？注册"}
        </button>
      </div>
    </div>
  );
}
