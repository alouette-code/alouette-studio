import React, { useState } from "react";
import {
  Activity,
  AlertTriangle,
  Check,
  ChevronDown,
  ChevronRight,
  Cpu,
  FileCode2,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  X,
} from "lucide-react";

export interface SuperAgentWorkerInfo {
  id: string;
  role: string;
  mode: string;
  status: string;
  iteration: number;
  max_iterations: number;
  current_action: string;
  files_touched: string[];
  error_count: number;
  logs: string[];
  summary?: string | null;
}

export interface SuperSwarmSnapshot {
  session_id: string;
  phase: string;
  execution_mode: string;
  commander_model: string;
  worker_model: string;
  wave: number;
  total_spawned: number;
  max_agents: number;
  active_count: number;
  supervisor_status: string;
  supervisor_alerts: string[];
  workers: SuperAgentWorkerInfo[];
  headline: string;
}

export interface SuperAgentApprovalRequest {
  approval_id: string;
  session_id: string;
  agent_id: string;
  agent_role: string;
  tool_name: string;
  args_preview: string;
  execution_mode: string;
}

interface SuperSwarmBoardProps {
  snapshot: SuperSwarmSnapshot;
  pendingApprovals: SuperAgentApprovalRequest[];
  onRespondApproval: (
    approvalId: string,
    approved: boolean,
    approveAllSession: boolean,
  ) => void;
}

const STATUS_STYLE: Record<
  string,
  { color: string; bg: string; label: string }
> = {
  queued: { color: "var(--text-muted)", bg: "var(--bg-secondary)", label: "Hàng đợi" },
  running: { color: "var(--text-primary)", bg: "rgba(255,255,255,0.12)", label: "Đang thực thi" },
  waiting_approval: {
    color: "var(--text-primary)",
    bg: "rgba(255,255,255,0.16)",
    label: "Chờ phê duyệt",
  },
  done: { color: "var(--text-primary)", bg: "var(--bg-secondary)", label: "Hoàn tất" },
  halted: {
    color: "var(--text-secondary)",
    bg: "var(--bg-secondary)",
    label: "Ngắt vòng lặp",
  },
  killed: { color: "var(--text-muted)", bg: "var(--bg-secondary)", label: "Đã dừng" },
  error: { color: "var(--text-primary)", bg: "var(--bg-secondary)", label: "Lỗi" },
};

