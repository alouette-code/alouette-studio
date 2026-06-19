import React, { useMemo } from "react";
import { FileCode, FileJson, Image as ImageIcon, Layers, SearchCode } from "lucide-react";
import type { ApiResponse } from "./PingZeroTypes";

interface PingZeroSourceCatcherProps {
  responseInfo?: ApiResponse;
}

interface ParsedResource {
  type: string;
  url: string;
  tag: string;
  order: number;
  attributes?: Record<string, string>;
}

export default function PingZeroSourceCatcher({ responseInfo }: PingZeroSourceCatcherProps) {
  const parsedData = useMemo(() => {
    if (!responseInfo?.body) return null;

    const body = responseInfo.body.trim();
    // Quick check if it might be HTML
    const isHtml = /^<(html|!doctype html)/i.test(body) || body.toLowerCase().includes("<head>");
    if (!isHtml) return null;

    try {
      const parser = new DOMParser();
      const doc = parser.parseFromString(body, "text/html");
      
      const resources: ParsedResource[] = [];
      let orderCounter = 1;

      // Title
      const title = doc.title || "(No Title)";

      // Meta tags
      const metas: { name: string, content: string }[] = [];
      doc.querySelectorAll("meta").forEach((m) => {
        const name = m.getAttribute("name") || m.getAttribute("property") || m.getAttribute("charset");
        const content = m.getAttribute("content");
        if (name) metas.push({ name, content: content || "" });
      });

      // Scripts
      doc.querySelectorAll("script").forEach((s) => {
        const src = s.getAttribute("src");
        if (src) {
          resources.push({
            type: "script",
            url: src,
            tag: "<script>",
            order: orderCounter++
          });
        }
      });

      // Stylesheets
      doc.querySelectorAll("link[rel='stylesheet']").forEach((l) => {
        const href = l.getAttribute("href");
        if (href) {
          resources.push({
            type: "stylesheet",
            url: href,
            tag: "<link>",
            order: orderCounter++
          });
        }
      });

      // Images
      doc.querySelectorAll("img").forEach((img) => {
        const src = img.getAttribute("src");
        if (src && !src.startsWith("data:image")) { // Skip inline base64
          resources.push({
            type: "image",
            url: src,
            tag: "<img>",
            order: orderCounter++,
            attributes: { alt: img.getAttribute("alt") || "" }
          });
        }
      });

      return { title, metas, resources };
    } catch (e) {
      console.error("DOMParser error", e);
      return null;
    }
  }, [responseInfo]);

  if (!parsedData) {
    return (
      <div style={{ padding: "20px", textAlign: "center", color: "var(--text-muted)", fontStyle: "italic", fontSize: "13px" }}>
        No valid HTML document detected. Please ensure the response is an HTML page to use the Source Catcher.
      </div>
    );
  }

  const scripts = parsedData.resources.filter(r => r.type === "script");
  const styles = parsedData.resources.filter(r => r.type === "stylesheet");
  const images = parsedData.resources.filter(r => r.type === "image");

  const renderResourceTable = (title: string, icon: React.ReactNode, items: ParsedResource[], color: string) => {
    if (items.length === 0) return null;
    
    return (
      <div style={{ marginBottom: "24px" }}>
        <h4 style={{ fontSize: "12px", fontWeight: "bold", color, display: "flex", alignItems: "center", gap: "6px", textTransform: "uppercase", letterSpacing: "1px", marginBottom: "8px", borderBottom: `1px solid var(--border-primary)`, paddingBottom: "4px" }}>
          {icon} {title} ({items.length})
        </h4>
        <div style={{ background: "var(--bg-tertiary, rgba(0,0,0,0.2))", borderRadius: "6px", border: "1px solid var(--border-primary)", overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "12px" }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary, rgba(255,255,255,0.05))", textAlign: "left", color: "var(--text-secondary)" }}>
                <th style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-primary)", width: "50px" }}>Order</th>
                <th style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-primary)" }}>Path / URL</th>
              </tr>
            </thead>
            <tbody>
              {items.map(item => (
                <tr key={`${item.type}-${item.order}`} style={{ borderBottom: "1px solid var(--border-primary, rgba(255,255,255,0.05))" }}>
                  <td style={{ padding: "6px 12px", color: "var(--text-muted)", fontFamily: "monospace" }}>#{item.order}</td>
                  <td style={{ padding: "6px 12px", color: "var(--text-primary)", wordBreak: "break-all" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                      {item.url}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    );
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", color: "var(--text-primary)" }}>
      {/* Header */}
      <div style={{ padding: "10px 16px", borderBottom: "1px solid var(--border-primary)", display: "flex", alignItems: "center", gap: "8px", background: "var(--bg-secondary)" }}>
        <SearchCode size={16} style={{ color: "var(--color-warning)" }} />
        <h3 style={{ margin: 0, fontWeight: "bold", fontSize: "13px", textTransform: "uppercase", letterSpacing: "0.5px" }}>
          HTML Source Inspector
        </h3>
        <span style={{ marginLeft: "auto", fontSize: "12px", color: "var(--text-muted)" }}>
          <span style={{ color: "var(--text-secondary)" }}>Title:</span> {parsedData.title}
        </span>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: "16px" }} className="custom-scrollbar">
        
        {/* Meta tags */}
        {parsedData.metas.length > 0 && (
          <div style={{ marginBottom: "24px" }}>
            <h4 style={{ fontSize: "12px", fontWeight: "bold", color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "6px", textTransform: "uppercase", letterSpacing: "1px", marginBottom: "8px", borderBottom: `1px solid var(--border-primary)`, paddingBottom: "4px" }}>
              <Layers size={14} /> Meta Configuration ({parsedData.metas.length})
            </h4>
            <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
              {parsedData.metas.map((m, i) => (
                <div key={i} style={{ fontSize: "11px", background: "var(--bg-tertiary, rgba(255,255,255,0.05))", border: "1px solid var(--border-primary)", padding: "4px 8px", borderRadius: "4px", display: "flex", flexDirection: "column" }}>
                  <span style={{ color: "var(--color-info)", fontWeight: "bold", marginBottom: "2px" }}>{m.name}</span>
                  <span style={{ color: "var(--text-muted)", maxWidth: "300px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={m.content}>{m.content}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Resources */}
        {renderResourceTable("Javascript Sources", <FileJson size={14} />, scripts, "var(--color-warning)")}
        {renderResourceTable("CSS Stylesheets", <FileCode size={14} />, styles, "var(--color-info)")}
        {renderResourceTable("Image Assets", <ImageIcon size={14} />, images, "var(--color-success)")}

        {scripts.length === 0 && styles.length === 0 && images.length === 0 && (
          <div style={{ textAlign: "center", padding: "20px", color: "var(--text-muted)", fontStyle: "italic", fontSize: "13px" }}>
            No external resources (JS, CSS, Images) found in this document.
          </div>
        )}
      </div>
    </div>
  );
}
