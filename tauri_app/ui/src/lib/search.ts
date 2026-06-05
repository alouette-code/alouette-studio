/**
 * Search Engine for Alouette Studio
 *
 * Module tìm kiếm chuyên dụng, scalable cho nhiều loại tìm kiếm.
 * Hiện tại: Agent History search với Fuse.js fuzzy search.
 * Architecture mở: thêm search type mới chỉ cần thêm intent + searcher.
 *
 * Features:
 * - Intent Detection (phát hiện người dùng muốn tìm gì)
 * - Fuzzy Search (Fuse.js với cấu hình tối ưu)
 * - Fallback Exact Search (nếu fuzzy không ra kết quả)
 * - Multi-field weighted search
 * - Sắp xếp kết quả thông minh (score + thời gian)
 */

import Fuse, { type IFuseOptions } from "fuse.js";

// ─── Types ───────────────────────────────────────────────────────────────────

export type SearchIntent = "agent_history" | "none";

export interface SearchResult<T = any> {
  intent: SearchIntent;
  filterText: string;
  results: T[];
  raw: T[];
  /** Tổng số item gốc (trước filter) */
  total: number;
}

export interface AgentHistoryItem {
  session_id: string;
  title: string;
  model: string;
  mode: string;
  created_at: number;
}

// ─── Intent Detection ────────────────────────────────────────────────────────
// Dùng includes() (giống code cũ) + regex mở rộng để extract filter text.
// Không dùng ^ anchor để match được ở mọi vị trí trong câu query.

const HISTORY_KEYWORDS = [
  "history agent",
  "history agen",
  "agent history",
  "agen history",
  "historyagent",
  "agenthistory",
  "ls agent",
  "ls history",
];

/**
 * Phát hiện và extract thông tin tìm kiếm từ query.
 * Dùng includes() để match linh hoạt (giống code cũ).
 */
export function detectSearchIntent(query: string): {
  intent: SearchIntent;
  filterText: string;
} {
  const qLower = query.toLowerCase().trim();

  // Step 1: Kiểm tra có chứa từ khóa history agent không (includes - linh hoạt)
  const hasHistoryKeyword = HISTORY_KEYWORDS.some((kw) => qLower.includes(kw));

  if (!hasHistoryKeyword) {
    return { intent: "none", filterText: "" };
  }

  // Step 2: Extract filter text (phần còn lại sau khi bỏ từ khóa)
  // Dùng regex để extract chính xác
  const extractPatterns = [
    /history\s+agent\s+(.+)/i,
    /history\s+agen\s+(.+)/i,
    /agent\s+history\s+(.+)/i,
    /agen\s+history\s+(.+)/i,
    /historyagent\s+(.+)/i,
    /agenthistory\s+(.+)/i,
    /ls\s+agent\s+(.+)/i,
    /ls\s+history\s+(.+)/i,
  ];

  for (const pattern of extractPatterns) {
    const match = qLower.match(pattern);
    if (match && match[1]?.trim()) {
      return {
        intent: "agent_history",
        filterText: match[1].trim(),
      };
    }
  }

  // Có keyword nhưng không có filter text → trả về tất cả
  return { intent: "agent_history", filterText: "" };
}

// ─── Fuzzy Search Engine (Fuse.js) ───────────────────────────────────────────

/**
 * Cấu hình Fuse.js mở rộng, tối ưu cho tìm kiếm agent history.
 * - threshold thấp = chính xác, cao = fuzzy nhiều
 * - keys: title quan trọng nhất (60%), model (25%), mode (15%)
 */
const defaultFuseOptions: IFuseOptions<AgentHistoryItem> = {
  keys: [
    { name: "title", weight: 0.7 },
    { name: "model", weight: 0.2 },
    { name: "mode", weight: 0.1 },
  ],
  // threshold: 0 = exact match, 1 = match tất cả
  // 0.4 = balance giữa chính xác và fuzzy
  threshold: 0.5,
  // Khoảng cách tối đa cho fuzzy matching
  distance: 200,
  // Số ký tự tối thiểu để bắt đầu match
  minMatchCharLength: 2,
  // Sắp xếp theo độ phù hợp
  shouldSort: true,
  // Bao gồm score để debug
  includeScore: true,
  // Bỏ qua khoảng trắng thừa
  ignoreLocation: false,
  // Ưu tiên match ở đầu field
  findAllMatches: true,
};

