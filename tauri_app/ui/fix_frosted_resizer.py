import re

with open('src/App.tsx', 'r') as f:
    content = f.read()

# We need to replace the current Ghost Panel logic with the Frosted Glass Resizer logic.

left_old = r'''const handleLeftResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    setIsDraggingLeft\(true\);
    let currentWidth = leftSidebarWidth;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.top = "0";
    ghost\.style\.bottom = "0";
    ghost\.style\.left = "0";
    ghost\.style\.width = `\$\{currentWidth\}px`;
    ghost\.style\.backgroundColor = "rgba\(139, 92, 246, 0\.1\)";
    ghost\.style\.borderRight = "2px solid var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.pointerEvents = "none";
    ghost\.style\.backdropFilter = "blur\(2px\)";
    document\.body\.appendChild\(ghost\);
    
    const overlay = document\.createElement\("div"\);
    overlay\.style\.position = "fixed";
    overlay\.style\.inset = "0";
    overlay\.style\.zIndex = "9998";
    overlay\.style\.cursor = "ew-resize";
    document\.body\.appendChild\(overlay\);

    const handleMouseMove = \(moveEvent: MouseEvent\) => \{
      const newWidth = Math\.max\(160, Math\.min\(450, moveEvent\.clientX\)\);
      currentWidth = newWidth;
      ghost\.style\.width = `\$\{newWidth\}px`;
    \};
    
    const handleMouseUp = \(\) => \{
      setIsDraggingLeft\(false\);
      setLeftSidebarWidth\(currentWidth\);
      document\.documentElement\.style\.setProperty\("--left-sidebar-width", `\$\{currentWidth\}px`\);
      ghost\.remove\(\);
      overlay\.remove\(\);
      window\.removeEventListener\("mousemove", handleMouseMove\);
      window\.removeEventListener\("mouseup", handleMouseUp\);
    \};
    window\.addEventListener\("mousemove", handleMouseMove\);
    window\.addEventListener\("mouseup", handleMouseUp\);
  \};'''

