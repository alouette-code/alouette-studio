import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Copy } from "lucide-react";
import { AlouetteSelect } from "./AlouetteSelect";
import { AlouetteNumberInput } from "./AlouetteControls";

type ToolsTab =
  | "parallel"
  | "dns"
  | "ping"
  | "ssl"
  | "jwt"
  | "hash"
  | "base64"
  | "timestamp"
  | "status";

interface ToolResult {
  title: string;
  content: string;
  elapsedMs?: number;
  error?: string;
}

interface ParallelDiagnosticResult {
  target: string;
  elapsed_ms: number;
  dns: { Ok?: any; Err?: string };
  ping: { Ok?: any; Err?: string };
  ssl: { Ok?: any; Err?: string };
}

function parseTargets(raw: string): string[] {
  return raw
    .split(/[,\n]+/)
    .map((s) => s.trim())
    .filter(Boolean);
}

export default function PingZeroNetworkTools(
  _props: { onClose?: () => void } = {},
) {
  const [activeTool, setActiveTool] = useState<ToolsTab>("parallel");
  const [runningTools, setRunningTools] = useState<Partial<Record<ToolsTab, boolean>>>({});
  const [results, setResults] = useState<Record<string, ToolResult>>({});

  // Parallel multi-host diagnostics
  const [batchTargets, setBatchTargets] = useState("google.com, cloudflare.com");
  const [batchPingCount, setBatchPingCount] = useState(3);
  const [batchSslPort, setBatchSslPort] = useState(443);

  // Individual tool inputs (DNS/Ping/SSL also support comma-separated multi-hosts)
  const [dnsDomain, setDnsDomain] = useState("google.com");
  const [pingHost, setPingHost] = useState("google.com");
  const [pingCount, setPingCount] = useState(4);
  const [sslHost, setSslHost] = useState("google.com");
  const [sslPort, setSslPort] = useState(443);
  const [jwtToken, setJwtToken] = useState("");
  const [hashText, setHashText] = useState("");
  const [base64Text, setBase64Text] = useState("");
  const [base64Action, setBase64Action] = useState<"encode" | "decode">("encode");
  const [tsValue, setTsValue] = useState("");
  const [statusCode, setStatusCode] = useState("200");

  const setToolRunning = (tool: ToolsTab, busy: boolean) => {
    setRunningTools((prev) => ({ ...prev, [tool]: busy }));
  };

  const runTool = async (targetTool: ToolsTab = activeTool) => {
    setToolRunning(targetTool, true);
    const t0 = performance.now();
    try {
      switch (targetTool) {
        case "parallel": {
          const targets = parseTargets(batchTargets);
          if (targets.length === 0) {
            throw new Error("Enter at least one host/domain (comma-separated).");
          }
          const res = await invoke<ParallelDiagnosticResult[]>("run_parallel_diagnostics", {
            targets,
            pingCount: batchPingCount,
            sslPort: batchSslPort,
          });
          const elapsedMs = Math.round(performance.now() - t0);
          const blocks = res.map((r) => {
            const dnsLine = r.dns.Ok
              ? `DNS:  IPv4=[${r.dns.Ok.ipv4.join(", ") || "None"}] IPv6=[${r.dns.Ok.ipv6.join(", ") || "None"}]`
              : `DNS:  Error: ${r.dns.Err}`;
            const pingLine = r.ping.Ok
              ? `PING: ${r.ping.Ok.received}/${r.ping.Ok.sent} ok | loss=${r.ping.Ok.packet_loss.toFixed(1)}% | avg=${r.ping.Ok.avg_ms.toFixed(1)}ms`
              : `PING: Error: ${r.ping.Err}`;
            const sslLine = r.ssl.Ok
              ? `SSL:  ${r.ssl.Ok.tls_version} | expires in ${r.ssl.Ok.expires_in_days}d (${r.ssl.Ok.valid_to}) | ${r.ssl.Ok.issuer}`
              : `SSL:  Error: ${r.ssl.Err}`;
            return `=== ${r.target} (${r.elapsed_ms}ms) ===\n${dnsLine}\n${pingLine}\n${sslLine}`;
          });
          setResults((prev) => ({
            ...prev,
            parallel: {
              title: `Parallel Diagnostics (${res.length} targets)`,
              elapsedMs,
              content: blocks.join("\n\n"),
            },
          }));
          break;
        }
        case "dns": {
          const targets = parseTargets(dnsDomain);
          const list = targets.length > 0 ? targets : ["google.com"];
          const all = await Promise.all(
            list.map(async (domain) => {
              const res: any = await invoke("dns_lookup", { domain });
              return [
                `[${res.domain}]`,
                `  IPv4: ${res.ipv4.join(", ") || "None"}`,
                `  IPv6: ${res.ipv6.join(", ") || "None"}`,
                `  CNAME: ${res.cname || "None"}`,
                `  MX: ${res.mx.join(", ") || "None"}`,
                `  NS: ${res.ns.join(", ") || "None"}`,
                `  TXT: ${res.txt.join(", ") || "None"}`,
              ].join("\n");
            }),
          );
          setResults((prev) => ({
            ...prev,
            dns: {
              title: list.length > 1 ? `DNS Lookup (${list.length} parallel)` : "DNS Lookup",
              elapsedMs: Math.round(performance.now() - t0),
              content: all.join("\n\n"),
            },
          }));
          break;
        }
        case "ping": {
          const targets = parseTargets(pingHost);
          const list = targets.length > 0 ? targets : ["google.com"];
          const all = await Promise.all(
            list.map(async (host) => {
              const res: any = await invoke("ping_host", { host, count: pingCount });
              return [
                `[${res.host}]`,
                `  Packets: Sent=${res.sent}, Received=${res.received}, Loss=${res.packet_loss.toFixed(1)}%`,
                `  Round Trip: Min=${res.min_ms.toFixed(1)}ms, Max=${res.max_ms.toFixed(1)}ms, Avg=${res.avg_ms.toFixed(1)}ms`,
              ].join("\n");
            }),
          );
          setResults((prev) => ({
            ...prev,
            ping: {
              title: list.length > 1 ? `Ping (${list.length} hosts parallel)` : "Ping",
              elapsedMs: Math.round(performance.now() - t0),
              content: all.join("\n\n"),
            },
          }));
          break;
        }
        case "ssl": {
          const targets = parseTargets(sslHost);
          const list = targets.length > 0 ? targets : ["google.com"];
          const all = await Promise.all(
            list.map(async (host) => {
              const res: any = await invoke("ssl_certificate_info", { host, port: sslPort });
              return [
                `[${host}:${sslPort}]`,
                `  Subject: ${res.subject}`,
                `  Issuer: ${res.issuer}`,
                `  Valid From: ${res.valid_from}`,
                `  Valid To: ${res.valid_to}`,
                `  Expires In: ${res.expires_in_days} days`,
                `  Fingerprint: ${res.fingerprint}`,
                `  TLS: ${res.tls_version}`,
              ].join("\n");
            }),
          );
          setResults((prev) => ({
            ...prev,
            ssl: {
              title: list.length > 1 ? `SSL Check (${list.length} hosts parallel)` : "SSL Certificate Info",
              elapsedMs: Math.round(performance.now() - t0),
              content: all.join("\n\n"),
            },
          }));
          break;
        }
        case "jwt": {
          const res: any = await invoke("jwt_decode", { token: jwtToken });
          if (res.error) {
            setResults((prev) => ({
              ...prev,
              jwt: { title: "JWT Decode", content: "", error: res.error },
            }));
          } else {
            const header = JSON.stringify(res.header, null, 2);
            const payload = JSON.stringify(res.payload, null, 2);
            setResults((prev) => ({
              ...prev,
              jwt: {
                title: "JWT Decode",
                elapsedMs: Math.round(performance.now() - t0),
                content: `HEADER:\n${header}\n\nPAYLOAD:\n${payload}\n\nSignature: ${res.signature.substring(0, 20)}...`,
              },
            }));
          }
          break;
        }
        case "hash": {
          const res: any = await invoke("hash_tool", { text: hashText || "Hello World" });
          const lines = [`MD5:    ${res.md5}`, `SHA1:   ${res.sha1}`, `SHA256: ${res.sha256}`];
          setResults((prev) => ({
            ...prev,
            hash: {
              title: "Hash Generator",
              elapsedMs: Math.round(performance.now() - t0),
              content: lines.join("\n"),
            },
          }));
          break;
        }
        case "base64": {
          const res: any = await invoke("base64_tool", {
            input: { text: base64Text, action: base64Action },
          });
          setResults((prev) => ({
            ...prev,
            base64: {
              title: `Base64 ${base64Action === "encode" ? "Encode" : "Decode"}`,
              elapsedMs: Math.round(performance.now() - t0),
              content: res.result,
            },
          }));
          break;
        }
        case "timestamp": {
          const ts = tsValue ? parseInt(tsValue) : null;
          const res: any = await invoke("timestamp_convert", { timestamp: ts, format: null });
          const lines = [
            `Unix Seconds: ${res.unix_seconds}`,
            `Unix Milliseconds: ${res.unix_milliseconds}`,
            `UTC: ${res.utc_string}`,
            `Local: ${res.local_string}`,
            `ISO 8601: ${res.iso_8601}`,
            `Relative: ${res.relative}`,
          ];
          setResults((prev) => ({
            ...prev,
            timestamp: {
              title: "Timestamp Converter",
              elapsedMs: Math.round(performance.now() - t0),
              content: lines.join("\n"),
            },
          }));
          break;
        }
        case "status": {
          const res: any = await invoke("http_status_info", {
            code: parseInt(statusCode) || 200,
          });
          const lines = [
            `Code: ${res.code}`,
            `Name: ${res.name}`,
            `Category: ${res.category}`,
            `Description: ${res.description}`,
          ];
          setResults((prev) => ({
            ...prev,
            status: {
              title: "HTTP Status Code",
              elapsedMs: Math.round(performance.now() - t0),
              content: lines.join("\n"),
            },
          }));
          break;
        }
      }
    } catch (e: any) {
      setResults((prev) => ({
        ...prev,
        [targetTool]: { title: "Error", content: "", error: e.toString() },
      }));
    } finally {
      setToolRunning(targetTool, false);
    }
  };

  const tools: { id: ToolsTab; label: string }[] = [
    { id: "parallel", label: "Parallel" },
    { id: "dns", label: "DNS" },
    { id: "ping", label: "Ping" },
    { id: "ssl", label: "SSL" },
    { id: "jwt", label: "JWT" },
    { id: "hash", label: "Hash" },
    { id: "base64", label: "Base64" },
    { id: "timestamp", label: "Time" },
    { id: "status", label: "Status" },
  ];

  const copyResult = () => {
    const r = results[activeTool];
    if (r?.content) {
      navigator.clipboard.writeText(r.content);
    }
  };

  const activeRunning = !!runningTools[activeTool];
  const totalRunningCount = Object.values(runningTools).filter(Boolean).length;

  return (
    <div className="network-tools-panel">
      <div className="tools-sidebar">
        {tools.map((tool) => {
          const isRunning = !!runningTools[tool.id];
          return (
            <button
              key={tool.id}
              className={`tool-btn ${activeTool === tool.id ? "active" : ""}`}
              onClick={() => setActiveTool(tool.id)}
              style={{ justifyContent: "space-between" }}
            >
              <span>{tool.label}</span>
              {isRunning && (
                <span
                  title="Running concurrently"
                  style={{
                    width: "6px",
                    height: "6px",
                    borderRadius: "50%",
                    backgroundColor: "var(--color-accent)",
                    flexShrink: 0,
                  }}
                />
              )}
            </button>
          );
        })}
      </div>

      <div className="tool-workspace">
        {/* Input Section */}
        <div className="tool-inputs">
          {activeTool === "parallel" && (
            <>
              <div className="input-group">
                <label>Hosts / Domains (comma-separated — runs DNS + Ping + SSL concurrently)</label>
                <input
                  type="text"
                  value={batchTargets}
                  onChange={(e) => setBatchTargets(e.target.value)}
                  placeholder="google.com, cloudflare.com, 1.1.1.1"
                />
              </div>
              <div className="input-group" style={{ flex: "0 0 115px", minWidth: "110px" }}>
                <label>Ping Count</label>
                <AlouetteNumberInput
                  min={1}
                  max={10}
                  value={batchPingCount}
                  onChange={(val) => setBatchPingCount(val || 3)}
                />
              </div>
              <div className="input-group" style={{ flex: "0 0 120px", minWidth: "115px" }}>
                <label>SSL Port</label>
                <AlouetteNumberInput
                  min={1}
                  max={65535}
                  value={batchSslPort}
                  onChange={(val) => setBatchSslPort(val || 443)}
                />
              </div>
            </>
          )}
          {activeTool === "dns" && (
            <div className="input-group">
              <label>Domain Name(s) (comma-separated for parallel)</label>
              <input
                type="text"
                value={dnsDomain}
                onChange={(e) => setDnsDomain(e.target.value)}
                placeholder="e.g. google.com, github.com"
              />
            </div>
          )}
          {activeTool === "ping" && (
            <>
              <div className="input-group">
                <label>Host(s) (comma-separated for parallel)</label>
                <input
                  type="text"
                  value={pingHost}
                  onChange={(e) => setPingHost(e.target.value)}
                  placeholder="e.g. google.com, 1.1.1.1"
                />
              </div>
              <div className="input-group" style={{ flex: "0 0 130px", minWidth: "120px" }}>
                <label>Count</label>
                <AlouetteNumberInput
                  min={1}
                  max={10}
                  value={pingCount}
                  onChange={(val) => setPingCount(val || 4)}
                />
              </div>
            </>
          )}
          {activeTool === "ssl" && (
            <>
              <div className="input-group">
                <label>Host(s) (comma-separated for parallel)</label>
                <input
                  type="text"
                  value={sslHost}
                  onChange={(e) => setSslHost(e.target.value)}
                  placeholder="e.g. google.com, cloudflare.com"
                />
              </div>
              <div className="input-group" style={{ flex: "0 0 140px", minWidth: "130px" }}>
                <label>Port</label>
                <AlouetteNumberInput
                  min={1}
                  max={65535}
                  value={sslPort}
                  onChange={(val) => setSslPort(val || 443)}
                />
              </div>
            </>
          )}
          {activeTool === "jwt" && (
            <div className="input-group">
              <label>JWT Token</label>
              <textarea
                value={jwtToken}
                onChange={(e) => setJwtToken(e.target.value)}
                placeholder="eyJhbGciOiJIUzI1NiIs..."
                rows={3}
              />
            </div>
          )}
          {activeTool === "hash" && (
            <div className="input-group">
              <label>Text to Hash</label>
              <textarea
                value={hashText}
                onChange={(e) => setHashText(e.target.value)}
                placeholder="Enter text to hash..."
                rows={2}
              />
            </div>
          )}
          {activeTool === "base64" && (
            <>
              <div className="input-group" style={{ flex: "0 0 185px", minWidth: "170px" }}>
                <label>Action</label>
                <AlouetteSelect
                  value={base64Action}
                  options={[
                    { value: "encode", label: "Encode → Base64" },
                    { value: "decode", label: "Decode ← Base64" },
                  ]}
                  onChange={(val) => setBase64Action(val as "encode" | "decode")}
                  searchable={false}
                />
              </div>
              <div className="input-group">
                <label>{base64Action === "encode" ? "Plain Text" : "Base64 Text"}</label>
                <textarea
                  value={base64Text}
                  onChange={(e) => setBase64Text(e.target.value)}
                  placeholder="Enter text..."
                  rows={2}
                />
              </div>
            </>
          )}
          {activeTool === "timestamp" && (
            <div className="input-group">
              <label>Unix Timestamp (leave empty for now)</label>
              <input
                type="text"
                value={tsValue}
                onChange={(e) => setTsValue(e.target.value)}
                placeholder="e.g. 1716000000 or 1716000000000"
              />
            </div>
          )}
          {activeTool === "status" && (
            <div className="input-group" style={{ flex: "0 0 160px", minWidth: "140px" }}>
              <label>HTTP Status Code</label>
              <AlouetteNumberInput
                min={100}
                max={599}
                value={Number(statusCode) || 200}
                onChange={(val) => setStatusCode(String(val))}
              />
            </div>
          )}

          <button
            className="btn btn-primary tool-run-btn"
            onClick={() => runTool(activeTool)}
            disabled={activeRunning}
          >
            <span>{activeRunning ? "Running..." : activeTool === "parallel" ? "Run All" : "Run"}</span>
          </button>
        </div>

        {/* Result Section */}
        {results[activeTool] && (
          <div className="tool-result">
            <div className="tool-result-header">
              <span className="font-bold text-xs">
                {results[activeTool].title}
                {results[activeTool].elapsedMs !== undefined && (
                  <span style={{ color: "var(--text-muted)", fontWeight: 400, marginLeft: "8px" }}>
                    ({results[activeTool].elapsedMs}ms)
                  </span>
                )}
              </span>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                {totalRunningCount > 0 && (
                  <span style={{ fontSize: "10px", color: "var(--color-accent)" }}>
                    {totalRunningCount} task{totalRunningCount > 1 ? "s" : ""} active
                  </span>
                )}
                <button
                  className="tool-copy-btn"
                  onClick={copyResult}
                  title="Copy to clipboard"
                >
                  <Copy size={12} />
                </button>
              </div>
            </div>
            {results[activeTool].error ? (
              <div className="tool-error">
                <pre>{results[activeTool].error}</pre>
              </div>
            ) : (
              <pre className="tool-output">{results[activeTool].content}</pre>
            )}
          </div>
        )}

        {!results[activeTool] && (
          <div className="tool-placeholder">
            <span className="text-muted text-xs">
              {activeRunning
                ? "Running in background — you can switch to other tools concurrently"
                : 'Configure and click "Run" to execute (supports multiple concurrent tasks)'}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
