import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Power, PowerOff, Trash2, Clock, Zap } from "lucide-react";
import { HeaderItem } from "./PingZeroTypes";

interface PingZeroSseProps {
  url: string;
  headers: HeaderItem[];
}

interface SseMessage {
  id: string;
  type: "event" | "error" | "connected" | "disconnected";
  data: string;
  eventName?: string;
  eventId?: string;
  timestamp: number;
}

export default function PingZeroSse({ url, headers }: PingZeroSseProps) {
  const [status, setStatus] = useState<"disconnected" | "connecting" | "connected">("disconnected");
  const [messages, setMessages] = useState<SseMessage[]>([]);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const connect = async () => {
    if (!url.trim()) return;
    try {
      setStatus("connecting");
      
      if (unlistenRef.current) {
        unlistenRef.current();
      }
      unlistenRef.current = await listen("sse-message", (event: any) => {
        const payload = event.payload as any;
        setMessages(prev => [...prev, {
          id: Math.random().toString(36).substr(2, 9),
          type: payload.type,
          data: payload.data,
          eventName: payload.event_name,
          eventId: payload.event_id,
          timestamp: payload.timestamp || Date.now()
        }]);
        
        if (payload.type === 'error' || payload.type === 'disconnected') {
            setStatus("disconnected");
        }
        if (payload.type === 'connected') {
            setStatus("connected");
        }
      });

      const headerMap: Record<string, string> = {};
      headers.filter(h => h.key && h.enabled).forEach(h => {
        headerMap[h.key] = h.value;
      });
      // ensure accept text/event-stream
      headerMap["Accept"] = "text/event-stream";

      await invoke("sse_connect", { url, headers: headerMap });
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
      await invoke("sse_disconnect");
      setStatus("disconnected");
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      setMessages(prev => [...prev, {
        id: Math.random().toString(36).substr(2, 9),
        type: "disconnected",
        data: "Disconnected",
        timestamp: Date.now()
      }]);
    } catch (e: any) {
      console.error(e);
    }
  };

  const clearMessages = () => {
    setMessages([]);
  };

  // cleanup on unmount
  useEffect(() => {
    return () => {
      if (status !== "disconnected") {
        disconnect();
      }
    };
  }, []);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "10px", gap: "10px" }}>
      <div style={{ display: "flex", gap: "10px", alignItems: "center", justifyContent: "space-between" }}>
        <div style={{display: "flex", gap: "10px", alignItems: "center"}}>
            <Zap size={16} color="var(--accent-color)" />
            <span style={{fontWeight: "bold", fontSize: "14px"}}>Server-Sent Events</span>
        </div>
        <div>
            {status === "disconnected" ? (
            <button className="btn btn-primary" onClick={connect} disabled={!url.trim()} style={{ height: "34px", padding: "0 15px", gap: "5px" }}>
                <Power size={14} /> Connect
            </button>
            ) : (
            <button className="btn btn-danger" onClick={disconnect} style={{ height: "34px", padding: "0 15px", gap: "5px" }}>
                <PowerOff size={14} /> Disconnect
            </button>
            )}
        </div>
      </div>

      <div style={{ flex: 1, border: "1px solid var(--border-primary)", borderRadius: "4px", backgroundColor: "var(--bg-secondary)", display: "flex", flexDirection: "column", overflow: "hidden", minHeight: "400px" }}>
        <div style={{ padding: "8px", borderBottom: "1px solid var(--border-primary)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontSize: "12px", fontWeight: "bold", color: "var(--text-secondary)" }}>
            Stream Log ({messages.length})
          </span>
          <button className="btn btn-secondary" onClick={clearMessages} style={{ padding: "4px 8px", fontSize: "11px", height: "auto" }}>
            <Trash2 size={12} style={{ marginRight: "4px" }} /> Clear
          </button>
        </div>
        <div style={{ flex: 1, overflowY: "auto", padding: "10px", display: "flex", flexDirection: "column", gap: "8px" }}>
          {messages.length === 0 && (
             <div style={{ margin: "auto", color: "var(--text-muted)", fontSize: "12px" }}>Waiting for events...</div>
          )}
          {messages.map(msg => (
            <div key={msg.id} style={{ 
              display: "flex", 
              flexDirection: "column", 
              width: "100%",
              marginBottom: "4px"
            }}>
              <div style={{
                padding: "8px 12px",
                borderRadius: "6px",
                backgroundColor: msg.type === "event" ? "var(--bg-primary)" : 
                                msg.type === "error" ? "#ff4d4f" : "transparent",
                color: msg.type === "error" ? "#fff" : "var(--text-primary)",
                border: msg.type === "event" ? "1px solid var(--border-primary)" : "none",
                fontSize: "13px",
                fontFamily: "monospace",
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
                fontStyle: (msg.type === "connected" || msg.type === "disconnected") ? "italic" : "normal",
                opacity: (msg.type === "connected" || msg.type === "disconnected") ? 0.7 : 1
              }}>
                {msg.type === "event" && msg.eventName && (
                    <div style={{color: "var(--accent-color)", fontWeight: "bold", marginBottom: "4px", fontSize: "11px", textTransform: "uppercase"}}>
                        Event: {msg.eventName} {msg.eventId ? `(ID: ${msg.eventId})` : ""}
                    </div>
                )}
                {msg.data}
              </div>
              <div style={{ fontSize: "10px", color: "var(--text-muted)", marginTop: "4px" }}>
                <Clock size={10} style={{ display: "inline", marginRight: "2px" }}/> 
                {new Date(msg.timestamp).toLocaleTimeString()}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
