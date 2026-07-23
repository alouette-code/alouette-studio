/**
 * Global Error Store for tracking code/syntax error counts across all open files.
 * Provides path-agnostic matching and real-time subscription for UI components.
 */

type Listener = () => void;

class ErrorStore {
  private errors: Map<string, number> = new Map();
  private listeners: Set<Listener> = new Set();

  /**
   * Set error count for a specific file path
   */
  setFileError(path: string | null | undefined, count: number) {
    if (!path) return;
    const norm = this.normalizePath(path);
    const prev = this.errors.get(norm) || 0;
    if (prev !== count) {
      if (count > 0) {
        this.errors.set(norm, count);
      } else {
        this.errors.delete(norm);
      }
      this.notify();
    }
  }

  /**
   * Clear errors for a file path
   */
  clearFileError(path: string | null | undefined) {
    if (!path) return;
    const norm = this.normalizePath(path);
    if (this.errors.has(norm)) {
      this.errors.delete(norm);
      this.notify();
    }
  }

  /**
   * Check if a file or directory path has syntax errors
   */
  hasError(nodePath: string | null | undefined, isDir: boolean = false): boolean {
    return this.getErrorCount(nodePath, isDir) > 0;
  }

  /**
   * Get error count for a file or directory
   */
  getErrorCount(nodePath: string | null | undefined, isDir: boolean = false): number {
    if (!nodePath) return 0;
    const normNode = this.normalizePath(nodePath);
    if (!normNode) return 0;

    if (isDir) {
      const prefix = normNode.endsWith("/") ? normNode : normNode + "/";
      let total = 0;
      for (const [errPath, count] of this.errors.entries()) {
        if (errPath.startsWith(prefix) || errPath.includes("/" + normNode + "/")) {
          total += count;
        }
      }
      return total;
    }

    // Direct match
    if (this.errors.has(normNode)) {
      return this.errors.get(normNode) || 0;
    }

    // Flexible end-with match for relative/absolute path variations
    const nodeFileName = normNode.split("/").pop() || "";
    for (const [errPath, count] of this.errors.entries()) {
      if (
        errPath === normNode ||
        errPath.endsWith("/" + normNode) ||
        normNode.endsWith("/" + errPath) ||
        (nodeFileName && errPath.endsWith("/" + nodeFileName) && normNode.endsWith("/" + nodeFileName))
      ) {
        return count;
      }
    }

    return 0;
  }

  /**
   * Subscribe to error store state updates
   */
  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private notify() {
    this.listeners.forEach((fn) => {
      try {
        fn();
      } catch (err) {
        console.error("ErrorStore notification listener error:", err);
      }
    });
  }

  private normalizePath(p: string): string {
    return p.replace(/\\/g, "/").toLowerCase().trim();
  }
}

export const globalErrorStore = new ErrorStore();
