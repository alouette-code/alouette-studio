import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Play, FileText, Server } from "lucide-react";
import { HeaderItem } from "./PingZeroTypes";

interface PingZeroGrpcProps {
  url: string;
  headers: HeaderItem[];
  body: string;
}

export default function PingZeroGrpc({ url, body }: PingZeroGrpcProps) {
  const [service, setService] = useState("");
  const [methodName, setMethodName] = useState("");
  const [protoPath, setProtoPath] = useState("");
  const [status, setStatus] = useState<"idle" | "calling" | "done">("idle");
  const [result, setResult] = useState<any>(null);

  const callGrpc = async () => {
    if (!url || !service || !methodName) {
        alert("Please fill in Server URL, Service Name, and Method Name");
        return;
    }
    try {
      setStatus("calling");
      const res = await invoke("grpc_call", {
        input: {
          url,
          service,
          method: methodName,
          payload: body,
          proto_path: protoPath || null
        }
      });
      setResult(res);
    } catch (e: any) {
      setResult({ status: "ERROR", message: e.toString(), elapsed_ms: 0 });
    } finally {
      setStatus("done");
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "15px", gap: "20px" }}>
      <div style={{ display: "flex", gap: "10px", alignItems: "center" }}>
          <Server size={18} color="var(--accent-color)" />
          <span style={{fontWeight: "bold", fontSize: "16px"}}>gRPC Dynamic Client</span>
      </div>

      <div style={{ display: "flex", gap: "20px" }}>
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: "8px" }}>
          <label style={{ fontSize: "12px", fontWeight: "bold", color: "var(--text-muted)", textTransform: "uppercase" }}>Service Name</label>
          <input
            type="text"
            placeholder="helloworld.Greeter"
            value={service}
            onChange={(e) => setService(e.target.value)}
            style={{ width: "100%", padding: "8px 12px", borderRadius: "4px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}
          />
        </div>
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: "8px" }}>
          <label style={{ fontSize: "12px", fontWeight: "bold", color: "var(--text-muted)", textTransform: "uppercase" }}>Method Name</label>
          <input
            type="text"
            placeholder="SayHello"
            value={methodName}
            onChange={(e) => setMethodName(e.target.value)}
            style={{ width: "100%", padding: "8px 12px", borderRadius: "4px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}
          />
        </div>
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: "8px" }}>
          <label style={{ fontSize: "12px", fontWeight: "bold", color: "var(--text-muted)", textTransform: "uppercase" }}>.proto File Path (Optional)</label>
          <input
            type="text"
            placeholder="/path/to/service.proto"
            value={protoPath}
            onChange={(e) => setProtoPath(e.target.value)}
            style={{ width: "100%", padding: "8px 12px", borderRadius: "4px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}
          />
        </div>
      </div>

      <div style={{ marginTop: "10px" }}>
          <button className="btn btn-primary" onClick={callGrpc} disabled={status === "calling" || !url || !service || !methodName} style={{ height: "36px", padding: "0 20px", gap: "8px", fontWeight: "bold" }}>
            <Play size={14} fill="currentColor" /> {status === "calling" ? "Invoking..." : "Invoke gRPC Method"}
          </button>
      </div>

      {result && (
        <div style={{ flex: 1, border: "1px solid var(--border-primary)", borderRadius: "6px", backgroundColor: "var(--bg-secondary)", display: "flex", flexDirection: "column", overflow: "hidden", marginTop: "10px" }}>
          <div style={{ padding: "10px 15px", borderBottom: "1px solid var(--border-primary)", backgroundColor: "var(--bg-primary)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontSize: "13px", fontWeight: "bold", color: result.status === "ERROR" ? "var(--error-color)" : "var(--success-color)" }}>
              Response Status: {result.status}
            </span>
            <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>{result.elapsed_ms} ms</span>
          </div>
          <div style={{ padding: "15px", flex: 1, overflowY: "auto", fontFamily: "monospace", fontSize: "13px", whiteSpace: "pre-wrap" }}>
             {result.message}
          </div>
        </div>
      )}

      {!result && (
        <div style={{ flex: 1, border: "1px solid var(--border-primary)", borderRadius: "6px", backgroundColor: "var(--bg-secondary)", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", color: "var(--text-muted)", marginTop: "10px" }}>
           <FileText size={48} opacity={0.2} style={{ marginBottom: "15px" }} />
           <p style={{ fontSize: "14px" }}>Enter service, method, and click Invoke to call gRPC.</p>
           <p style={{ fontSize: "12px", opacity: 0.7, marginTop: "5px" }}>Uses JSON payload from the Body tab.</p>
        </div>
      )}
    </div>
  );
}
