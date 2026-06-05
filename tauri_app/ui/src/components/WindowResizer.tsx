import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * ResizeHandleDirection maps to Tauri v2 ResizeDirection values.
 * These tell the OS which edge/corner is being dragged so it shows the
 * correct cursor and handles the resize natively.
 */
type ResizeDirection =
  | "East"
  | "West"
  | "North"
  | "South"
  | "NorthEast"
  | "NorthWest"
  | "SouthEast"
  | "SouthWest";

/**
 * Invisible window-edge resize handles that restore resizability when
 * `decorations: false` is set in tauri.conf.json.
 *
 * Each handle sits on a window edge or corner and calls
 * `appWindow.startResizeDragging(direction)` on mousedown so the OS
 * takes over the resize operation.
 */
export default function WindowResizer() {
  const appWindow = getCurrentWindow();

  const startResize =
    (direction: ResizeDirection) => (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      appWindow.startResizeDragging(direction).catch((err) => {
        console.error("Resize drag error:", err);
      });
    };

  const handleProps = (direction: ResizeDirection) => ({
    onMouseDown: startResize(direction),
    className: `window-resize-handle resize-${direction.toLowerCase()}`,
  });

  return (
    <>
      {/* ── Edges ── */}
      <div {...handleProps("North")} />
      <div {...handleProps("South")} />
      <div {...handleProps("East")} />
      <div {...handleProps("West")} />

      {/* ── Corners ── */}
      <div {...handleProps("NorthEast")} />
      <div {...handleProps("NorthWest")} />
      <div {...handleProps("SouthEast")} />
      <div {...handleProps("SouthWest")} />
    </>
  );
}
