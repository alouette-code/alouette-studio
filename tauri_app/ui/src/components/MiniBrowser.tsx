import React, { useState, useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { Webview } from "@tauri-apps/api/webview";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { RotateCcw, Home, X } from "lucide-react";

export default function MiniBrowser() {
  const [url, setUrl] = useState("https://google.com");
  const [inputUrl, setInputUrl] = useState("https://google.com");
  const containerRef = useRef<HTMLDivElement>(null);
  const webviewCreated = useRef(false);

  useEffect(() => {
    if (webviewCreated.current) return;
    webviewCreated.current = true;
    
    const appWindow = getCurrentWindow();
    let webview: Webview;
    let isMounted = true;
    
    const initWebview = async () => {
      webview = new Webview(appWindow, 'browser-content', {
        url: url,
        x: 0,
        y: 50, // Height of the top bar
        width: window.innerWidth,
        height: window.innerHeight - 50,
      });

      const handleResize = () => {
        if (webview) {
          webview.setSize(new LogicalSize(window.innerWidth, window.innerHeight - 50)).catch(console.error);
        }
      };

      window.addEventListener('resize', handleResize);
      
      return () => {
        window.removeEventListener('resize', handleResize);
        if (webview) {
          webview.close().catch(console.error);
        }
      };
    };
    
    let cleanup: any;
    initWebview().then(c => {
      if (isMounted) cleanup = c;
      else if (c) c(); // cleanup immediately if unmounted
    });

    return () => {
      isMounted = false;
      if (cleanup) cleanup();
    };
  }, []);

  const handleNavigate = async (e: React.FormEvent) => {
    e.preventDefault();
    let finalUrl = inputUrl.trim();
    
    // Kiểm tra xem là URL hay từ khóa tìm kiếm
    const isUrl = /^(https?:\/\/)?[\w.-]+\.[a-z]{2,}(\/.*)?$/i.test(finalUrl) || finalUrl.startsWith('http://') || finalUrl.startsWith('https://');

    if (isUrl) {
      if (!finalUrl.startsWith("http://") && !finalUrl.startsWith("https://")) {
        finalUrl = "https://" + finalUrl;
      }
    } else {
      // Nếu là từ khóa thì chuyển thành tìm kiếm Google
      finalUrl = `https://www.google.com/search?q=${encodeURIComponent(finalUrl)}`;
    }
    
    setUrl(finalUrl);
    setInputUrl(finalUrl);
    
    try {
      await invoke('navigate_webview', { url: finalUrl });
    } catch (err) {
      console.error('Failed to navigate webview:', err);
    }
  };

  const navigateTo = async (newUrl: string) => {
    setUrl(newUrl);
    setInputUrl(newUrl);
    try {
      await invoke('navigate_webview', { url: newUrl });
    } catch (err) {
      console.error('Failed to navigate webview:', err);
    }
  };

  const handleClose = async () => {
    await getCurrentWindow().close();
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100vh", backgroundColor: "#1e1e1e", color: "white" }}>
      {/* Chrome UI */}
      <div
        data-tauri-drag-region
        style={{
          height: "50px",
          display: "flex",
          alignItems: "center",
          padding: "0 10px",
          gap: "10px",
          backgroundColor: "#2d2d2d",
          borderBottom: "1px solid #3d3d3d"
        }}
      >
        <button onClick={() => navigateTo("https://google.com")} style={btnStyle} title="Home"><Home size={16} /></button>
        <button onClick={() => navigateTo(url)} style={btnStyle} title="Reload"><RotateCcw size={16} /></button>

        <form onSubmit={handleNavigate} style={{ flex: 1, display: "flex" }}>
          <input
            type="text"
            value={inputUrl}
            onChange={(e) => setInputUrl(e.target.value)}
            style={{
              flex: 1,
              padding: "6px 12px",
              borderRadius: "4px",
              border: "1px solid #444",
              backgroundColor: "#1e1e1e",
              color: "white",
              outline: "none",
              fontSize: "13px"
            }}
            placeholder="Search or enter web address"
          />
        </form>

        <button onClick={handleClose} style={{ ...btnStyle, color: "#ff5f56" }} title="Close"><X size={18} /></button>
      </div>

      {/* Content Area using Native Webview Placeholder */}
      <div ref={containerRef} style={{ flex: 1, display: "flex", flexDirection: "column", backgroundColor: "white" }}>
        {/* Native Webview will be positioned over this area */}
      </div>
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  background: "transparent",
  border: "none",
  color: "#ccc",
  cursor: "pointer",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  padding: "4px",
  borderRadius: "4px",
};
