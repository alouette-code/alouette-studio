/**
 * Global Error Store for tracking code/syntax error counts across files.
 * Universal, path-agnostic, cross-platform architecture with segment-bounded matching.
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
    const norm = this.canonicalizePath(path);
    if (!norm) return;

    if (count > 0) {
      if (this.errors.get(norm) !== count) {
        this.errors.set(norm, count);
        this.notify();
      }
    } else {
      let changed = false;
      for (const errPath of Array.from(this.errors.keys())) {
        if (this.isPathMatch(errPath, norm)) {
          this.errors.delete(errPath);
          changed = true;
        }
      }
      if (changed) {
        this.notify();
      }
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
    const normNode = this.canonicalizePath(nodePath);
    if (!normNode) return 0;

    if (isDir) {
      let total = 0;
      for (const [errPath, count] of this.errors.entries()) {
        const normErr = this.canonicalizePath(errPath);
        if (
          normErr.startsWith(normNode + "/") ||
          normErr.includes("/" + normNode + "/") ||
          this.isPathMatch(normErr, normNode)
        ) {
          total += count;
        }
      }
      return total;
    }

    // Path-agnostic segment-bounded match across relative and absolute path representations
    for (const [errPath, count] of this.errors.entries()) {
      const normErr = this.canonicalizePath(errPath);
      if (this.isPathMatch(normErr, normNode)) {
        if (count > 0) return count;
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

  /**
   * Universal path match: exact match, boundary-delimited segment suffix match, or filename fallback
   */
  private isPathMatch(pathA: string, pathB: string): boolean {
    if (pathA === pathB) return true;
    if (pathA.endsWith("/" + pathB)) return true;
    if (pathB.endsWith("/" + pathA)) return true;

    // Basename fallback if one of the paths is filename-only
    const baseA = pathA.split("/").pop();
    const baseB = pathB.split("/").pop();
    if (baseA && baseB && baseA === baseB) {
      if (!pathA.includes("/") || !pathB.includes("/")) {
        return true;
      }
    }
    return false;
  }

  /**
   * Canonicalize file path: decode URI (%20), strip file:// scheme, normalize slashes, collapse duplicates, lower-case
   */
  private canonicalizePath(p: string): string {
    if (!p) return "";
    let decoded = p;
    try {
      decoded = decodeURIComponent(p);
    } catch (e) {}
    let norm = decoded.replace(/\\/g, "/").replace(/\/+/g, "/").toLowerCase().trim();
    if (norm.startsWith("file:///")) norm = norm.substring(8);
    else if (norm.startsWith("file://")) norm = norm.substring(7);
    if (norm.startsWith("/")) norm = norm.substring(1);
    if (norm.endsWith("/")) norm = norm.substring(0, norm.length - 1);
    return norm;
  }
}

export const globalErrorStore = new ErrorStore();
