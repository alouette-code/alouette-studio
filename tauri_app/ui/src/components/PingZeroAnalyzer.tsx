import React, { useState, useMemo } from "react";
import { ChevronRight, ChevronDown, Braces, Brackets, Hash, Type, AlignLeft, ToggleLeft, BoxSelect, Server, Network } from "lucide-react";
import type { ApiResponse, HeaderItem } from "./PingZeroTypes";

interface PingZeroAnalyzerProps {
  mode: "request" | "response";
  requestInfo?: {
    method: string;
    url: string;
    headers: HeaderItem[];
    bodyType: string;
    body: string;
  };
  responseInfo?: ApiResponse;
}

const getTypeIcon = (type: string) => {
  switch (type) {
    case "object": return <Braces size={12} style={{ color: "var(--color-info, #4facfe)" }} />;
    case "array": return <Brackets size={12} style={{ color: "var(--color-warning, #f6d365)" }} />;
    case "number": return <Hash size={12} style={{ color: "var(--color-success, #43e97b)" }} />;
    case "boolean": return <ToggleLeft size={12} style={{ color: "var(--color-error, #ff0844)" }} />;
    case "string": return <Type size={12} style={{ color: "var(--color-accent, #fa709a)" }} />;
    case "null": return <AlignLeft size={12} style={{ color: "var(--text-muted, #888)" }} />;
    default: return <BoxSelect size={12} style={{ color: "var(--text-muted, #888)" }} />;
  }
};

const getTypeLabel = (value: any): string => {
  if (value === null) return "null";
  if (Array.isArray(value)) return `array [${value.length}]`;
  return typeof value;
};

