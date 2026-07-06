import React, { useState, useEffect } from 'react';
import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window';
import { Minus, Copy, Square, X } from 'lucide-react';
import './WindowControls.css';

export function WindowControls() {
  const appWindow = getCurrentWindow();
  
  const [isMaximized, setIsMaximized] = useState(false);
  const [originalSize, setOriginalSize] = useState<any>(null);
  const [originalPos, setOriginalPos] = useState<any>(null);

  useEffect(() => {
    // Check initial state
    appWindow.isMaximized().then(setIsMaximized);

    // Listen to resize events from OS
    const unlisten = appWindow.onResized(async () => {
      const max = await appWindow.isMaximized();
      setIsMaximized(max);
    });

    return () => {
      unlisten.then(f => f());
    };
  }, []);

  const handleMaximizeToggle = async () => {
    try {
      if (isMaximized) {
        // Khôi phục kích thước và vị trí cũ
        if (originalSize && originalPos) {
          await appWindow.setSize(originalSize);
          await appWindow.setPosition(originalPos);
        } else {
          await appWindow.unmaximize();
        }
        setIsMaximized(false);
      } else {
        // Lưu lại vị trí và kích thước hiện tại
        const size = await appWindow.outerSize();
        const pos = await appWindow.outerPosition();
        setOriginalSize(size);
        setOriginalPos(pos);

        // Tự tính toán theo workArea của màn hình (trừ đi thanh Taskbar)
        const monitor = await currentMonitor();
        if (monitor && monitor.workArea) {
          await appWindow.setPosition(monitor.workArea.position);
          await appWindow.setSize(monitor.workArea.size);
          setIsMaximized(true);
        } else {
          await appWindow.maximize(); // Fallback
          setIsMaximized(true);
        }
      }
    } catch (e) {
      console.error("Lỗi khi phóng to:", e);
      // Fallback API native
      await appWindow.toggleMaximize();
    }
  };

  return (
    <div className="window-controls-container">
      {/* Nút Thu nhỏ */}
      <button
        onClick={() => appWindow.minimize()}
        className="window-control-btn"
        title="Minimize"
      >
        <Minus size={12} strokeWidth={1.5} color="white" />
      </button>

      {/* Nút Phóng to / Thu nhỏ (Restore) */}
      <button
        onClick={handleMaximizeToggle}
        className="window-control-btn"
        title={isMaximized ? "Restore" : "Maximize"}
      >
        {isMaximized ? (
          <Copy size={10} strokeWidth={1.5} color="white" className="rotate-90" />
        ) : (
          <Square size={10} strokeWidth={1.5} color="white" />
        )}
      </button>

      {/* Nút Đóng */}
      <button
        onClick={() => appWindow.close()}
        className="window-control-btn close-btn"
        title="Close"
      >
        <X size={12} strokeWidth={1.5} color="white" />
      </button>
    </div>
  );
}


