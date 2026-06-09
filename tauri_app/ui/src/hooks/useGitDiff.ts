import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { GitDiffLine } from "../types";

interface UseGitDiffOptions {
  /** Absolute or workspace-relative path to the file */
  filePath: string | null;
  /** Git working directory (project cwd) — optional, backend auto-detects from file */
  cwd?: string;
  /** Flag to trigger a refresh (e.g. after file save) */
  revision?: number;
}

/**
 * Custom hook that fetches git diff decorations for a single file.
 *
 * Uses a local state so the parent component (App.tsx) does NOT re-render
 * when the diff updates. Only the CodeEditor that uses this hook re-renders.
 *
 * `cwd` is optional: the Rust backend will try to auto-detect the git
 * repository by walking up from the file's parent directory.
 */
export function useGitDiff({
  filePath,
  cwd,
  revision = 0,
}: UseGitDiffOptions): {
  diffLines: GitDiffLine[];
  isUntracked: boolean;
  isLoading: boolean;
  refresh: () => void;
} {
  const [diffLines, setDiffLines] = useState<GitDiffLine[]>([]);
  const [isUntracked, setIsUntracked] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const refreshKeyRef = useRef(0);

  const refresh = useCallback(() => {
    refreshKeyRef.current += 1;
  }, []);

  useEffect(() => {
    // Reset state immediately when filePath changes (don't wait for fetch)
    setDiffLines([]);
    setIsUntracked(false);

    if (!filePath) {
      return;
    }

    let cancelled = false;
    const key = refreshKeyRef.current;

    const fetchDiff = async () => {
      setIsLoading(true);
      try {
        const result = await invoke<{
          lines: GitDiffLine[];
          untracked: boolean;
        }>("git_get_file_diff", {
          // Pass cwd if available, otherwise let backend auto-detect
          cwd: cwd || null,
          file: filePath,
        });

        if (!cancelled && key === refreshKeyRef.current) {
          setDiffLines(result.lines);
          setIsUntracked(result.untracked);
        }
      } catch (err: any) {
        // Git diff may fail (not a git repo, no HEAD, etc.) — that's fine, no decorations
        if (!cancelled && key === refreshKeyRef.current) {
          setDiffLines([]);
          setIsUntracked(false);
        }
      } finally {
        if (!cancelled && key === refreshKeyRef.current) {
          setIsLoading(false);
        }
      }
    };

    fetchDiff();

    return () => {
      cancelled = true;
    };
  }, [filePath, cwd, revision]);

  return { diffLines, isUntracked, isLoading, refresh };
}
