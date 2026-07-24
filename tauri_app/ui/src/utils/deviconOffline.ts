import { deviconOfflineMap } from "./deviconOfflineData";

// Initialize fetch interceptor for devicon CDN requests so they run 100% offline
export function initDeviconOfflineInterceptor() {
  if (typeof window === "undefined" || !window.fetch) return;

  const windowWithFlag = window as unknown as { _deviconInterceptorInitialized?: boolean };
  if (windowWithFlag._deviconInterceptorInitialized) return;
  windowWithFlag._deviconInterceptorInitialized = true;

  const originalFetch = window.fetch;
  window.fetch = async function (input: RequestInfo | URL, init?: RequestInit) {
    const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
    if (url && url.includes("cdn.jsdelivr.net/gh/devicons/devicon")) {
      const match = url.match(/\/icons\/([^/]+)\/([^/]+)-(plain|original|line|line-woodmark|original-woodmark|plain-woodmark)\.svg/);
      if (match) {
        const icon = match[1];
        const style = match[3];
        const key = `${icon}-${style}`;
        const fallbackKey = `${icon}-plain` in deviconOfflineMap ? `${icon}-plain` : `${icon}-original`;
        const svgContent = deviconOfflineMap[key] || deviconOfflineMap[fallbackKey];
        if (svgContent) {
          return new Response(svgContent, {
            status: 200,
            headers: { "Content-Type": "image/svg+xml" },
          });
        }
      }
    }
    return originalFetch.call(this, input, init);
  };
}

// Map file names / extensions to react-simple-devicons icon names and styles
export interface DevIconInfo {
  icon: string;
  style: "plain" | "original";
  color?: string;
}

export function getDevIconInfoForFile(fileName: string): DevIconInfo | null {
  const lower = fileName.toLowerCase();

  // Special file names
  if (lower === "package.json" || lower === "package-lock.json") return { icon: "npm", style: "plain" };
  if (lower === "dockerfile" || lower.startsWith("dockerfile.") || lower === "docker-compose.yml" || lower === "docker-compose.yaml") return { icon: "docker", style: "plain" };
  if (lower === ".gitignore" || lower === ".gitmodules" || lower === ".gitattributes") return { icon: "git", style: "plain" };
  if (lower === "tailwind.config.js" || lower === "tailwind.config.ts" || lower === "tailwind.config.cjs" || lower === "tailwind.config.mjs") return { icon: "tailwindcss", style: "original" };

  // File extension based
  const parts = lower.split(".");
  if (parts.length <= 1) return null;
  const ext = parts.pop()!;

  switch (ext) {
    case "js":
    case "mjs":
    case "cjs":
      return { icon: "javascript", style: "plain" };
    case "ts":
    case "mts":
    case "cts":
      return { icon: "typescript", style: "plain" };
    case "tsx":
    case "jsx":
      return { icon: "react", style: "original" };
    case "py":
    case "pyw":
      return { icon: "python", style: "plain" };
    case "rs":
      return { icon: "rust", style: "original" };
    case "go":
      return { icon: "go", style: "plain" };
    case "java":
      return { icon: "java", style: "plain" };
    case "c":
    case "h":
      return { icon: "c", style: "original" };
    case "cpp":
    case "hpp":
    case "cc":
    case "cxx":
      return { icon: "cplusplus", style: "plain" };
    case "cs":
      return { icon: "csharp", style: "plain" };
    case "html":
    case "htm":
      return { icon: "html5", style: "plain" };
    case "css":
      return { icon: "css3", style: "plain" };
    case "scss":
    case "sass":
      return { icon: "sass", style: "original" };
    case "php":
      return { icon: "php", style: "plain" };
    case "rb":
      return { icon: "ruby", style: "plain" };
    case "swift":
      return { icon: "swift", style: "plain" };
    case "kt":
    case "kts":
      return { icon: "kotlin", style: "plain" };
    case "vue":
      return { icon: "vuejs", style: "plain" };
    case "svelte":
      return { icon: "svelte", style: "plain" };
    case "sh":
    case "bash":
    case "zsh":
      return { icon: "bash", style: "plain" };
    case "sql":
      return { icon: "sqldeveloper", style: "plain" };
    case "json":
    case "jsonc":
    case "json5":
      return { icon: "json", style: "plain" };
    case "md":
    case "markdown":
      return { icon: "markdown", style: "original" };
    case "yml":
    case "yaml":
      return { icon: "yaml", style: "plain" };
    case "xml":
      return { icon: "xml", style: "plain" };
    case "graphql":
    case "gql":
      return { icon: "graphql", style: "plain" };
    default:
      return null;
  }
}

export function getDevIconSvgForFile(fileName: string): string | null {
  const info = getDevIconInfoForFile(fileName);
  if (!info) return null;
  const key = `${info.icon}-${info.style}`;
  const fallbackKey = `${info.icon}-plain` in deviconOfflineMap ? `${info.icon}-plain` : `${info.icon}-original`;
  return deviconOfflineMap[key] || deviconOfflineMap[fallbackKey] || null;
}
