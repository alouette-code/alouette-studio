let initialized = false;

/**
 * Suppresses default browser chrome behaviors (native context menu, DevTools keys,
 * page reload/navigation, zoom hotkeys, and accidental drag-drop navigation)
 * so every Tauri WebView window behaves like a native desktop application.
 */
export function initNativeAppLockdown(): void {
  if (initialized || typeof window === "undefined") return;
  initialized = true;

  // 1. Suppress native WebKit/WebView2 context menu in bubble phase so custom React/Monaco menus still work.
  window.addEventListener("contextmenu", (e) => {
    e.preventDefault();
  });

  // 2. Capture-phase lockdown for DevTools, view-source, reload, caret-browsing, and browser history navigation.
  window.addEventListener(
    "keydown",
    (e) => {
      const key = e.key.toLowerCase();
      const mod = e.ctrlKey || e.metaKey;
      const target = e.target as HTMLElement | null;
      const insideGuestOrTerm = Boolean(
        target?.closest?.(".vm-vnc-container, .xterm")
      );

      // Always block DevTools & Inspector shortcuts
      if (
        e.key === "F12" ||
        (mod && e.shiftKey && (key === "i" || key === "j" || key === "c")) ||
        (e.metaKey && e.altKey && (key === "i" || key === "j" || key === "c")) ||
        (mod && key === "u") ||
        e.key === "F7"
      ) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }

      // Always prevent browser reload (F5 / Ctrl+R / Cmd+R / Ctrl+Shift+R)
      if (e.key === "F5" || (mod && key === "r")) {
        e.preventDefault();
        if (!insideGuestOrTerm) {
          e.stopPropagation();
        }
        return;
      }

      // Block browser back/forward navigation shortcuts unless inside VNC/terminal
      if (
        e.key === "BrowserBack" ||
        e.key === "BrowserForward" ||
        (!insideGuestOrTerm && e.altKey && (e.key === "ArrowLeft" || e.key === "ArrowRight"))
      ) {
        e.preventDefault();
      }
    },
    true
  );

  // 3. Bubble-phase fallback for browser dialogs (Print, Save Page, Browser Find, Downloads, Zoom)
  // when not already handled by Monaco / CodeMirror / App shortcuts.
  window.addEventListener("keydown", (e) => {
    if (e.defaultPrevented) return;
    const key = e.key.toLowerCase();
    const mod = e.ctrlKey || e.metaKey;

    if (
      e.key === "F3" ||
      (mod && (key === "p" || key === "s" || key === "f" || key === "g" || key === "h" || key === "j")) ||
      (mod && (key === "+" || key === "-" || key === "=" || key === "0"))
    ) {
      e.preventDefault();
    }
  });

  // 4. Disable Ctrl/Cmd + mouse-wheel browser zoom
  window.addEventListener(
    "wheel",
    (e) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
      }
    },
    { passive: false }
  );

  // 5. Block mouse side buttons (browser Back/Forward buttons 3 & 4)
  const blockMouseNav = (e: MouseEvent) => {
    if (e.button === 3 || e.button === 4) {
      e.preventDefault();
    }
  };
  window.addEventListener("mousedown", blockMouseNav, true);
  window.addEventListener("mouseup", blockMouseNav, true);

  // 6. Prevent accidental file drops outside explicit dropzones from navigating the WebView
  window.addEventListener("dragover", (e) => {
    e.preventDefault();
  });
  window.addEventListener("drop", (e) => {
    e.preventDefault();
  });
}