/**
 * Search Agent History với Fuse.js fuzzy search.
 * Nếu Fuse.js không ra kết quả, fallback sang exact substring search.
 */
export function searchAgentHistory(
  items: AgentHistoryItem[],
  filterText: string,
): AgentHistoryItem[] {
  if (!items || items.length === 0) return [];

  // Không có filter → trả về tất cả (sắp xếp mới nhất)
  if (!filterText) {
    return [...items].sort((a, b) => b.created_at - a.created_at);
  }

  // ── Chiến lược 1: Fuzzy Search (Fuse.js) ──
  const fuse = new Fuse(items, defaultFuseOptions);
  const fuseResults = fuse.search(filterText);

  if (fuseResults.length > 0) {
    return fuseResults.map((r) => r.item);
  }

  // ── Chiến lược 2: Fallback Exact Substring Search ──
  // Nếu Fuse.js không tìm thấy (có thể do tên ngắn hoặc ký tự đặc biệt)
  const fT = filterText.toLowerCase();
  const fallbackResults = items.filter((item) => {
    return (
      item.title?.toLowerCase().includes(fT) ||
      item.model?.toLowerCase().includes(fT) ||
      item.mode?.toLowerCase().includes(fT)
    );
  });

  if (fallbackResults.length > 0) {
    return fallbackResults.sort((a, b) => b.created_at - a.created_at);
  }

  // ── Chiến lược 3: Token-based Search ──
  // Tách filter text thành các token, tìm item nào match NHIỀU token nhất
  const tokens = fT.split(/\s+/).filter(Boolean);
  if (tokens.length > 1) {
    const scored = items.map((item) => {
      const searchable = [
        item.title?.toLowerCase() || "",
        item.model?.toLowerCase() || "",
        item.mode?.toLowerCase() || "",
      ].join(" ");
      const matchCount = tokens.filter((t) => searchable.includes(t)).length;
      return { item, score: matchCount / tokens.length };
    });

    const tokenResults = scored
      .filter((s) => s.score > 0)
      .sort((a, b) => {
        // Ưu tiên score cao, sau đó mới nhất
        if (b.score !== a.score) return b.score - a.score;
        return b.item.created_at - a.item.created_at;
      })
      .map((s) => s.item);

    if (tokenResults.length > 0) {
      return tokenResults;
    }
  }

  // Không tìm thấy gì
  return [];
}

/**
 * Full search pipeline: detect intent → search → trả về kết quả.
 */
export function searchAgentHistoryFull(
  query: string,
  items: AgentHistoryItem[],
): SearchResult<AgentHistoryItem> {
  const { intent, filterText } = detectSearchIntent(query);

  if (intent !== "agent_history") {
    return {
      intent,
      filterText,
      results: [],
      raw: items,
      total: items?.length || 0,
    };
  }

  const results = searchAgentHistory(items, filterText);
  return {
    intent,
    filterText,
    results,
    raw: items,
    total: items?.length || 0,
  };
}

// ─── Quick Search (alias nhanh) ──────────────────────────────────────────────
// Người dùng chỉ cần gõ "history agent <keyword>" là search được ngay.

/**
 * Kiểm tra nhanh xem query có phải đang tìm agent history không.
 * Dùng cho Header để quyết định hiển thị dropdown.
 */
export function isAgentHistorySearch(query: string): boolean {
  return detectSearchIntent(query).intent === "agent_history";
}

// ─── Multi-Search Architecture (mở rộng sau này) ─────────────────────────────
//
// Template cho search type mới:
//
// export type SearchIntent = "agent_history" | "projects" | "files" | "logs" | "none";
//
// export interface Searcher<T> {
//   intent: SearchIntent;
//   keywords: string[];
//   detect(query: string): boolean;
//   search(items: T[], filterText: string): T[];
// }
//
// export const searchers: Searcher<any>[] = [
//   agentHistorySearcher,
//   projectSearcher,
//   fileSearcher,
//   logSearcher,
// ];
//
// export function searchAll(query: string, data: { ... }) {
//   const activeSearchers = searchers.filter(s => s.detect(query));
//   return activeSearchers.map(s => ({
//     intent: s.intent,
//     results: s.search(data[s.intent], extractFilterText(query)),
//   }));
// }
