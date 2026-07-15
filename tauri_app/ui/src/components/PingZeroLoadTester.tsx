import { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Play, Activity } from "lucide-react";
import { HeaderItem } from "./PingZeroTypes";

interface PingZeroLoadTesterProps {
  url: string;
  method: string;
  headers: HeaderItem[];
  body: string;
}

interface LoadTestProgress {
  current_sec: number;
  total_sec: number;
  total_requests: number;
  rps: number;
  p95_latency: number;
  error_count: number;
}

interface LoadTestResult {
  total_requests: number;
  success_count: number;
  error_count: number;
  total_time_ms: number;
  rps: number;
  min_latency_ms: number;
  max_latency_ms: number;
  avg_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
}

export default function PingZeroLoadTester({ url, method, headers, body }: PingZeroLoadTesterProps) {
  const [vus, setVus] = useState<number>(10);
  const [duration, setDuration] = useState<number>(10); // seconds
  const [status, setStatus] = useState<"idle" | "running" | "done">("idle");
  const [progress, setProgress] = useState<LoadTestProgress | null>(null);
  const [result, setResult] = useState<LoadTestResult | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const startTest = async () => {
    if (!url.trim()) return;
    setStatus("running");
    setProgress(null);
    setResult(null);

    try {
      if (unlistenRef.current) {
        unlistenRef.current();
      }
      unlistenRef.current = await listen("load-test-progress", (event: any) => {
        setProgress(event.payload as LoadTestProgress);
      });

      const headerMap: Record<string, string> = {};
      headers.filter(h => h.key && h.enabled).forEach(h => {
        headerMap[h.key] = h.value;
      });

      const res = await invoke<LoadTestResult>("run_load_test", {
        input: {
          url,
          method,
          headers: headerMap,
          body: body ? body : null,
          vus,
          duration_sec: duration
        }
      });

      setResult(res);
      setStatus("done");
    } catch (e) {
      console.error(e);
      setStatus("done");
    } finally {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "10px", gap: "15px" }}>
      <div style={{ display: "flex", gap: "20px", alignItems: "flex-end" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
          <label style={{ fontSize: "11px", fontWeight: "bold", color: "var(--text-muted)" }}>Virtual Users (Concurrency)</label>
          <input
            type="number"
            min="1"
            max="1000"
            value={vus}
            onChange={(e) => setVus(parseInt(e.target.value) || 1)}
            disabled={status === "running"}
            style={{ width: "120px", padding: "6px", borderRadius: "4px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}
          />
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
          <label style={{ fontSize: "11px", fontWeight: "bold", color: "var(--text-muted)" }}>Duration (Seconds)</label>
          <input
            type="number"
            min="1"
            max="3600"
            value={duration}
            onChange={(e) => setDuration(parseInt(e.target.value) || 10)}
            disabled={status === "running"}
            style={{ width: "120px", padding: "6px", borderRadius: "4px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}
          />
        </div>
        <div>
          {status === "running" ? (
            <button className="btn btn-secondary" disabled style={{ height: "32px", padding: "0 15px", gap: "5px", opacity: 0.7 }}>
              <Activity size={14} className="spin" /> Running...
            </button>
          ) : (
            <button className="btn btn-primary" onClick={startTest} disabled={!url} style={{ height: "32px", padding: "0 15px", gap: "5px" }}>
              <Play size={14} fill="currentColor" /> Start Load Test
            </button>
          )}
        </div>
      </div>

      {status === "running" && progress && (
        <div style={{ border: "1px solid var(--border-primary)", borderRadius: "6px", padding: "15px", backgroundColor: "var(--bg-secondary)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "10px" }}>
            <span style={{ fontSize: "13px", fontWeight: "bold" }}>Running Test...</span>
            <span style={{ fontSize: "13px", color: "var(--accent-color)" }}>{progress.current_sec}s / {progress.total_sec}s</span>
          </div>
          <div style={{ width: "100%", height: "8px", backgroundColor: "var(--bg-primary)", borderRadius: "4px", overflow: "hidden" }}>
            <div style={{ width: `${(progress.current_sec / progress.total_sec) * 100}%`, height: "100%", backgroundColor: "var(--accent-color)", transition: "width 0.3s ease" }} />
          </div>
          <div style={{ display: "flex", gap: "20px", marginTop: "15px" }}>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase" }}>Current RPS</div>
              <div style={{ fontSize: "24px", fontWeight: "bold" }}>{progress.rps.toFixed(1)}</div>
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase" }}>P95 Latency</div>
              <div style={{ fontSize: "24px", fontWeight: "bold", color: "var(--warning-color)" }}>{progress.p95_latency.toFixed(1)} ms</div>
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase" }}>Total Requests</div>
              <div style={{ fontSize: "24px", fontWeight: "bold" }}>{progress.total_requests}</div>
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase" }}>Errors</div>
              <div style={{ fontSize: "24px", fontWeight: "bold", color: "var(--error-color)" }}>{progress.error_count}</div>
            </div>
          </div>
        </div>
      )}

      {status === "done" && result && (
        <div style={{ border: "1px solid var(--border-primary)", borderRadius: "6px", backgroundColor: "var(--bg-secondary)", display: "flex", flexDirection: "column", overflow: "hidden" }}>
          <div style={{ padding: "10px 15px", borderBottom: "1px solid var(--border-primary)", backgroundColor: "rgba(0,0,0,0.2)" }}>
            <span style={{ fontSize: "14px", fontWeight: "bold", color: "var(--accent-color)" }}>Test Report Summary</span>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1px", backgroundColor: "var(--border-primary)" }}>
            <div style={{ padding: "15px", backgroundColor: "var(--bg-secondary)" }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase", marginBottom: "5px" }}>Total Requests</div>
              <div style={{ fontSize: "20px", fontWeight: "bold" }}>{result.total_requests}</div>
            </div>
            <div style={{ padding: "15px", backgroundColor: "var(--bg-secondary)" }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase", marginBottom: "5px" }}>Avg Requests / Sec (RPS)</div>
              <div style={{ fontSize: "20px", fontWeight: "bold" }}>{result.rps.toFixed(2)}</div>
            </div>
            
            <div style={{ padding: "15px", backgroundColor: "var(--bg-secondary)" }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase", marginBottom: "5px" }}>Avg Latency</div>
              <div style={{ fontSize: "20px", fontWeight: "bold" }}>{result.avg_latency_ms.toFixed(2)} ms</div>
            </div>
            <div style={{ padding: "15px", backgroundColor: "var(--bg-secondary)" }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase", marginBottom: "5px" }}>P95 / P99 Latency</div>
              <div style={{ fontSize: "20px", fontWeight: "bold" }}>{result.p95_latency_ms.toFixed(2)} / {result.p99_latency_ms.toFixed(2)} ms</div>
            </div>

            <div style={{ padding: "15px", backgroundColor: "var(--bg-secondary)" }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase", marginBottom: "5px" }}>Success Rate</div>
              <div style={{ fontSize: "20px", fontWeight: "bold", color: "var(--success-color)" }}>
                {result.total_requests > 0 ? ((result.success_count / result.total_requests) * 100).toFixed(1) : 0}%
              </div>
            </div>
            <div style={{ padding: "15px", backgroundColor: "var(--bg-secondary)" }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", textTransform: "uppercase", marginBottom: "5px" }}>Error Count</div>
              <div style={{ fontSize: "20px", fontWeight: "bold", color: result.error_count > 0 ? "var(--error-color)" : "inherit" }}>
                {result.error_count}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
