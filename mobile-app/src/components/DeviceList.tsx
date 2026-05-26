import { Device } from "../services/tauri";

interface Props {
  devices: Device[];
  onSelect: (device: Device) => void;
  onBindClick: () => void;
  onRefresh: () => void;
}

export default function DeviceList({
  devices,
  onSelect,
  onBindClick,
  onRefresh,
}: Props) {
  return (
    <div className="device-list-container">
      <div className="device-list-header">
        <h2>我的设备</h2>
        <div className="header-actions">
          <button className="refresh-btn" onClick={onRefresh}>
            刷新
          </button>
          <button className="bind-btn" onClick={onBindClick}>
            + 绑定设备
          </button>
        </div>
      </div>

      {devices.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">💻</div>
          <p>还没有绑定任何设备</p>
          <p className="empty-hint">
            在PC端打开 PC Butler，获取配对码后点击"绑定设备"
          </p>
        </div>
      ) : (
        <div className="device-cards">
          {devices.map((device) => (
            <div
              key={device.id}
              className={`device-card ${device.online ? "online" : "offline"}`}
              onClick={() => device.online && onSelect(device)}
            >
              <div className="device-icon">
                {device.online ? "🖥️" : "💤"}
              </div>
              <div className="device-info">
                <div className="device-name">{device.name}</div>
                <div className="device-status">
                  <span
                    className={`status-dot ${device.online ? "online" : ""}`}
                  />
                  {device.online ? "在线" : "离线"}
                </div>
                <div className="device-id">{device.id}</div>
              </div>
              {!device.online && (
                <div className="device-disabled-hint">请先在PC端启动</div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
