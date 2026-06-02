import { Project, ProcessState } from "../types";

interface ExtendedDashboardProps {
  projects: Project[];
  projectStates: { [projectId: string]: ProcessState };
  resourceHistory: {
    [projectId: string]: {
      cpu: number[];
      ram: number[];
    };
  };
  chartType: "cpu" | "ram";
  setChartType: (type: "cpu" | "ram") => void;
  aiErrors: any[];
}

export default function ExtendedDashboard({
  projects,
  projectStates,
  resourceHistory,
  chartType,
  setChartType,
  aiErrors
}: ExtendedDashboardProps) {
  // Compute overall resource metrics for Bottom Bar (4)
  const runningProjsList = projects.filter(p => projectStates[p.id]?.type === "Running");
  const totalRunning = runningProjsList.length;
  
  let totalCpu = 0;
  let totalRam = 0;
  
  projects.forEach(p => {
    const state = projectStates[p.id]?.type;
    if (state === "Running" || state === "Setup") {
      const hist = resourceHistory[p.id] || { cpu: [], ram: [] };
      const currentCpu = hist.cpu.length > 0 ? hist.cpu[hist.cpu.length - 1] : 0;
      const currentRam = hist.ram.length > 0 ? hist.ram[hist.ram.length - 1] : 0;
      totalCpu += currentCpu;
      totalRam += currentRam;
    }
  });

  // Compute SVG paths for running projects and the total line (2)
  const maxLen = Math.max(2, ...runningProjsList.map(p => (resourceHistory[p.id]?.[chartType]?.length || 0)));
  
  // Sum total history at each point
  const totalHistory: number[] = Array.from({ length: maxLen }, (_, idx) => {
    let sum = 0;
    runningProjsList.forEach(p => {
      const hist = resourceHistory[p.id]?.[chartType] || [];
      // Align to end
      const offset = hist.length - maxLen + idx;
      if (offset >= 0 && offset < hist.length) {
        sum += hist[offset];
      }
    });
    return sum;
  });

  const chartW = 480;
  const chartH = 120;
  const startX = 45;
  const startY = 145; // bottom of the chart
  
  // Scale values
  const allHistoryValues = [
    ...totalHistory,
    ...runningProjsList.flatMap(p => resourceHistory[p.id]?.[chartType] || [])
  ];
  const maxValInHistory = allHistoryValues.length > 0 ? Math.max(10, ...allHistoryValues) : 100;
  const chartMaxY = chartType === "cpu" ? 100 : maxValInHistory * 1.15; // 15% headroom

  const getSvgPath = (points: number[]) => {
    if (points.length < 2) return "";
    return points.map((val, idx) => {
      const x = startX + (idx / (points.length - 1)) * chartW;
      const y = startY - (val / chartMaxY) * chartH;
      return `${idx === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
    }).join(" ");
  };

  // Monochrome strokes for minimalist look: clean white, lighter gray, darker gray
  const getMonochromeStroke = (index: number) => {
    const STROKES = ["#a1a1aa", "#71717a", "#52525b", "#3f3f46"];
    return STROKES[index % STROKES.length];
  };

  return (
    <div className="extended-dashboard monochrome">
      {/* Left Column */}
      <div className="dashboard-left">
        {/* Section 1: Imported Projects list & status & resource consumption */}
        <div className="dashboard-section-1">
          <header className="dash-sec-header">
            <h3 className="dash-sec-title">
              <span>PROJECTS & RESOURCES</span>
            </h3>
            <span className="diagnostics-badge-header">
              TOTAL: {projects.length}
            </span>
          </header>

          <div className="dash-project-grid">
            {projects.map((p) => {
              const state = projectStates[p.id] || { type: "Stopped" };
              const isRunning = state.type === "Running";
              const hist = resourceHistory[p.id] || { cpu: [], ram: [] };
              const curCpu = hist.cpu.length > 0 ? hist.cpu[hist.cpu.length - 1] : 0;
              const curRam = hist.ram.length > 0 ? hist.ram[hist.ram.length - 1] : 0;

              return (
                <div key={p.id} className="dash-project-card monochrome">
                  <div className="dash-card-top">
                    <span className="dash-card-name">{p.name}</span>
                    <div className="dash-status-dot">
                      <span className={`dash-dot ${isRunning ? "running" : "stopped"}`} />
                      <span className="dash-status-text">
                        {state.type}
                      </span>
                    </div>
                  </div>

                  <div className="dash-card-metrics">
                    <div className="dash-metric-item">
                      <span className="dash-metric-label">CPU</span>
                      <span className="dash-metric-value">{isRunning ? `${curCpu.toFixed(1)}%` : "0.0%"}</span>
                      <div className="progress-bar-wrapper">
                        <div
                          className="progress-bar-fill cpu"
                          style={{ width: `${isRunning ? Math.min(100, curCpu) : 0}%`, transition: "width 0.3s ease" }}
                        />
                      </div>
                    </div>
                    <div className="dash-metric-item">
                      <span className="dash-metric-label">RAM</span>
                      <span className="dash-metric-value">{isRunning ? `${curRam.toFixed(1)} MB` : "0 MB"}</span>
                      <div className="progress-bar-wrapper">
                        <div
                          className="progress-bar-fill ram"
                          style={{
                            width: `${isRunning ? (p.max_ram_mb ? Math.min(100, (curRam / p.max_ram_mb) * 100) : Math.min(100, (curRam / 2000) * 100)) : 0}%`,
                            transition: "width 0.3s ease"
                          }}
                        />
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Section 2: Resource detail charts (SVG) */}
        <div className="dashboard-section-2">
          <header className="dash-sec-header">
            <h3 className="dash-sec-title">
              <span>RESOURCE UTILIZATION ({chartType.toUpperCase()})</span>
            </h3>
            <div className="dash-sec-actions">
              <button
                className={`btn-top-box ${chartType === "cpu" ? "active" : ""}`}
                onClick={() => setChartType("cpu")}
              >
                CPU
              </button>
              <button
                className={`btn-top-box ${chartType === "ram" ? "active" : ""}`}
                onClick={() => setChartType("ram")}
              >
                RAM
              </button>
            </div>
          </header>

          <div className="dash-chart-container">
            <svg className="dash-chart-svg" viewBox="0 0 550 180">
              {/* Grid lines */}
              <line x1="45" y1="25" x2="525" y2="25" className="chart-grid-line" />
              <line x1="45" y1="65" x2="525" y2="65" className="chart-grid-line" />
              <line x1="45" y1="105" x2="525" y2="105" className="chart-grid-line" />
              <line x1="45" y1="145" x2="525" y2="145" className="chart-axis-line" />
              <line x1="45" y1="25" x2="45" y2="145" className="chart-axis-line" />

              {/* Y axis labels */}
              <text x="35" y="30" textAnchor="end" className="chart-axis-label">{(chartMaxY).toFixed(0)}</text>
              <text x="35" y="85" textAnchor="end" className="chart-axis-label">{(chartMaxY / 2).toFixed(0)}</text>
              <text x="35" y="148" textAnchor="end" className="chart-axis-label">0</text>

              {/* Project Lines */}
              {runningProjsList.map((p, idx) => {
                const hist = resourceHistory[p.id]?.[chartType] || [];
                if (hist.length < 2) return null;
                return (
                  <path
                    key={p.id}
                    d={getSvgPath(hist)}
                    fill="none"
                    stroke={getMonochromeStroke(idx)}
                    strokeWidth="1.2"
                    style={{ transition: "d 0.3s ease" }}
                  />
                );
              })}

              {/* Total Line (Solid bold white for minimalist style) */}
              {totalHistory.length >= 2 && (
                <path
                  d={getSvgPath(totalHistory)}
                  fill="none"
                  stroke="#ffffff"
                  strokeWidth="2"
                  style={{ transition: "d 0.3s ease" }}
                />
              )}
            </svg>
          </div>

          <div className="chart-legend">
            {runningProjsList.map((p, idx) => (
              <div key={p.id} className="legend-item">
                <span className="legend-color" style={{ backgroundColor: getMonochromeStroke(idx) }} />
                <span>{p.name}</span>
              </div>
            ))}
            {totalHistory.length >= 2 && (
              <div className="legend-item">
                <span className="legend-color" style={{ backgroundColor: "#ffffff" }} />
                <span style={{ fontWeight: "600" }}>TOTAL USAGE</span>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Right Column: AI Error feed (3) */}
      <div className="dashboard-section-3">
        <header className="dash-sec-header">
          <h3 className="dash-sec-title">
            <span>AI DIAGNOSTICS & ALERTS</span>
          </h3>
        </header>

        <div className="dash-error-list">
          {aiErrors.map((err) => (
            <div key={err.id} className="dash-error-card monochrome" style={{ position: "relative" }}>
              <div className="dash-error-title-row" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <span className="dash-error-proj" style={{ marginRight: "8px" }}>{err.project}</span>
                  <span style={{
                    fontSize: "9px",
                    padding: "1px 5px",
                    backgroundColor: "rgba(239, 68, 68, 0.1)",
                    border: "1px solid rgba(239, 68, 68, 0.2)",
                    borderRadius: "3px",
                    color: "#f43f5e",
                    fontWeight: 600
                  }}>
                    {err.type}
                  </span>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span className="dash-error-time">{err.timestamp}</span>
                  <button
                    onClick={() => {
                      navigator.clipboard.writeText(err.message);
                    }}
                    style={{
                      background: "rgba(255, 255, 255, 0.05)",
                      border: "1px solid rgba(255, 255, 255, 0.1)",
                      borderRadius: "3px",
                      color: "#a1a1aa",
                      padding: "2px 6px",
                      fontSize: "9px",
                      cursor: "pointer",
                      fontWeight: 600
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.backgroundColor = "rgba(255, 255, 255, 0.1)";
                      e.currentTarget.style.color = "#ffffff";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.backgroundColor = "rgba(255, 255, 255, 0.05)";
                      e.currentTarget.style.color = "#a1a1aa";
                    }}
                  >
                    Copy
                  </button>
                </div>
              </div>
              <p className="dash-error-msg" style={{ color: "#ef4444", fontFamily: "Consolas, monospace", fontSize: "11px", marginTop: "8px", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>{err.message}</p>
            </div>
          ))}
          {aiErrors.length === 0 && (
            <div className="empty-zone-message" style={{ height: "100%" }}>
              <span>NO ACTIVE REPORTS</span>
            </div>
          )}
        </div>
      </div>

      {/* Bottom Bar: Total Resource Consumption (4) */}
      <div className="dashboard-bottom">
        <div className="bottom-metric-capsule">
          <div className="bottom-metric">
            <span className="bottom-metric-label">PROCESSES:</span>
            <span className="bottom-metric-value">{totalRunning} ACTIVE</span>
          </div>
        </div>

        <div className="bottom-metric-capsule" style={{ gap: "16px" }}>
          <div className="bottom-metric">
            <span className="bottom-metric-label">TOTAL CPU:</span>
            <span className="bottom-metric-value">
              {totalCpu.toFixed(1)}%
            </span>
          </div>
          <div className="bottom-metric">
            <span className="bottom-metric-label">TOTAL RAM:</span>
            <span className="bottom-metric-value">
              {totalRam.toFixed(1)} MB
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