left_new = '''const handleLeftResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsDraggingLeft(true);
    let currentWidth = leftSidebarWidth;
    
    const ghost = document.createElement("div");
    ghost.style.position = "fixed";
    ghost.style.top = "0";
    ghost.style.bottom = "0";
    ghost.style.width = "12px";
    ghost.style.left = `${e.clientX - 6}px`;
    ghost.style.backgroundColor = "rgba(150, 150, 150, 0.08)";
    ghost.style.borderLeft = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.borderRight = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.zIndex = "9999";
    ghost.style.pointerEvents = "none";
    ghost.style.backdropFilter = "blur(8px)";
    document.body.appendChild(ghost);
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ew-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = Math.max(160, Math.min(450, moveEvent.clientX));
      currentWidth = newWidth;
      ghost.style.left = `${newWidth - 6}px`;
    };
    
    const handleMouseUp = () => {
      setIsDraggingLeft(false);
      setLeftSidebarWidth(currentWidth);
      document.documentElement.style.setProperty("--left-sidebar-width", `${currentWidth}px`);
      ghost.remove();
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''


right_old = r'''const handleRightResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    setIsDraggingRight\(true\);
    let currentWidth = rightSidebarWidth;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.top = "0";
    ghost\.style\.bottom = "0";
    ghost\.style\.right = "0";
    ghost\.style\.width = `\$\{currentWidth\}px`;
    ghost\.style\.backgroundColor = "rgba\(139, 92, 246, 0\.1\)";
    ghost\.style\.borderLeft = "2px solid var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.pointerEvents = "none";
    ghost\.style\.backdropFilter = "blur\(2px\)";
    document\.body\.appendChild\(ghost\);
    
    const overlay = document\.createElement\("div"\);
    overlay\.style\.position = "fixed";
    overlay\.style\.inset = "0";
    overlay\.style\.zIndex = "9998";
    overlay\.style\.cursor = "ew-resize";
    document\.body\.appendChild\(overlay\);

    const handleMouseMove = \(moveEvent: MouseEvent\) => \{
      const newWidth = Math\.max\(
        280,
        Math\.min\(600, window\.innerWidth - moveEvent\.clientX\),
      \);
      currentWidth = newWidth;
      ghost\.style\.width = `\$\{newWidth\}px`;
    \};
    
    const handleMouseUp = \(\) => \{
      setIsDraggingRight\(false\);
      setRightSidebarWidth\(currentWidth\);
      document\.documentElement\.style\.setProperty\("--right-sidebar-width", `\$\{currentWidth\}px`\);
      ghost\.remove\(\);
      overlay\.remove\(\);
      window\.removeEventListener\("mousemove", handleMouseMove\);
      window\.removeEventListener\("mouseup", handleMouseUp\);
    \};
    window\.addEventListener\("mousemove", handleMouseMove\);
    window\.addEventListener\("mouseup", handleMouseUp\);
  \};'''

right_new = '''const handleRightResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsDraggingRight(true);
    let currentWidth = rightSidebarWidth;
    
    const ghost = document.createElement("div");
    ghost.style.position = "fixed";
    ghost.style.top = "0";
    ghost.style.bottom = "0";
    ghost.style.width = "12px";
    ghost.style.left = `${e.clientX - 6}px`;
    ghost.style.backgroundColor = "rgba(150, 150, 150, 0.08)";
    ghost.style.borderLeft = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.borderRight = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.zIndex = "9999";
    ghost.style.pointerEvents = "none";
    ghost.style.backdropFilter = "blur(8px)";
    document.body.appendChild(ghost);
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ew-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = Math.max(
        280,
        Math.min(600, window.innerWidth - moveEvent.clientX),
      );
      currentWidth = newWidth;
      ghost.style.left = `${window.innerWidth - newWidth - 6}px`;
    };
    
    const handleMouseUp = () => {
      setIsDraggingRight(false);
      setRightSidebarWidth(currentWidth);
      document.documentElement.style.setProperty("--right-sidebar-width", `${currentWidth}px`);
      ghost.remove();
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''

tablist_old = r'''const handleTabListResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    if \(!tabListRef\.current\) return;
    setIsDraggingTabList\(true\);
    const tabListRect = tabListRef\.current\.getBoundingClientRect\(\);
    let currentHeight = tabListHeight;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.left = `\$\{tabListRect\.left\}px`;
    ghost\.style\.top = `\$\{tabListRect\.top\}px`;
    ghost\.style\.width = `\$\{tabListRect\.width\}px`;
    ghost\.style\.height = `\$\{currentHeight\}px`;
    ghost\.style\.backgroundColor = "rgba\(139, 92, 246, 0\.1\)";
    ghost\.style\.borderBottom = "2px solid var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.pointerEvents = "none";
    ghost\.style\.backdropFilter = "blur\(2px\)";
    document\.body\.appendChild\(ghost\);
    
    const overlay = document\.createElement\("div"\);
    overlay\.style\.position = "fixed";
    overlay\.style\.inset = "0";
    overlay\.style\.zIndex = "9998";
    overlay\.style\.cursor = "ns-resize";
    document\.body\.appendChild\(overlay\);

    const handleMouseMove = \(moveEvent: MouseEvent\) => \{
      const newHeight = Math\.max\(
        80,
        Math\.min\(400, moveEvent\.clientY - tabListRect\.top\),
      \);
      currentHeight = newHeight;
      ghost\.style\.height = `\$\{newHeight\}px`;
    \};
    
    const handleMouseUp = \(\) => \{
      setIsDraggingTabList\(false\);
      setTabListHeight\(currentHeight\);
      document\.documentElement\.style\.setProperty\("--tab-list-height", `\$\{currentHeight\}px`\);
      ghost\.remove\(\);
      overlay\.remove\(\);
      window\.removeEventListener\("mousemove", handleMouseMove\);
      window\.removeEventListener\("mouseup", handleMouseUp\);
    \};
    window\.addEventListener\("mousemove", handleMouseMove\);
    window\.addEventListener\("mouseup", handleMouseUp\);
  \};'''

tablist_new = '''const handleTabListResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    if (!tabListRef.current) return;
    setIsDraggingTabList(true);
    const tabListRect = tabListRef.current.getBoundingClientRect();
    let currentHeight = tabListHeight;
    
    const ghost = document.createElement("div");
    ghost.style.position = "fixed";
    ghost.style.left = `${tabListRect.left}px`;
    ghost.style.width = `${tabListRect.width}px`;
    ghost.style.height = "12px";
    ghost.style.top = `${e.clientY - 6}px`;
    ghost.style.backgroundColor = "rgba(150, 150, 150, 0.08)";
    ghost.style.borderTop = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.borderBottom = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.zIndex = "9999";
    ghost.style.pointerEvents = "none";
    ghost.style.backdropFilter = "blur(8px)";
    document.body.appendChild(ghost);
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ns-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(
        80,
        Math.min(400, moveEvent.clientY - tabListRect.top),
      );
      currentHeight = newHeight;
      ghost.style.top = `${tabListRect.top + newHeight - 6}px`;
    };
    
    const handleMouseUp = () => {
      setIsDraggingTabList(false);
      setTabListHeight(currentHeight);
      document.documentElement.style.setProperty("--tab-list-height", `${currentHeight}px`);
      ghost.remove();
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''

monitor_old = r'''const handleMonitorResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    if \(!monitorRef\.current\) return;
    setIsDraggingMonitor\(true\);
    const monitorRect = monitorRef\.current\.getBoundingClientRect\(\);
    let currentHeight = monitorHeight;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.left = `\$\{monitorRect\.left\}px`;
    ghost\.style\.top = `\$\{monitorRect\.top\}px`;
    ghost\.style\.width = `\$\{monitorRect\.width\}px`;
    ghost\.style\.height = `\$\{currentHeight\}px`;
    ghost\.style\.backgroundColor = "rgba\(139, 92, 246, 0\.1\)";
    ghost\.style\.borderBottom = "2px solid var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.pointerEvents = "none";
    ghost\.style\.backdropFilter = "blur\(2px\)";
    document\.body\.appendChild\(ghost\);
    
    const overlay = document\.createElement\("div"\);
    overlay\.style\.position = "fixed";
    overlay\.style\.inset = "0";
    overlay\.style\.zIndex = "9998";
    overlay\.style\.cursor = "ns-resize";
    document\.body\.appendChild\(overlay\);

    const handleMouseMove = \(moveEvent: MouseEvent\) => \{
      const newHeight = Math\.max\(
        100,
        Math\.min\(500, moveEvent\.clientY - monitorRect\.top\),
      \);
      currentHeight = newHeight;
      ghost\.style\.height = `\$\{newHeight\}px`;
    \};
    
    const handleMouseUp = \(\) => \{
      setIsDraggingMonitor\(false\);
      setMonitorHeight\(currentHeight\);
      document\.documentElement\.style\.setProperty\("--monitor-height", `\$\{currentHeight\}px`\);
      ghost\.remove\(\);
      overlay\.remove\(\);
      window\.removeEventListener\("mousemove", handleMouseMove\);
      window\.removeEventListener\("mouseup", handleMouseUp\);
    \};
    window\.addEventListener\("mousemove", handleMouseMove\);
    window\.addEventListener\("mouseup", handleMouseUp\);
  \};'''

monitor_new = '''const handleMonitorResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    if (!monitorRef.current) return;
    setIsDraggingMonitor(true);
    const monitorRect = monitorRef.current.getBoundingClientRect();
    let currentHeight = monitorHeight;
    
    const ghost = document.createElement("div");
    ghost.style.position = "fixed";
    ghost.style.left = `${monitorRect.left}px`;
    ghost.style.width = `${monitorRect.width}px`;
    ghost.style.height = "12px";
    ghost.style.top = `${e.clientY - 6}px`;
    ghost.style.backgroundColor = "rgba(150, 150, 150, 0.08)";
    ghost.style.borderTop = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.borderBottom = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.zIndex = "9999";
    ghost.style.pointerEvents = "none";
    ghost.style.backdropFilter = "blur(8px)";
    document.body.appendChild(ghost);
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ns-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(
        100,
        Math.min(500, moveEvent.clientY - monitorRect.top),
      );
      currentHeight = newHeight;
      ghost.style.top = `${monitorRect.top + newHeight - 6}px`;
    };
    
    const handleMouseUp = () => {
      setIsDraggingMonitor(false);
      setMonitorHeight(currentHeight);
      document.documentElement.style.setProperty("--monitor-height", `${currentHeight}px`);
      ghost.remove();
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''

config_old = r'''const handleConfigResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    if \(!configRef\.current\) return;
    setIsDraggingConfig\(true\);
    const configRect = configRef\.current\.getBoundingClientRect\(\);
    let currentHeight = configHeight;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.left = `\$\{configRect\.left\}px`;
    ghost\.style\.top = `\$\{configRect\.top\}px`;
    ghost\.style\.width = `\$\{configRect\.width\}px`;
    ghost\.style\.height = `\$\{currentHeight\}px`;
    ghost\.style\.backgroundColor = "rgba\(139, 92, 246, 0\.1\)";
    ghost\.style\.borderBottom = "2px solid var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.pointerEvents = "none";
    ghost\.style\.backdropFilter = "blur\(2px\)";
    document\.body\.appendChild\(ghost\);
    
    const overlay = document\.createElement\("div"\);
    overlay\.style\.position = "fixed";
    overlay\.style\.inset = "0";
    overlay\.style\.zIndex = "9998";
    overlay\.style\.cursor = "ns-resize";
    document\.body\.appendChild\(overlay\);

    const handleMouseMove = \(moveEvent: MouseEvent\) => \{
      const newHeight = Math\.max\(
        120,
        Math\.min\(450, moveEvent\.clientY - configRect\.top\),
      \);
      currentHeight = newHeight;
      ghost\.style\.height = `\$\{newHeight\}px`;
    \};
    
    const handleMouseUp = \(\) => \{
      setIsDraggingConfig\(false\);
      setConfigHeight\(currentHeight\);
      document\.documentElement\.style\.setProperty\("--config-height", `\$\{currentHeight\}px`\);
      ghost\.remove\(\);
      overlay\.remove\(\);
      window\.removeEventListener\("mousemove", handleMouseMove\);
      window\.removeEventListener\("mouseup", handleMouseUp\);
    \};
    window\.addEventListener\("mousemove", handleMouseMove\);
    window\.addEventListener\("mouseup", handleMouseUp\);
  \};'''

config_new = '''const handleConfigResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    if (!configRef.current) return;
    setIsDraggingConfig(true);
    const configRect = configRef.current.getBoundingClientRect();
    let currentHeight = configHeight;
    
    const ghost = document.createElement("div");
    ghost.style.position = "fixed";
    ghost.style.left = `${configRect.left}px`;
    ghost.style.width = `${configRect.width}px`;
    ghost.style.height = "12px";
    ghost.style.top = `${e.clientY - 6}px`;
    ghost.style.backgroundColor = "rgba(150, 150, 150, 0.08)";
    ghost.style.borderTop = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.borderBottom = "1px solid rgba(200, 200, 200, 0.15)";
    ghost.style.zIndex = "9999";
    ghost.style.pointerEvents = "none";
    ghost.style.backdropFilter = "blur(8px)";
    document.body.appendChild(ghost);
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ns-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(
        120,
        Math.min(450, moveEvent.clientY - configRect.top),
      );
      currentHeight = newHeight;
      ghost.style.top = `${configRect.top + newHeight - 6}px`;
    };
    
    const handleMouseUp = () => {
      setIsDraggingConfig(false);
      setConfigHeight(currentHeight);
      document.documentElement.style.setProperty("--config-height", `${currentHeight}px`);
      ghost.remove();
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''

content = re.sub(left_old, left_new, content)
content = re.sub(right_old, right_new, content)
content = re.sub(tablist_old, tablist_new, content)
content = re.sub(monitor_old, monitor_new, content)
content = re.sub(config_old, config_new, content)

with open('src/App.tsx', 'w') as f:
    f.write(content)

print("Updated App.tsx successfully!")
