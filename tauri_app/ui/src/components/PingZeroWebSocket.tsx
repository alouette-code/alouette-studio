import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Send, Power, PowerOff, Trash2, Clock } from "lucide-react";
import { WsMessage } from "./PingZeroTypes";

interface PingZeroWebSocketProps {
  url: string;
}

export default function PingZeroWebSocket({ url }: PingZeroWebSocketProps) {
  const [wsUrl, setWsUrl] = useState(url.startsWith("ws") ? url : "ws://localhost:8080");
  const [status, setStatus] = useState<"disconnected" | "connecting" | "connected">("disconnected");
  const [messages, setMessages] = useState<WsMessage[]>([]);
  const [inputMessage, setInputMessage] = useState("");
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    if (status === "disconnected" && url.startsWith("ws")) {
      setWsUrl(url);
    }
  }, [url, status]);

  const connect = async () => {
    if (!wsUrl.trim()) return;
    try {
      setStatus("connecting");
      
      if (unlistenRef.current) {
        unlistenRef.current();
      }
      unlistenRef.current = await listen("ws-message", (event: any) => {
        const payload = event.payload as any;
        setMessages(prev => [...prev, {
          id: Math.random().toString(36).substr(2, 9),
          type: payload.type,
          data: payload.data,
          timestamp: payload.timestamp || Date.now()
        }]);
        
        if (payload.type === 'error' && payload.data.includes('disconnected')) {
            setStatus("disconnected");
        }
      });

      await invoke("ws_connect", { url: wsUrl });
      setStatus("connected");
      setMessages(prev => [...prev, {
        id: Math.random().toString(36).substr(2, 9),
        type: "system",
        data: `Connected to ${wsUrl}`,
        timestamp: Date.now()
      }]);
    } catch (e: any) {
      setStatus("disconnected");
      setMessages(prev => [...prev, {
        id: Math.random().toString(36).substr(2, 9),
        type: "error",
        data: e.toString(),
        timestamp: Date.now()
      }]);
    }
  };

  const disconnect = async () => {
    try {
      await invoke("ws_disconnect");
      setStatus("disconnected");
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      setMessages(prev => [...prev, {
        id: Math.random().toString(36).substr(2, 9),
        type: "system",
        data: "Disconnected",
        timestamp: Date.now()
      }]);
    } catch (e: any) {
      console.error(e);
    }
  };

  const sendMessage = async () => {
    if (!inputMessage.trim() || status !== "connected") return;
    try {
      await invoke("ws_send", { message: inputMessage });
      setMessages(prev => [...prev, {
        id: Math.random().toString(36).substr(2, 9),
        type: "sent",
        data: inputMessage,
        timestamp: Date.now()
      }]);
      setInputMessage("");
    } catch (e: any) {
      setMessages(prev => [...prev, {
        id: Math.random().toString(36).substr(2, 9),
        type: "error",
        data: `Failed to send: ${e.toString()}`,
        timestamp: Date.now()
      }]);
    }
  };

  const clearMessages = () => {
    setMessages([]);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "10px", gap: "10px" }}>
      <div style={{ display: "flex", gap: "10px", alignItems: "center" }}>
        <input
          type="text"
          className="api-input"
          value={wsUrl}
          onChange={e => setWsUrl(e.target.value)}
          placeholder="ws:// or wss://"
          style={{ flex: 1, padding: "8px", borderRadius: "4px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}
          disabled={status !== "disconnected"}
        />
        {status === "disconnected" ? (
          <button className="btn btn-primary" onClick={connect} disabled={!wsUrl.trim()} style={{ height: "34px", padding: "0 15px", gap: "5px" }}>
            <Power size={14} /> Connect
          </button>
        ) : (
          <button className="btn btn-danger" onClick={disconnect} style={{ height: "34px", padding: "0 15px", gap: "5px" }}>
            <PowerOff size={14} /> Disconnect
          </button>
        )}
      </div>

      <div style={{ flex: 1, border: "1px solid var(--border-primary)", borderRadius: "4px", backgroundColor: "var(--bg-secondary)", display: "flex", flexDirection: "column", overflow: "hidden", minHeight: "300px" }}>
        <div style={{ padding: "8px", borderBottom: "1px solid var(--border-primary)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontSize: "12px", fontWeight: "bold", color: "var(--text-secondary)" }}>
            Messages ({messages.length})
          </span>
          <button className="btn btn-secondary" onClick={clearMessages} style={{ padding: "4px 8px", fontSize: "11px", height: "auto" }}>
            <Trash2 size={12} style={{ marginRight: "4px" }} /> Clear
          </button>
        </div>
        <div style={{ flex: 1, overflowY: "auto", padding: "10px", display: "flex", flexDirection: "column", gap: "8px" }}>
          {messages.length === 0 && (
             <div style={{ margin: "auto", color: "var(--text-muted)", fontSize: "12px" }}>No messages yet</div>
          )}
          {messages.map(msg => (
            <div key={msg.id} style={{ 
              display: "flex", 
              flexDirection: "column", 
              alignSelf: msg.type === "sent" ? "flex-end" : "flex-start",
              maxWidth: "80%"
            }}>
              <div style={{
                padding: "8px 12px",
                borderRadius: "6px",
                backgroundColor: msg.type === "sent" ? "var(--accent-color)" : 
                                msg.type === "received" ? "var(--bg-primary)" : 
                                msg.type === "error" ? "#ff4d4f" : "transparent",
                color: msg.type === "sent" ? "#fff" : "var(--text-primary)",
                border: msg.type === "received" ? "1px solid var(--border-primary)" : "none",
                fontSize: "13px",
                fontFamily: msg.type === "system" ? "inherit" : "monospace",
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
                fontStyle: msg.type === "system" ? "italic" : "normal",
                opacity: msg.type === "system" ? 0.7 : 1
              }}>
                {msg.data}
              </div>
              <div style={{ fontSize: "10px", color: "var(--text-muted)", marginTop: "4px", alignSelf: msg.type === "sent" ? "flex-end" : "flex-start" }}>
                <Clock size={10} style={{ display: "inline", marginRight: "2px" }}/> 
                {new Date(msg.timestamp).toLocaleTimeString()}
              </div>
            </div>
          ))}
        </div>
      </div>

      <div style={{ display: "flex", gap: "10px" }}>
        <input
          type="text"
          value={inputMessage}
          onChange={e => setInputMessage(e.target.value)}
          onKeyDown={e => e.key === "Enter" && sendMessage()}
          placeholder="Type a message..."
          disabled={status !== "connected"}
          style={{ flex: 1, padding: "8px", borderRadius: "4px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}
        />
        <button className="btn btn-primary" onClick={sendMessage} disabled={status !== "connected" || !inputMessage.trim()} style={{ height: "34px", padding: "0 15px", gap: "5px" }}>
          <Send size={14} /> Send
        </button>
      </div>
    </div>
  );
}
