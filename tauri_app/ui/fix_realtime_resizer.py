import re

with open('src/App.tsx', 'r') as f:
    content = f.read()

# I will write a regex to find all handlers since they all follow the same pattern now.
# Wait, let's just do exact string replacement for each one.
# It's safer if I just write a python script to replace the new handlers with rAF ones.

# 1. Left Resize
left_old = r'''const handleLeftResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    setIsDraggingLeft\(true\);
    let currentWidth = leftSidebarWidth;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.top = "0";
    ghost\.style\.bottom = "0";
    ghost\.style\.width = "4px";
    ghost\.style\.backgroundColor = "var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.left = `\$\{e\.clientX\}px`;
    ghost\.style\.cursor = "ew-resize";
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
      ghost\.style\.left = `\$\{newWidth\}px`;
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
    let animationFrameId: number | null = null;
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ew-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (animationFrameId !== null) return;
      animationFrameId = requestAnimationFrame(() => {
        const newWidth = Math.max(160, Math.min(450, moveEvent.clientX));
        currentWidth = newWidth;
        document.documentElement.style.setProperty("--left-sidebar-width", `${newWidth}px`);
        animationFrameId = null;
      });
    };
    
    const handleMouseUp = () => {
      setIsDraggingLeft(false);
      setLeftSidebarWidth(currentWidth);
      if (animationFrameId !== null) cancelAnimationFrame(animationFrameId);
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''


# 2. Right Resize
right_old = r'''const handleRightResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    setIsDraggingRight\(true\);
    let currentWidth = rightSidebarWidth;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.top = "0";
    ghost\.style\.bottom = "0";
    ghost\.style\.width = "4px";
    ghost\.style\.backgroundColor = "var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.left = `\$\{e\.clientX\}px`;
    ghost\.style\.cursor = "ew-resize";
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
      ghost\.style\.left = `\$\{window\.innerWidth - newWidth\}px`;
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
    let animationFrameId: number | null = null;
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ew-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (animationFrameId !== null) return;
      animationFrameId = requestAnimationFrame(() => {
        const newWidth = Math.max(
          280,
          Math.min(600, window.innerWidth - moveEvent.clientX),
        );
        currentWidth = newWidth;
        document.documentElement.style.setProperty("--right-sidebar-width", `${newWidth}px`);
        animationFrameId = null;
      });
    };
    
    const handleMouseUp = () => {
      setIsDraggingRight(false);
      setRightSidebarWidth(currentWidth);
      if (animationFrameId !== null) cancelAnimationFrame(animationFrameId);
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''

# 3. Tab List Resize
tablist_old = r'''const handleTabListResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    if \(!tabListRef\.current\) return;
    setIsDraggingTabList\(true\);
    const tabListRect = tabListRef\.current\.getBoundingClientRect\(\);
    let currentHeight = tabListHeight;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.left = `\$\{tabListRect\.left\}px`;
    ghost\.style\.width = `\$\{tabListRect\.width\}px`;
    ghost\.style\.height = "4px";
    ghost\.style\.backgroundColor = "var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.top = `\$\{e\.clientY\}px`;
    ghost\.style\.cursor = "ns-resize";
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
      ghost\.style\.top = `\$\{tabListRect\.top \+ newHeight\}px`;
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
    let animationFrameId: number | null = null;
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ns-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (animationFrameId !== null) return;
      animationFrameId = requestAnimationFrame(() => {
        const newHeight = Math.max(
          80,
          Math.min(400, moveEvent.clientY - tabListRect.top),
        );
        currentHeight = newHeight;
        document.documentElement.style.setProperty("--tab-list-height", `${newHeight}px`);
        animationFrameId = null;
      });
    };
    
    const handleMouseUp = () => {
      setIsDraggingTabList(false);
      setTabListHeight(currentHeight);
      if (animationFrameId !== null) cancelAnimationFrame(animationFrameId);
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''


# 4. Monitor Resize
monitor_old = r'''const handleMonitorResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    if \(!monitorRef\.current\) return;
    setIsDraggingMonitor\(true\);
    const monitorRect = monitorRef\.current\.getBoundingClientRect\(\);
    let currentHeight = monitorHeight;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.left = `\$\{monitorRect\.left\}px`;
    ghost\.style\.width = `\$\{monitorRect\.width\}px`;
    ghost\.style\.height = "4px";
    ghost\.style\.backgroundColor = "var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.top = `\$\{e\.clientY\}px`;
    ghost\.style\.cursor = "ns-resize";
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
      ghost\.style\.top = `\$\{monitorRect\.top \+ newHeight\}px`;
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
    let animationFrameId: number | null = null;
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ns-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (animationFrameId !== null) return;
      animationFrameId = requestAnimationFrame(() => {
        const newHeight = Math.max(
          100,
          Math.min(500, moveEvent.clientY - monitorRect.top),
        );
        currentHeight = newHeight;
        document.documentElement.style.setProperty("--monitor-height", `${newHeight}px`);
        animationFrameId = null;
      });
    };
    
    const handleMouseUp = () => {
      setIsDraggingMonitor(false);
      setMonitorHeight(currentHeight);
      if (animationFrameId !== null) cancelAnimationFrame(animationFrameId);
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''


# 5. Config Resize
config_old = r'''const handleConfigResizeStart = \(e: React\.MouseEvent\) => \{
    e\.preventDefault\(\);
    if \(!configRef\.current\) return;
    setIsDraggingConfig\(true\);
    const configRect = configRef\.current\.getBoundingClientRect\(\);
    let currentHeight = configHeight;
    
    const ghost = document\.createElement\("div"\);
    ghost\.style\.position = "fixed";
    ghost\.style\.left = `\$\{configRect\.left\}px`;
    ghost\.style\.width = `\$\{configRect\.width\}px`;
    ghost\.style\.height = "4px";
    ghost\.style\.backgroundColor = "var\(--color-accent\)";
    ghost\.style\.zIndex = "9999";
    ghost\.style\.top = `\$\{e\.clientY\}px`;
    ghost\.style\.cursor = "ns-resize";
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
      ghost\.style\.top = `\$\{configRect\.top \+ newHeight\}px`;
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
    let animationFrameId: number | null = null;
    
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "9998";
    overlay.style.cursor = "ns-resize";
    document.body.appendChild(overlay);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (animationFrameId !== null) return;
      animationFrameId = requestAnimationFrame(() => {
        const newHeight = Math.max(
          120,
          Math.min(450, moveEvent.clientY - configRect.top),
        );
        currentHeight = newHeight;
        document.documentElement.style.setProperty("--config-height", `${newHeight}px`);
        animationFrameId = null;
      });
    };
    
    const handleMouseUp = () => {
      setIsDraggingConfig(false);
      setConfigHeight(currentHeight);
      if (animationFrameId !== null) cancelAnimationFrame(animationFrameId);
      overlay.remove();
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };'''


# Replace
content = re.sub(left_old, left_new, content)
content = re.sub(right_old, right_new, content)
content = re.sub(tablist_old, tablist_new, content)
content = re.sub(monitor_old, monitor_new, content)
content = re.sub(config_old, config_new, content)

with open('src/App.tsx', 'w') as f:
    f.write(content)

print("Updated App.tsx successfully!")
