import React, { useEffect, useId, useRef, useState } from "react";
import mermaid from "mermaid";
import DOMPurify from "dompurify";
import {
  Maximize2,
  X,
  ZoomIn,
  ZoomOut,
  RotateCcw,
  Code2,
  Network,
  Copy,
  Check,
  Download,
  MousePointerClick,
} from "lucide-react";

let mermaidInitialized = false;

function ensureMermaidInit() {
  if (mermaidInitialized) return;
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: "base",
    fontFamily: "var(--font-sans), Inter, system-ui, sans-serif",
    flowchart: {
      useMaxWidth: true,
      htmlLabels: false,
      curve: "basis",
      padding: 14,
      nodeSpacing: 36,
      rankSpacing: 44,
    },
    sequence: {
      useMaxWidth: true,
      actorMargin: 36,
      messageMargin: 28,
    },
    themeVariables: {
      darkMode: true,
      background: "transparent",
      primaryColor: "#1e293b",
      primaryTextColor: "#f1f5f9",
      primaryBorderColor: "#6366f1",
      secondaryColor: "#0f172a",
      secondaryTextColor: "#cbd5e1",
      secondaryBorderColor: "#3b82f6",
      tertiaryColor: "#18181b",
      tertiaryTextColor: "#e2e8f0",
      tertiaryBorderColor: "#475569",
      lineColor: "#64748b",
      textColor: "#e2e8f0",
      mainBkg: "#1e293b",
      nodeBorder: "#6366f1",
      clusterBkg: "rgba(99, 102, 241, 0.08)",
      clusterBorder: "rgba(99, 102, 241, 0.4)",
      edgeLabelBackground: "#0f172a",
      fontSize: "12px",
    },
  });
  mermaidInitialized = true;
}

interface MermaidDiagramProps {
  chart: string;
}

