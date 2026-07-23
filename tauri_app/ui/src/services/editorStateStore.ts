/**
 * Persistent Editor State Store
 * Manages cursor positions, selections, scroll positions, and open tab sessions
 * across tab switches and app restarts.
 */

interface FileEditorState {
  lineNumber: number;
  column: number;
  viewState?: any; // Monaco ICodeEditorViewState
  scrollTop?: number;
  scrollLeft?: number;
  updatedAt: number;
}

interface SessionState {
  openFiles: string[];
  activeFilePath: string | null;
}

const STORAGE_KEY_FILE_STATES = "alouette_editor_file_states";
const STORAGE_KEY_SESSION_STATE = "alouette_editor_session_state";

class EditorStateStore {
  private fileStates: Map<string, FileEditorState> = new Map();
  private saveDebounceTimer: any = null;

  constructor() {
    this.loadFromStorage();
  }

  private normalizePath(p: string): string {
    return p.replace(/\\/g, "/").trim();
  }

  private loadFromStorage() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY_FILE_STATES);
      if (raw) {
        const parsed = JSON.parse(raw);
        for (const [k, v] of Object.entries(parsed)) {
          this.fileStates.set(this.normalizePath(k), v as FileEditorState);
        }
      }
    } catch (e) {
      console.warn("Failed to load editor file states from localStorage:", e);
    }
  }

  private saveToStorageDebounced() {
    if (this.saveDebounceTimer) clearTimeout(this.saveDebounceTimer);
    this.saveDebounceTimer = setTimeout(() => {
      try {
        const obj: Record<string, FileEditorState> = {};
        this.fileStates.forEach((val, key) => {
          obj[key] = val;
        });
        localStorage.setItem(STORAGE_KEY_FILE_STATES, JSON.stringify(obj));
      } catch (e) {
        console.warn("Failed to save editor file states to localStorage:", e);
      }
    }, 500);
  }

  /**
   * Save cursor & view state for a file
   */
  saveFileState(
    filePath: string | null | undefined,
    lineNumber: number,
    column: number,
    viewState?: any,
    scrollTop?: number,
    scrollLeft?: number
  ) {
    if (!filePath) return;
    const norm = this.normalizePath(filePath);
    const state: FileEditorState = {
      lineNumber,
      column,
      viewState: viewState || null,
      scrollTop,
      scrollLeft,
      updatedAt: Date.now(),
    };
    this.fileStates.set(norm, state);
    this.saveToStorageDebounced();
  }

  /**
   * Get saved cursor & view state for a file
   */
  getFileState(filePath: string | null | undefined): FileEditorState | null {
    if (!filePath) return null;
    const norm = this.normalizePath(filePath);
    return this.fileStates.get(norm) || null;
  }

  /**
   * Save current open tabs session
   */
  saveSessionState(openFiles: string[], activeFilePath: string | null) {
    try {
      const session: SessionState = {
        openFiles: openFiles.map((p) => this.normalizePath(p)),
        activeFilePath: activeFilePath ? this.normalizePath(activeFilePath) : null,
      };
      localStorage.setItem(STORAGE_KEY_SESSION_STATE, JSON.stringify(session));
    } catch (e) {
      console.warn("Failed to save editor session state:", e);
    }
  }

  /**
   * Get saved open tabs session
   */
  getSessionState(): SessionState | null {
    try {
      const raw = localStorage.getItem(STORAGE_KEY_SESSION_STATE);
      if (raw) {
        return JSON.parse(raw) as SessionState;
      }
    } catch (e) {
      console.warn("Failed to get editor session state:", e);
    }
    return null;
  }
}

export const editorStateStore = new EditorStateStore();