export const SuperSwarmBoard: React.FC<SuperSwarmBoardProps> = ({
  snapshot,
  pendingApprovals,
  onRespondApproval,
}) => {
  const [expandedWorkerId, setExpandedWorkerId] = useState<string | null>(null);
  const [showWorkersGrid, setShowWorkersGrid] = useState<boolean>(true);
  const [showAlerts, setShowAlerts] = useState<boolean>(false);

  const sessionApprovals = pendingApprovals.filter(
    (a) => a.session_id === snapshot.session_id,
  );

  const pct =
    snapshot.max_agents > 0
      ? Math.min(100, Math.round((snapshot.total_spawned / snapshot.max_agents) * 100))
      : 0;

  return (
    <div
      style={{
        border: "1px solid var(--border-primary)",
        background: "var(--bg-secondary)",
        borderRadius: "10px",
        padding: "12px",
        display: "flex",
        flexDirection: "column",
        gap: "10px",
        margin: "4px 0",
      }}
    >
      {/* Header */}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "8px",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <Sparkles size={15} color="var(--text-primary)" />
          <span style={{ fontWeight: 700, fontSize: "13px", color: "var(--text-primary)" }}>
            Super Multi-Agent Swarm
          </span>
          <span
            style={{
              fontSize: "10.5px",
              padding: "2px 7px",
              borderRadius: "999px",
              background: "var(--bg-primary)",
              border: "1px solid var(--border-primary)",
              color: "var(--text-primary)",
              fontWeight: 600,
            }}
          >
            {snapshot.phase.toUpperCase()} · Wave {snapshot.wave}
          </span>
          <span
            style={{
              fontSize: "10.5px",
              padding: "2px 7px",
              borderRadius: "999px",
              background: "var(--bg-primary)",
              border: "1px solid var(--border-primary)",
              color: "var(--text-secondary)",
            }}
          >
            Mode: {snapshot.execution_mode}
          </span>
        </div>

        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "10px",
            fontSize: "11px",
            color: "var(--text-secondary)",
          }}
        >
          <span>
            <strong style={{ color: "var(--text-primary)" }}>
              {snapshot.total_spawned}
            </strong>
            /{snapshot.max_agents} agents
          </span>
          <span>Đang thực thi: {snapshot.active_count}</span>
        </div>
      </div>

      {/* Headline & Progress Bar */}
      <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
        <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
          {snapshot.headline}
        </div>
        <div
          style={{
            width: "100%",
            height: "5px",
            borderRadius: "999px",
            background: "var(--bg-primary)",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              width: `${pct}%`,
              height: "100%",
              background: "var(--text-primary)",
              transition: "width 0.25s ease",
            }}
          />
        </div>
      </div>

      {/* Model & Supervisor Bar */}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "8px",
          padding: "7px 9px",
          borderRadius: "6px",
          background: "var(--bg-primary)",
          border: "1px solid var(--border-primary)",
          fontSize: "11px",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "10px", flexWrap: "wrap" }}>
          <span style={{ display: "flex", alignItems: "center", gap: "4px", color: "var(--text-primary)" }}>
            <Cpu size={12} /> Commander/Supervisor: <strong>{snapshot.commander_model}</strong>
          </span>
          <span style={{ display: "flex", alignItems: "center", gap: "4px", color: "var(--text-secondary)" }}>
            <Activity size={12} /> Workers: <strong>{snapshot.worker_model}</strong>
          </span>
        </div>

        <button
          type="button"
          onClick={() => setShowAlerts(!showAlerts)}
          style={{
            display: "flex",
            alignItems: "center",
            gap: "4px",
            background: "transparent",
            border: "none",
            color:
              snapshot.supervisor_alerts.length > 0 ? "var(--text-primary)" : "var(--text-muted)",
            cursor: "pointer",
            fontSize: "11px",
            padding: 0,
          }}
        >
          {snapshot.supervisor_alerts.length > 0 ? (
            <ShieldAlert size={12} />
          ) : (
            <ShieldCheck size={12} />
          )}
          <span>
            {snapshot.supervisor_status}
            {snapshot.supervisor_alerts.length > 0
              ? ` (${snapshot.supervisor_alerts.length} cảnh báo)`
              : ""}
          </span>
        </button>
      </div>

      {showAlerts && snapshot.supervisor_alerts.length > 0 && (
        <div
          style={{
            padding: "8px 10px",
            borderRadius: "6px",
            background: "var(--bg-primary)",
            border: "1px solid var(--border-primary)",
            fontSize: "11px",
            color: "var(--text-secondary)",
            display: "flex",
            flexDirection: "column",
            gap: "4px",
            maxHeight: "140px",
            overflowY: "auto",
          }}
        >
          {snapshot.supervisor_alerts.map((al, i) => (
            <div key={i}>{al}</div>
          ))}
        </div>
      )}

      {/* Pending Approvals Queue */}
      {sessionApprovals.length > 0 && (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "8px",
            padding: "9px",
            borderRadius: "8px",
            background: "var(--bg-primary)",
            border: "1px solid var(--text-primary)",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              gap: "8px",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "6px",
                fontSize: "12px",
                fontWeight: 600,
                color: "var(--text-primary)",
              }}
            >
              <AlertTriangle size={14} />
              <span>
                {sessionApprovals.length} yêu cầu phê duyệt thao tác ({snapshot.execution_mode})
              </span>
            </div>
            {sessionApprovals.length > 1 && (
              <button
                type="button"
                onClick={() =>
                  onRespondApproval(sessionApprovals[0].approval_id, true, true)
                }
                style={{
                  padding: "3px 8px",
                  borderRadius: "5px",
                  border: "none",
                  background: "var(--text-primary)",
                  color: "var(--bg-primary)",
                  fontSize: "11px",
                  fontWeight: 600,
                  cursor: "pointer",
                }}
              >
                Phê duyệt toàn bộ phiên
              </button>
            )}
          </div>

          {sessionApprovals.map((req) => (
            <div
              key={req.approval_id}
              style={{
                padding: "8px",
                borderRadius: "6px",
                background: "var(--bg-secondary)",
                border: "1px solid var(--border-primary)",
                display: "flex",
                flexDirection: "column",
                gap: "6px",
              }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  gap: "8px",
                  flexWrap: "wrap",
                }}
              >
                <div style={{ fontSize: "11.5px", color: "var(--text-primary)" }}>
                  <strong>{req.agent_role}</strong> ({req.agent_id}) yêu cầu thực thi{" "}
                  <code style={{ color: "var(--text-primary)" }}>{req.tool_name}</code>
                </div>
                <div style={{ display: "flex", gap: "6px" }}>
                  <button
                    type="button"
                    onClick={() => onRespondApproval(req.approval_id, true, false)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "3px",
                      padding: "3px 8px",
                      borderRadius: "5px",
                      border: "none",
                      background: "var(--text-primary)",
                      color: "var(--bg-primary)",
                      fontSize: "11px",
                      fontWeight: 600,
                      cursor: "pointer",
                    }}
                  >
                    <Check size={12} /> Phê duyệt
                  </button>
                  <button
                    type="button"
                    onClick={() => onRespondApproval(req.approval_id, true, true)}
                    style={{
                      padding: "3px 8px",
                      borderRadius: "5px",
                      border: "1px solid var(--border-primary)",
                      background: "transparent",
                      color: "var(--text-primary)",
                      fontSize: "11px",
                      cursor: "pointer",
                    }}
                  >
                    Phê duyệt tất cả
                  </button>
                  <button
                    type="button"
                    onClick={() => onRespondApproval(req.approval_id, false, false)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "3px",
                      padding: "3px 8px",
                      borderRadius: "5px",
                      border: "1px solid var(--border-primary)",
                      background: "var(--bg-primary)",
                      color: "var(--text-secondary)",
                      fontSize: "11px",
                      fontWeight: 600,
                      cursor: "pointer",
                    }}
                  >
                    <X size={12} /> Từ chối
                  </button>
                </div>
              </div>
              <pre
                style={{
                  margin: 0,
                  padding: "6px",
                  borderRadius: "4px",
                  background: "var(--bg-secondary)",
                  fontSize: "10.5px",
                  maxHeight: "90px",
                  overflowY: "auto",
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                  color: "var(--text-secondary)",
                }}
              >
                {req.args_preview}
              </pre>
            </div>
          ))}
        </div>
      )}

      {/* Workers Section */}
      {snapshot.workers.length > 0 && (
        <div>
          <button
            type="button"
            onClick={() => setShowWorkersGrid(!showWorkersGrid)}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "4px",
              background: "transparent",
              border: "none",
              color: "var(--text-secondary)",
              fontSize: "11.5px",
              fontWeight: 600,
              cursor: "pointer",
              padding: "2px 0",
              marginBottom: "6px",
            }}
          >
            {showWorkersGrid ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
            <span>Danh sách Worker Agents ({snapshot.workers.length})</span>
          </button>

          {showWorkersGrid && (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))",
                gap: "6px",
                maxHeight: "340px",
                overflowY: "auto",
              }}
            >
              {snapshot.workers.map((w) => {
                const st = STATUS_STYLE[w.status] || STATUS_STYLE.queued;
                const isExp = expandedWorkerId === w.id;
                return (
                  <div
                    key={w.id}
                    onClick={() => setExpandedWorkerId(isExp ? null : w.id)}
                    style={{
                      padding: "7px 9px",
                      borderRadius: "7px",
                      background: "var(--bg-primary)",
                      border: `1px solid ${isExp ? st.color : "var(--border-primary)"}`,
                      cursor: "pointer",
                      display: "flex",
                      flexDirection: "column",
                      gap: "4px",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        gap: "6px",
                      }}
                    >
                      <span
                        style={{
                          fontSize: "11.5px",
                          fontWeight: 600,
                          color: "var(--text-primary)",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                        title={`${w.role} (${w.id})`}
                      >
                        {w.role}
                      </span>
                      <span
                        style={{
                          fontSize: "9.5px",
                          padding: "1px 6px",
                          borderRadius: "999px",
                          background: st.bg,
                          color: st.color,
                          fontWeight: 600,
                          flexShrink: 0,
                        }}
                      >
                        {st.label}
                      </span>
                    </div>

                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        fontSize: "10px",
                        color: "var(--text-muted)",
                      }}
                    >
                      <span>
                        {w.mode} · lượt {w.iteration}/{w.max_iterations}
                      </span>
                      {w.files_touched.length > 0 && (
                        <span
                          style={{ display: "flex", alignItems: "center", gap: "2px" }}
                        >
                          <FileCode2 size={10} /> {w.files_touched.length}
                        </span>
                      )}
                    </div>

                    <div
                      style={{
                        fontSize: "10.5px",
                        color: "var(--text-secondary)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {w.current_action}
                    </div>

                    {isExp && (
                      <div
                        style={{
                          marginTop: "4px",
                          paddingTop: "4px",
                          borderTop: "1px solid var(--border-primary)",
                          fontSize: "10px",
                          color: "var(--text-secondary)",
                          display: "flex",
                          flexDirection: "column",
                          gap: "3px",
                        }}
                      >
                        {w.files_touched.length > 0 && (
                          <div style={{ color: "var(--text-primary)" }}>
                            Files: {w.files_touched.join(", ")}
                          </div>
                        )}
                        {w.logs.slice(-6).map((line, idx) => (
                          <div key={idx} style={{ opacity: 0.85 }}>
                            {line}
                          </div>
                        ))}
                        {w.summary && (
                          <div
                            style={{
                              marginTop: "2px",
                              color: "var(--text-primary)",
                              whiteSpace: "pre-wrap",
                            }}
                          >
                            {w.summary}
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