export const MermaidDiagram: React.FC<MermaidDiagramProps> = ({ chart }) => {
  const reactId = useId().replace(/:/g, "_");
  const [svgHtml, setSvgHtml] = useState<string>("");
  const [renderError, setRenderError] = useState<string | null>(null);
  const [showSourceInline, setShowSourceInline] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [modalTab, setModalTab] = useState<"diagram" | "source">("diagram");
  const [copied, setCopied] = useState(false);

  // Interactive Pan & Zoom state in modal
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const dragStartRef = useRef<{ x: number; y: number; panX: number; panY: number } | null>(null);
  const [selectedNodeText, setSelectedNodeText] = useState<string | null>(null);
  const modalSvgContainerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let cancelled = false;
    const trimmed = chart.trim();
    if (!trimmed) return;

    ensureMermaidInit();

    const renderAsync = async () => {
      try {
        const renderId = `alouette_mmd_${reactId}_${Date.now()}`;
        const { svg } = await mermaid.render(renderId, trimmed);
        if (cancelled) return;
        const cleanSvg = DOMPurify.sanitize(svg, {
          USE_PROFILES: { svg: true, svgFilters: true },
        });
        setSvgHtml(cleanSvg);
        setRenderError(null);
      } catch (err: any) {
        if (cancelled) return;
        setRenderError(err?.message || "Invalid Mermaid syntax");
      }
    };

    const timer = setTimeout(renderAsync, 80);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [chart, reactId]);

  const handleCopy = () => {
    navigator.clipboard?.writeText(chart.trim());
    setCopied(true);
    setTimeout(() => setCopied(false), 1600);
  };

  const handleDownloadSvg = () => {
    if (!svgHtml) return;
    const blob = new Blob([svgHtml], { type: "image/svg+xml;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "architecture-diagram.svg";
    a.click();
    URL.revokeObjectURL(url);
  };

  const openInteractiveModal = () => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
    setSelectedNodeText(null);
    setModalTab("diagram");
    setModalOpen(true);
  };

  // Click any node inside rendered SVG to highlight & inspect its label
  const handleSvgClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement | null;
    if (!target) return;
    const nodeGroup = target.closest("g.node, g.cluster, g.actor") as SVGGElement | null;
    if (!nodeGroup || !modalSvgContainerRef.current) return;

    modalSvgContainerRef.current
      .querySelectorAll(".mmd-node-selected")
      .forEach((el) => el.classList.remove("mmd-node-selected"));
    nodeGroup.classList.add("mmd-node-selected");

    const text = (nodeGroup.textContent || "").replace(/\s+/g, " ").trim();
    if (text) {
      setSelectedNodeText(text);
    }
  };

  // Fallback gracefully during streaming or syntax error
  if (renderError && !svgHtml) {
    return (
      <pre className="agent-code-block">
        <code>{chart}</code>
      </pre>
    );
  }

  return (
    <>
      <div className="agent-diagram-card">
        <div className="agent-diagram-header">
          <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
            <Network size={12} style={{ color: "#818cf8" }} />
            <span>Sơ đồ kiến trúc</span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
            <button
              type="button"
              className="agent-diagram-btn"
              onClick={() => setShowSourceInline((v) => !v)}
              title={showSourceInline ? "Xem sơ đồ" : "Xem mã Mermaid"}
            >
              {showSourceInline ? <Network size={11} /> : <Code2 size={11} />}
            </button>
            <button
              type="button"
              className="agent-diagram-btn"
              onClick={handleCopy}
              title="Sao chép mã Mermaid"
            >
              {copied ? <Check size={11} style={{ color: "#10b981" }} /> : <Copy size={11} />}
            </button>
            <button
              type="button"
              className="agent-diagram-btn primary"
              onClick={openInteractiveModal}
              title="Bấm để phóng to & tương tác sơ đồ"
            >
              <Maximize2 size={11} />
              <span>Xem lớn</span>
            </button>
          </div>
        </div>

        {showSourceInline ? (
          <pre className="agent-code-block" style={{ margin: 0, border: "none", borderRadius: 0 }}>
            <code>{chart.trim()}</code>
          </pre>
        ) : (
          <div
            className="agent-diagram-preview"
            onClick={openInteractiveModal}
            title="Bấm vào sơ đồ để phóng to, zoom & kéo thả"
          >
            <div
              className="agent-diagram-svg-wrap"
              dangerouslySetInnerHTML={{ __html: svgHtml }}
            />
            <div className="agent-diagram-hover-hint">
              <Maximize2 size={11} />
              <span>Bấm để phóng to & tương tác</span>
            </div>
          </div>
        )}
      </div>

      {modalOpen && (
        <div className="agent-diagram-modal-overlay" onClick={() => setModalOpen(false)}>
          <div className="agent-diagram-modal" onClick={(e) => e.stopPropagation()}>
            <div className="agent-diagram-modal-toolbar">
              <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                <Network size={15} style={{ color: "#818cf8" }} />
                <span style={{ fontSize: "12.5px", fontWeight: 600 }}>
                  Sơ đồ kiến trúc tương tác
                </span>
                <div className="agent-diagram-tabs">
                  <button
                    type="button"
                    className={modalTab === "diagram" ? "active" : ""}
                    onClick={() => setModalTab("diagram")}
                  >
                    Sơ đồ
                  </button>
                  <button
                    type="button"
                    className={modalTab === "source" ? "active" : ""}
                    onClick={() => setModalTab("source")}
                  >
                    Mã nguồn
                  </button>
                </div>
              </div>

              <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                {modalTab === "diagram" && (
                  <>
                    <button
                      type="button"
                      className="agent-diagram-btn"
                      onClick={() => setZoom((z) => Math.max(0.4, +(z - 0.15).toFixed(2)))}
                      title="Thu nhỏ"
                    >
                      <ZoomOut size={13} />
                    </button>
                    <span style={{ fontSize: "11px", minWidth: "42px", textAlign: "center", color: "var(--text-secondary)" }}>
                      {Math.round(zoom * 100)}%
                    </span>
                    <button
                      type="button"
                      className="agent-diagram-btn"
                      onClick={() => setZoom((z) => Math.min(3, +(z + 0.15).toFixed(2)))}
                      title="Phóng to"
                    >
                      <ZoomIn size={13} />
                    </button>
                    <button
                      type="button"
                      className="agent-diagram-btn"
                      onClick={() => {
                        setZoom(1);
                        setPan({ x: 0, y: 0 });
                      }}
                      title="Đặt lại góc nhìn"
                    >
                      <RotateCcw size={13} />
                    </button>
                  </>
                )}
                <button
                  type="button"
                  className="agent-diagram-btn"
                  onClick={handleCopy}
                  title="Sao chép mã Mermaid"
                >
                  {copied ? <Check size={13} style={{ color: "#10b981" }} /> : <Copy size={13} />}
                </button>
                <button
                  type="button"
                  className="agent-diagram-btn"
                  onClick={handleDownloadSvg}
                  title="Tải xuống SVG"
                >
                  <Download size={13} />
                </button>
                <button
                  type="button"
                  className="agent-diagram-btn"
                  onClick={() => setModalOpen(false)}
                  title="Đóng"
                >
                  <X size={14} />
                </button>
              </div>
            </div>

            {modalTab === "source" ? (
              <div style={{ flex: 1, overflow: "auto", padding: "16px", background: "var(--bg-primary)" }}>
                <pre className="agent-code-block" style={{ margin: 0 }}>
                  <code>{chart.trim()}</code>
                </pre>
              </div>
            ) : (
              <div
                className="agent-diagram-canvas"
                style={{ cursor: isDragging ? "grabbing" : "grab" }}
                onWheel={(e) => {
                  e.preventDefault();
                  const delta = e.deltaY < 0 ? 0.1 : -0.1;
                  setZoom((z) => Math.min(3, Math.max(0.4, +(z + delta).toFixed(2))));
                }}
                onMouseDown={(e) => {
                  if (e.button !== 0) return;
                  setIsDragging(true);
                  dragStartRef.current = {
                    x: e.clientX,
                    y: e.clientY,
                    panX: pan.x,
                    panY: pan.y,
                  };
                }}
                onMouseMove={(e) => {
                  if (!isDragging || !dragStartRef.current) return;
                  setPan({
                    x: dragStartRef.current.panX + (e.clientX - dragStartRef.current.x),
                    y: dragStartRef.current.panY + (e.clientY - dragStartRef.current.y),
                  });
                }}
                onMouseUp={() => {
                  setIsDragging(false);
                  dragStartRef.current = null;
                }}
                onMouseLeave={() => {
                  setIsDragging(false);
                  dragStartRef.current = null;
                }}
              >
                <div
                  ref={modalSvgContainerRef}
                  onClick={handleSvgClick}
                  className="agent-diagram-modal-svg"
                  style={{
                    transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
                  }}
                  dangerouslySetInnerHTML={{ __html: svgHtml }}
                />
              </div>
            )}

            <div className="agent-diagram-modal-footer">
              <div style={{ display: "flex", alignItems: "center", gap: "6px", color: "var(--text-muted)", fontSize: "11px" }}>
                <MousePointerClick size={12} />
                {selectedNodeText ? (
                  <span>
                    Thành phần đang chọn:{" "}
                    <strong style={{ color: "#a5b4fc" }}>{selectedNodeText}</strong>
                  </span>
                ) : (
                  <span>Kéo để di chuyển • Lăn chuột để zoom • Bấm vào từng khối để chọn xem</span>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
