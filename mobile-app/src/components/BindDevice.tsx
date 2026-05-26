import { useState } from "react";

interface Props {
  onBind: (code: string) => Promise<void>;
  onCancel: () => void;
}

export default function BindDevice({ onBind, onCancel }: Props) {
  const [code, setCode] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (code.length !== 6) {
      setError("请输入6位配对码");
      return;
    }

    setError("");
    setLoading(true);
    try {
      await onBind(code);
    } catch (err: any) {
      setError(err.toString());
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="bind-container">
      <div className="bind-card">
        <h2>绑定设备</h2>
        <p className="bind-hint">
          在PC端 PC Butler 中获取6位配对码，然后输入下方
        </p>

        <form onSubmit={handleSubmit}>
          <div className="code-input">
            <input
              type="text"
              value={code}
              onChange={(e) => setCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
              placeholder="000000"
              maxLength={6}
              autoFocus
            />
          </div>

          {error && <div className="error-msg">{error}</div>}

          <div className="bind-actions">
            <button
              type="button"
              className="cancel-btn"
              onClick={onCancel}
              disabled={loading}
            >
              取消
            </button>
            <button
              type="submit"
              className="confirm-btn"
              disabled={loading || code.length !== 6}
            >
              {loading ? "绑定中..." : "确认绑定"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
