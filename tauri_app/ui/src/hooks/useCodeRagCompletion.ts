import { useCallback, useRef } from "react";
import { queryCodeRagByName } from "../lib/code_rag";

interface CacheEntry {
  data: any[];
  expiresAt: number;
}

const CACHE_TTL_MS = 30_000;
const MAX_CACHE_SIZE = 100;
const DEBOUNCE_MS = 120;
const MIN_WORD_LENGTH = 2;

export function useCodeRagCompletion() {
  const cacheRef = useRef<Map<string, CacheEntry>>(new Map());
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const currentRequestId = useRef(0);
  const lastKeystrokeRef = useRef(0);
  // Track register để debug
  const registeredLanguagesRef = useRef<string[]>([]);

  const registerCompletion = useCallback(
    (monaco: any, _editor: any, projectId?: string | null) => {
      console.log("[CodeRAG] Registering completion provider...");

      // Kiểm tra monaco API
      if (!monaco?.languages?.registerCompletionItemProvider) {
        console.error("[CodeRAG] Monaco languages API not available!");
        return { dispose: () => {} };
      }

      const languages = [
        "typescript",
        "javascript",
        "python",
        "rust",
        "go",
        "java",
        "cpp",
        "c",
        "php",
        "ruby",
        "swift",
        "kotlin",
        "scala",
        "lua",
        "r",
        "perl",
        "haskell",
        "sql",
        "sh",
      ];
      registeredLanguagesRef.current = languages;
      console.log("[CodeRAG] Registering for languages:", languages);
      console.log("[CodeRAG] projectId:", projectId);

      const disposable = monaco.languages.registerCompletionItemProvider(
        languages,
        {
          triggerCharacters: [".", " ", "(", "/", '"'],
          provideCompletionItems: async (model: any, position: any) => {
            const word = model.getWordUntilPosition(position);
            const currentWord = word ? word.word : "";
            const langId = model.getLanguageId() || "plaintext";

            console.log("[CodeRAG] provideCompletionItems triggered", {
              currentWord,
              langId,
              lineNumber: position.lineNumber,
              column: position.column,
              modelUri: model.uri?.toString(),
            });

            if (currentWord.length < MIN_WORD_LENGTH) {
              console.log("[CodeRAG] Word too short, skipping");
              return { suggestions: [] };
            }

            const cacheKey = `${langId}:${currentWord}`;

            // FAST PATH: cache hit
            const cached = cacheRef.current.get(cacheKey);
            if (cached && Date.now() < cached.expiresAt) {
              console.log(
                "[CodeRAG] Cache HIT for:",
                cacheKey,
                "->",
                cached.data.length,
                "suggestions",
              );
              return { suggestions: cached.data };
            }
            if (cached) {
              cacheRef.current.delete(cacheKey);
            }

            // Cancel previous request
            currentRequestId.current += 1;
            const myRequestId = currentRequestId.current;
            lastKeystrokeRef.current = Date.now();

            try {
              // Debounce
              await new Promise<void>((resolve) => {
                if (debounceRef.current) {
                  clearTimeout(debounceRef.current);
                }
                debounceRef.current = setTimeout(() => {
                  resolve();
                }, DEBOUNCE_MS);
              });

              if (myRequestId !== currentRequestId.current) {
                console.log("[CodeRAG] Request cancelled (newer request)");
                return { suggestions: [] };
              }

              if (Date.now() - lastKeystrokeRef.current < 50) {
                console.log("[CodeRAG] User still typing, skip");
                return { suggestions: [] };
              }

              console.log("[CodeRAG] Calling queryCodeRagByName:", {
                currentWord,
                langId,
                projectId,
              });

              let suggestions: any[] = [];

              try {
                const nameResults = await queryCodeRagByName(
                  currentWord,
                  langId,
                  projectId ?? undefined,
                  8,
                );

                console.log(
                  "[CodeRAG] Got results:",
                  nameResults?.length || 0,
                  "matches",
                );

                if (myRequestId !== currentRequestId.current) {
                  return { suggestions: [] };
                }

                if (!nameResults || nameResults.length === 0) {
                  console.log("[CodeRAG] No results from backend");
                  return { suggestions: [] };
                }

                suggestions = nameResults.map((entry: any) => {
                  const detail = `${entry.file_path}:${entry.line_start}`;
                  let documentation = `**${entry.signature}**`;
                  if (entry.docstring) {
                    documentation += `\n\n${entry.docstring}`;
                  }
                  documentation += `\n\n📍 ${entry.file_path}:${entry.line_start}-${entry.line_end}`;

                  return {
                    label: entry.func_name,
                    kind: monaco.languages.CompletionItemKind.Function,
                    detail,
                    documentation: {
                      value: documentation,
                      isTrusted: true,
                    },
                    insertText: entry.signature,
                    range: {
                      startLineNumber: position.lineNumber,
                      startColumn:
                        word?.startColumn ||
                        position.column - currentWord.length,
                      endLineNumber: position.lineNumber,
                      endColumn: position.column,
                    },
                    sortText: `1${entry.func_name}`,
                    filterText: entry.func_name,
                  };
                });
              } catch (err) {
                console.warn("[CodeRAG] queryCodeRagByName failed:", err);
              }

              if (suggestions.length > 0) {
                if (cacheRef.current.size >= MAX_CACHE_SIZE) {
                  const oldestKey = cacheRef.current.keys().next().value;
                  if (oldestKey) cacheRef.current.delete(oldestKey);
                }
                cacheRef.current.set(cacheKey, {
                  data: suggestions,
                  expiresAt: Date.now() + CACHE_TTL_MS,
                });
                console.log(
                  "[CodeRAG] Returning",
                  suggestions.length,
                  "suggestions",
                );
              }

              return { suggestions };
            } catch (err) {
              console.error("[CodeRAG] Unexpected error:", err);
              return { suggestions: [] };
            }
          },
        },
      );

      console.log("[CodeRAG] ✅ Completion provider registered successfully");
      return disposable;
    },
    [],
  );

  return { registerCompletion };
}
