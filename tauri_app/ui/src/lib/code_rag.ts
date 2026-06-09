import { invoke } from "@tauri-apps/api/core";

export interface CodeRagFunction {
  id: string;
  func_name: string;
  signature: string;
  docstring: string | null;
  file_path: string;
  lang_id: string;
  project_id: string;
  line_start: number;
  line_end: number;
  normalized_text: string;
}

export interface CodeRagQueryResult {
  matches: Array<{
    entry: CodeRagFunction;
    score: number;
  }>;
  elapsed_ms: number;
  candidates_count: number;
  error: string | null;
}

export interface CodeRagLanguage {
  lang_id: string;
  display_name: string;
  tier: string;
  extensions: string[];
}

export interface CodeRagStats {
  total_files_indexed: number;
  total_functions_extracted: number;
  total_errors: number;
  total_entries: number;
}

/** Lấy danh sách ngôn ngữ được hỗ trợ */
export async function getSupportedLanguages(): Promise<CodeRagLanguage[]> {
  return invoke("code_rag_supported_languages");
}

/** Query function bằng text */
export async function queryCodeRag(
  query: string,
  langId?: string,
  projectId?: string,
  topK?: number,
): Promise<CodeRagQueryResult> {
  return invoke("code_rag_query", {
    query,
    langId: langId ?? null,
    projectId: projectId ?? null,
    topK: topK ?? 10,
  });
}

/** Query function bằng tên */
export async function queryCodeRagByName(
  name: string,
  langId?: string,
  projectId?: string,
  topK?: number,
): Promise<CodeRagFunction[]> {
  return invoke("code_rag_query_by_name", {
    name,
    langId: langId ?? null,
    projectId: projectId ?? null,
    topK: topK ?? 10,
  });
}

/** Index một file */
export async function indexFile(
  path: string,
  projectId: string,
): Promise<void> {
  return invoke("code_rag_index_file", { path, projectId });
}

/** Re-scan toàn bộ project (bất đồng bộ qua event queue) */
export async function rescanProject(
  projectId: string,
  basePath: string,
): Promise<void> {
  return invoke("code_rag_rescan_project", { projectId, basePath });
}

/** Scan và index đồng bộ một thư mục — gọi khi mở project */
export async function scanDirectory(
  projectId: string,
  basePath: string,
): Promise<number> {
  return invoke("code_rag_scan_directory", { projectId, basePath });
}

/** Xóa index của project */
export async function deleteProjectIndex(projectId: string): Promise<void> {
  return invoke("code_rag_delete_project", { projectId });
}

/** Lấy stats */
export async function getCodeRagStats(): Promise<CodeRagStats> {
  return invoke("code_rag_stats");
}

/** Resolve ngôn ngữ */
export async function resolveLanguage(
  filePath: string,
  content: string,
): Promise<{ lang_id: string; display_name: string; tier: string } | null> {
  return invoke("code_rag_resolve_language", { filePath, content });
}

/** Extract functions từ code string */
export async function extractFunctions(
  content: string,
  filePath: string,
  projectId: string,
  langId: string,
): Promise<CodeRagFunction[]> {
  return invoke("code_rag_extract_functions", {
    content,
    filePath,
    projectId,
    langId,
  });
}
