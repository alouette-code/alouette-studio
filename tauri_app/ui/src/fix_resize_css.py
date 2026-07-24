import re

with open("App.tsx", "r", encoding="utf-8") as f:
    content = f.read()

# 1. Replace the handlers block
handlers_pattern = re.compile(
    r"// 1\. Left Sidebar Width Resize Handler.*?\n  };\n",
    re.DOTALL
)

new_handlers = """// 1. Left Sidebar Width Resize Handler
  const handleLeftResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsDraggingLeft(true);
    let currentWidth = leftSidebarWidth;
    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = Math.max(160, Math.min(450, moveEvent.clientX));
      currentWidth = newWidth;
      document.documentElement.style.setProperty("--left-sidebar-width", `${newWidth}px`);
    };
    const handleMouseUp = () => {
      setIsDraggingLeft(false);
      setLeftSidebarWidth(currentWidth);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };

  // 2. Right Sidebar Width Resize Handler
  const handleRightResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsDraggingRight(true);
    let currentWidth = rightSidebarWidth;
    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = Math.max(
        280,
        Math.min(600, window.innerWidth - moveEvent.clientX),
      );
      currentWidth = newWidth;
      document.documentElement.style.setProperty("--right-sidebar-width", `${newWidth}px`);
    };
    const handleMouseUp = () => {
      setIsDraggingRight(false);
      setRightSidebarWidth(currentWidth);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };

  // 3. Tab List Horizontal Height Resize Handler
  const handleTabListResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    if (!tabListRef.current) return;
    setIsDraggingTabList(true);
    const tabListRect = tabListRef.current.getBoundingClientRect();
    let currentHeight = tabListHeight;
    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(
        80,
        Math.min(400, moveEvent.clientY - tabListRect.top),
      );
      currentHeight = newHeight;
      document.documentElement.style.setProperty("--tab-list-height", `${newHeight}px`);
    };
    const handleMouseUp = () => {
      setIsDraggingTabList(false);
      setTabListHeight(currentHeight);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };

  // 4. Resource Monitor Horizontal Height Resize Handler
  const handleMonitorResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    if (!monitorRef.current) return;
    setIsDraggingMonitor(true);
    const monitorRect = monitorRef.current.getBoundingClientRect();
    let currentHeight = monitorHeight;
    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(
        100,
        Math.min(500, moveEvent.clientY - monitorRect.top),
      );
      currentHeight = newHeight;
      document.documentElement.style.setProperty("--monitor-height", `${newHeight}px`);
    };
    const handleMouseUp = () => {
      setIsDraggingMonitor(false);
      setMonitorHeight(currentHeight);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };

  // 5. Config Setup Horizontal Height Resize Handler
  const handleConfigResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    if (!configRef.current) return;
    setIsDraggingConfig(true);
    const configRect = configRef.current.getBoundingClientRect();
    let currentHeight = configHeight;
    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(
        120,
        Math.min(450, moveEvent.clientY - configRect.top),
      );
      currentHeight = newHeight;
      document.documentElement.style.setProperty("--config-height", `${newHeight}px`);
    };
    const handleMouseUp = () => {
      setIsDraggingConfig(false);
      setConfigHeight(currentHeight);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };
"""

content = handlers_pattern.sub(new_handlers, content)

# 2. Replace the inline styles
content = content.replace(
    "gridTemplateColumns: `${isLeftSidebarOpen ? leftSidebarWidth : 0}px 1fr ${isRightSidebarOpen ? rightSidebarWidth : 0}px`,",
    "gridTemplateColumns: `${isLeftSidebarOpen ? `var(--left-sidebar-width, ${leftSidebarWidth}px)` : \"0px\"} 1fr ${isRightSidebarOpen ? `var(--right-sidebar-width, ${rightSidebarWidth}px)` : \"0px\"}`,"
)

content = content.replace(
    "flex: `0 0 ${tabListHeight}px`,",
    "flex: `0 0 var(--tab-list-height, ${tabListHeight}px)`,"
)

content = content.replace(
    "flex: isBottomPanelOpen ? `0 0 ${monitorHeight}px` : 1,",
    "flex: isBottomPanelOpen ? `0 0 var(--monitor-height, ${monitorHeight}px)` : 1,"
)

content = content.replace(
    "flex: `0 0 ${configHeight}px`,",
    "flex: `0 0 var(--config-height, ${configHeight}px)`,"
)

with open("App.tsx", "w", encoding="utf-8") as f:
    f.write(content)

print("Done replacing.")