const JsonTreeNode: React.FC<{ name: string; value: any; depth?: number }> = ({ name, value, depth = 0 }) => {
  const [expanded, setExpanded] = useState(depth < 2);
  const type = getTypeLabel(value);
  const isComplex = type === "object" || type.startsWith("array");

  return (
    <div className="mono text-sm" style={{ paddingLeft: `${depth * 16}px`, lineHeight: "1.6" }}>
      <div 
        style={{ 
          display: "flex", 
          alignItems: "center", 
          gap: "8px", 
          padding: "2px 4px", 
          borderRadius: "4px",
          cursor: isComplex ? "pointer" : "default" 
        }}
        className={isComplex ? "hover-bg" : ""}
        onClick={() => isComplex && setExpanded(!expanded)}
      >
        <span style={{ width: "16px", display: "flex", justifyContent: "center", opacity: 0.7 }}>
          {isComplex && (
            expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />
          )}
        </span>
        <span style={{ color: "var(--color-info, #60a5fa)", fontWeight: "bold" }}>{name}</span>
        <span style={{ color: "var(--text-secondary, #888)" }}>:</span>
        
        {!isComplex ? (
          <span style={{ 
            color: type === 'string' ? "var(--color-success, #4ade80)" : 
                   type === 'number' ? "var(--color-warning, #fbbf24)" : "var(--color-error, #f43f5e)",
            overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: "400px"
          }}>
            {type === 'string' ? `"${value}"` : String(value)}
          </span>
        ) : (
          !expanded && <span style={{ color: "var(--text-muted, #666)", fontStyle: "italic", fontSize: "11px" }}>{type}</span>
        )}
        
        <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: "4px" }}>
          <span style={{ 
            padding: "2px 6px", borderRadius: "4px", fontSize: "10px", textTransform: "uppercase", 
            letterSpacing: "0.5px", background: "var(--bg-tertiary, rgba(255,255,255,0.05))", 
            color: "var(--text-secondary, #999)", display: "flex", alignItems: "center", gap: "4px" 
          }}>
            {getTypeIcon(type.split(' ')[0])}
            {type}
          </span>
        </div>
      </div>
      
      {isComplex && expanded && (
        <div style={{ borderLeft: "1px solid var(--border-primary, rgba(255,255,255,0.1))", marginLeft: "8px" }}>
          {Object.entries(value).map(([k, v]) => (
            <JsonTreeNode key={k} name={k} value={v} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  );
};

export default function PingZeroAnalyzer({ mode, requestInfo, responseInfo }: PingZeroAnalyzerProps) {
  const parsedBody = useMemo(() => {
    let bodyRaw = "";
    if (mode === "request" && requestInfo) {
      bodyRaw = requestInfo.body;
    } else if (mode === "response" && responseInfo) {
      bodyRaw = responseInfo.body;
    }

    if (!bodyRaw) return null;

    try {
      return JSON.parse(bodyRaw);
    } catch {
      return null;
    }
  }, [mode, requestInfo, responseInfo]);

  const rawHttpString = useMemo(() => {
    let str = "";
    if (mode === "request" && requestInfo) {
      const urlObj = new URL(requestInfo.url || "http://localhost");
      str += `${requestInfo.method} ${urlObj.pathname}${urlObj.search} HTTP/1.1\n`;
      str += `Host: ${urlObj.host}\n`;
      requestInfo.headers.forEach(h => {
        if (h.enabled && h.key) {
          str += `${h.key}: ${h.value}\n`;
        }
      });
      str += "\n" + (requestInfo.body || "");
    } else if (mode === "response" && responseInfo) {
      str += `HTTP/1.1 ${responseInfo.status} ${responseInfo.statusText}\n`;
      Object.entries(responseInfo.headers || {}).forEach(([k, v]) => {
        str += `${k}: ${v}\n`;
      });
      str += "\n" + (responseInfo.body || "");
    }
    return str;
  }, [mode, requestInfo, responseInfo]);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", color: "var(--text-primary)" }}>
      <div style={{ 
        padding: "10px 16px", borderBottom: "1px solid var(--border-primary)", 
        display: "flex", alignItems: "center", gap: "8px", background: "var(--bg-secondary)" 
      }}>
        <Network size={16} style={{ color: "var(--color-info)" }} />
        <h3 style={{ margin: 0, fontWeight: "bold", fontSize: "13px", textTransform: "uppercase", letterSpacing: "0.5px" }}>
          {mode === "request" ? "Request Inspector" : "Response Inspector"}
        </h3>
        <span style={{ marginLeft: "auto", fontSize: "11px", color: "var(--text-muted)", fontFamily: "monospace" }}>
          {mode === "request" && requestInfo?.bodyType !== "none" ? `Type: ${requestInfo?.bodyType}` : ""}
          {mode === "response" && `Size: ${((responseInfo?.sizeBytes || 0) / 1024).toFixed(2)} KB`}
        </span>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: "16px", display: "flex", flexDirection: "column", gap: "24px" }} className="custom-scrollbar">
        
        {/* JSON Tree Analyzer */}
        <div>
          <h4 style={{ fontSize: "11px", fontWeight: "bold", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "1px", marginBottom: "12px", display: "flex", alignItems: "center", gap: "6px" }}>
            <Server size={14} /> Payload Structure
          </h4>
          {parsedBody ? (
            <div style={{ background: "var(--bg-tertiary, rgba(0,0,0,0.2))", borderRadius: "6px", padding: "12px", border: "1px solid var(--border-primary)" }}>
              <JsonTreeNode name="root" value={parsedBody} />
            </div>
          ) : (
            <div style={{ background: "var(--bg-tertiary, rgba(0,0,0,0.2))", borderRadius: "6px", padding: "16px", border: "1px solid var(--border-primary)", fontSize: "13px", color: "var(--text-muted)", fontStyle: "italic" }}>
              {mode === "request" 
                ? "No valid JSON payload detected in request." 
                : "No valid JSON payload detected in response."}
            </div>
          )}
        </div>

        {/* Raw HTTP Preview */}
        <div>
          <h4 style={{ fontSize: "11px", fontWeight: "bold", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "1px", marginBottom: "12px", display: "flex", alignItems: "center", gap: "6px" }}>
            <AlignLeft size={14} /> Raw HTTP Preview
          </h4>
          <div style={{ background: "var(--bg-tertiary, rgba(0,0,0,0.2))", borderRadius: "6px", padding: "12px", border: "1px solid var(--border-primary)", overflowX: "auto" }} className="custom-scrollbar">
            <pre style={{ fontSize: "12px", fontFamily: "monospace", color: "var(--text-secondary)", margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
              {rawHttpString || "No data"}
            </pre>
          </div>
        </div>

      </div>
    </div>
  );
}
