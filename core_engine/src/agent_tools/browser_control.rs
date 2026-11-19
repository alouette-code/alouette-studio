use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, InsertTextParams,
};
use chromiumoxide::cdp::browser_protocol::page::{
    AddScriptToEvaluateOnNewDocumentParams, CaptureScreenshotFormat,
};
use chromiumoxide::layout::Point;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::{Browser, Page};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// Indexed interactive element returned to the AI (Claude/Codex ref-based pattern).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementRef {
    pub ref_id: usize,
    pub tag: String,
    pub role: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSnapshot {
    pub url: String,
    pub title: String,
    pub scroll_y: f64,
    pub scroll_height: f64,
    pub viewport_height: f64,
    pub elements: Vec<ElementRef>,
}

const STEALTH_INIT_SCRIPT: &str = r#"
(() => {
    try {
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
    } catch (_) {}
    if (!window.chrome) {
        window.chrome = { runtime: {} };
    }
    try {
        Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en', 'vi-VN', 'vi'] });
    } catch (_) {}
})();
"#;

/// Persistent CDP Browser Controller powered by a single `chromiumoxide` engine.
pub struct BrowserControl {
    browser: Option<Browser>,
    handler_task: Option<tokio::task::JoinHandle<()>>,
    connected_ws: Option<String>,
    stealth_injected: bool,
}

impl Default for BrowserControl {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BrowserControl {
    fn drop(&mut self) {
        if let Some(task) = self.handler_task.take() {
            task.abort();
        }
    }
}

impl BrowserControl {
    pub fn new() -> Self {
        Self {
            browser: None,
            handler_task: None,
            connected_ws: None,
            stealth_injected: false,
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(task) = self.handler_task.take() {
            task.abort();
        }
        self.browser = None;
        self.connected_ws = None;
        self.stealth_injected = false;
    }

    /// Fetch the WebSocket debugger URL from Chrome's `/json/version` endpoint.
    pub async fn fetch_ws_url(cdp_port: u16) -> Result<String, String> {
        let url = format!("http://127.0.0.1:{}/json/version", cdp_port);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Could not connect to Chrome CDP on port {}: {}", cdp_port, e))?;

        let json_val: Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid JSON from /json/version: {}", e))?;

        json_val
            .get("webSocketDebuggerUrl")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("No webSocketDebuggerUrl found in /json/version: {}", json_val))
    }

    /// Connect to CDP if not already connected (or reconnect if ws_url changed / dropped).
    pub async fn ensure_connected(&mut self, cdp_port: u16) -> Result<(), String> {
        let ws_url = Self::fetch_ws_url(cdp_port).await?;

        if let (Some(ref browser), Some(ref prev_ws)) = (&self.browser, &self.connected_ws) {
            if prev_ws == &ws_url && browser.pages().await.is_ok() {
                return Ok(());
            }
        }

        self.disconnect();

        let (browser, mut handler) = Browser::connect(&ws_url)
            .await
            .map_err(|e| format!("Failed to connect to CDP ({}): {}", ws_url, e))?;

        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        self.browser = Some(browser);
        self.handler_task = Some(handle);
        self.connected_ws = Some(ws_url);
        self.stealth_injected = false;

        if let Ok(page) = self.active_page().await {
            if page
                .execute(AddScriptToEvaluateOnNewDocumentParams::new(
                    STEALTH_INIT_SCRIPT.to_string(),
                ))
                .await
                .is_ok()
            {
                self.stealth_injected = true;
            }
        }
        Ok(())
    }

    /// Get the active visible page (skipping devtools:// pages).
    pub async fn active_page(&self) -> Result<Page, String> {
        let browser = self
            .browser
            .as_ref()
            .ok_or_else(|| "Browser is not connected".to_string())?;

        let mut pages = Vec::new();
        for _ in 0..10 {
            if let Ok(list) = browser.pages().await {
                let filtered: Vec<Page> = list
                    .into_iter()
                    .collect();
                if !filtered.is_empty() {
                    pages = filtered;
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        if pages.is_empty() {
            return Err("No open browser tabs found".to_string());
        }

        let mut non_devtools = Vec::new();
        for p in &pages {
            if let Ok(Some(url)) = p.url().await {
                if url.starts_with("devtools://") || url.starts_with("chrome-extension://") {
                    continue;
                }
            }
            non_devtools.push(p.clone());
        }

        let candidates = if non_devtools.is_empty() {
            pages
        } else {
            non_devtools
        };

        for p in &candidates {
            if let Ok(res) = p.evaluate("document.visibilityState").await {
                if let Ok(val) = res.into_value::<String>() {
                    if val == "visible" {
                        return Ok(p.clone());
                    }
                }
            }
        }

        Ok(candidates.into_iter().next().unwrap())
    }

    /// Navigate to a URL, or `"back"` / `"forward"` / `"reload"`.
    pub async fn navigate(&self, target: &str) -> Result<String, String> {
        let page = self.active_page().await?;
        match target.trim().to_lowercase().as_str() {
            "back" => {
                page.evaluate("window.history.back()")
                    .await
                    .map_err(|e| format!("History back failed: {}", e))?;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            "forward" => {
                page.evaluate("window.history.forward()")
                    .await
                    .map_err(|e| format!("History forward failed: {}", e))?;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            "reload" | "refresh" => {
                page.reload()
                    .await
                    .map_err(|e| format!("Reload failed: {}", e))?;
            }
            _ => {
                page.goto(target)
                    .await
                    .map_err(|e| format!("Navigation to '{}' failed: {}", target, e))?;
            }
        }

        let current_url = page.url().await.ok().flatten().unwrap_or_else(|| target.to_string());
        let title = page
            .evaluate("document.title || ''")
            .await
            .ok()
            .and_then(|r| r.into_value::<String>().ok())
            .unwrap_or_default();

        Ok(format!("Navigated to: {} (title: \"{}\")", current_url, title))
    }

    /// Extract clean readable page text along with URL & title.
    pub async fn get_page_content(&self, max_chars: usize) -> Result<String, String> {
        let page = self.active_page().await?;
        let script = r#"
            (() => {
                const text = (document.body && document.body.innerText) ? document.body.innerText : '';
                return {
                    url: window.location.href || '',
                    title: document.title || '',
                    text: text.replace(/\n{3,}/g, '\n\n').trim()
                };
            })()
        "#;
        let val: Value = page
            .evaluate(script)
            .await
            .map_err(|e| format!("Failed to read page content: {}", e))?
            .into_value()
            .map_err(|e| format!("Failed to decode page content: {}", e))?;

        let url = val.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let title = val.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let text = val.get("text").and_then(|v| v.as_str()).unwrap_or("");

        let truncated: String = text.chars().take(max_chars).collect();
        let suffix = if text.chars().count() > max_chars {
            "\n\n[...content truncated...]"
        } else {
            ""
        };

        Ok(format!("URL: {}\nTitle: {}\n\n{}{}", url, title, truncated, suffix))
    }

    /// Snapshot visible interactive elements and stamp `data-alouette-ref="<ref_id>"` for reliable targeting.
    pub async fn snapshot_elements(&self) -> Result<BrowserSnapshot, String> {
        let page = self.active_page().await?;
        let script = r#"
            (() => {
                document.querySelectorAll('[data-alouette-ref]').forEach(el => {
                    el.removeAttribute('data-alouette-ref');
                });

                const isVisible = (el) => {
                    const style = window.getComputedStyle(el);
                    return style.display !== 'none'
                        && style.visibility !== 'hidden'
                        && parseFloat(style.opacity || '1') > 0;
                };

                const selector = 'a, button, input:not([type="hidden"]), textarea, select, [role="button"], [role="link"], [role="menuitem"], [role="tab"], [role="checkbox"], [role="radio"], [tabindex]:not([tabindex="-1"]), [contenteditable="true"]';
                const nodes = document.querySelectorAll(selector);
                const elements = [];
                let refCounter = 1;

                for (const el of nodes) {
                    if (elements.length >= 400) break;
                    const rect = el.getBoundingClientRect();
                    if (rect.width <= 0 || rect.height <= 0) continue;
                    if (rect.bottom < -window.innerHeight || rect.top > window.innerHeight * 2.5) continue;
                    if (!isVisible(el)) continue;

                    const cx = rect.left + rect.width / 2;
                    const cy = rect.top + rect.height / 2;
                    if (cx >= 0 && cy >= 0 && cx <= window.innerWidth && cy <= window.innerHeight) {
                        const topEl = document.elementFromPoint(cx, cy);
                        if (topEl && topEl !== el && !el.contains(topEl)) continue;
                    }

                    const refId = refCounter++;
                    el.setAttribute('data-alouette-ref', String(refId));

                    const tag = el.tagName.toLowerCase();
                    const typeAttr = el.getAttribute('type');
                    const roleAttr = el.getAttribute('role');
                    const role = roleAttr || (typeAttr ? `${tag}[${typeAttr}]` : tag);

                    const rawLabel = el.innerText
                        || el.getAttribute('aria-label')
                        || el.getAttribute('placeholder')
                        || el.value
                        || el.getAttribute('title')
                        || el.getAttribute('alt')
                        || '';
                    const label = String(rawLabel).replace(/\s+/g, ' ').trim().substring(0, 100);
                    const href = tag === 'a' ? (el.getAttribute('href') || '').substring(0, 120) : null;

                    elements.push({
                        ref_id: refId,
                        tag,
                        role,
                        label,
                        href: href || null,
                        x: Math.round(cx),
                        y: Math.round(cy),
                        width: Math.round(rect.width),
                        height: Math.round(rect.height)
                    });
                }

                return {
                    url: window.location.href || '',
                    title: document.title || '',
                    scroll_y: Math.round(window.scrollY || 0),
                    scroll_height: Math.round(document.documentElement.scrollHeight || 0),
                    viewport_height: Math.round(window.innerHeight || 0),
                    elements
                };
            })()
        "#;

        let val: Value = page
            .evaluate(script)
            .await
            .map_err(|e| format!("Failed to evaluate snapshot script: {}", e))?
            .into_value()
            .map_err(|e| format!("Failed to decode snapshot: {}", e))?;

        serde_json::from_value(val).map_err(|e| format!("Failed to parse BrowserSnapshot: {}", e))
    }

    /// Click an element by `ref_id` (preferred, auto-scrolls into view) or fallback `(x, y)` coordinates.
    pub async fn click(
        &self,
        ref_id: Option<usize>,
        x: Option<f64>,
        y: Option<f64>,
    ) -> Result<String, String> {
        let page = self.active_page().await?;

        if let Some(id) = ref_id {
            let selector = format!("[data-alouette-ref=\"{}\"]", id);
            let locate_js = format!(
                r#"(() => {{
                    const el = document.querySelector('{}');
                    if (!el) return null;
                    el.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'instant' }});
                    const r = el.getBoundingClientRect();
                    return {{ x: r.left + r.width / 2, y: r.top + r.height / 2, tag: el.tagName.toLowerCase() }};
                }})()"#,
                selector
            );
            let val: Value = page
                .evaluate(locate_js.as_str())
                .await
                .map_err(|e| format!("Failed to locate ref {}: {}", id, e))?
                .into_value()
                .map_err(|e| format!("Failed to decode element rect: {}", e))?;

            if val.is_null() {
                return Err(format!(
                    "Element ref [{}] not found in current DOM. Call get_browser_elements again.",
                    id
                ));
            }

            let cx = val.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let cy = val.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);

            page.click(Point::new(cx, cy))
                .await
                .map_err(|e| format!("CDP click on ref [{}] failed: {}", id, e))?;

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            return Ok(format!("Clicked element [{}] at ({:.0}, {:.0})", id, cx, cy));
        }

        if let (Some(cx), Some(cy)) = (x, y) {
            page.click(Point::new(cx, cy))
                .await
                .map_err(|e| format!("CDP click at ({}, {}) failed: {}", cx, cy, e))?;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            return Ok(format!("Clicked at ({:.0}, {:.0})", cx, cy));
        }

        Err("Either 'ref_id' or both 'x' and 'y' must be provided".to_string())
    }

    /// Type text into an element by `ref_id` (or currently focused element), optionally clearing first and pressing Enter.
    pub async fn type_text(
        &self,
        text: &str,
        ref_id: Option<usize>,
        clear: bool,
        press_enter: bool,
    ) -> Result<String, String> {
        let page = self.active_page().await?;

        if let Some(id) = ref_id {
            let selector = format!("[data-alouette-ref=\"{}\"]", id);
            let prep_js = format!(
                r#"(() => {{
                    const el = document.querySelector('{}');
                    if (!el) return false;
                    el.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'instant' }});
                    el.focus();
                    if ({}) {{
                        if (el.tagName === 'INPUT') {{
                            const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')?.set;
                            if (setter) setter.call(el, ''); else el.value = '';
                            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        }} else if (el.tagName === 'TEXTAREA') {{
                            const setter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value')?.set;
                            if (setter) setter.call(el, ''); else el.value = '';
                            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        }} else if (el.isContentEditable) {{
                            el.innerText = '';
                        }}
                    }}
                    return true;
                }})()"#,
                selector, clear
            );

            let found: bool = page
                .evaluate(prep_js.as_str())
                .await
                .map_err(|e| format!("Failed to focus ref [{}]: {}", id, e))?
                .into_value()
                .unwrap_or(false);

            if !found {
                return Err(format!(
                    "Element ref [{}] not found in current DOM. Call get_browser_elements again.",
                    id
                ));
            }
        }

        if !text.is_empty() {
            page.execute(InsertTextParams::new(text.to_string()))
                .await
                .map_err(|e| format!("CDP InsertText failed: {}", e))?;
        }

        if press_enter {
            self.press_key("Enter").await?;
        }

        Ok(format!(
            "Typed '{}' {}{}",
            text,
            ref_id.map(|id| format!("into [{}]", id)).unwrap_or_else(|| "into focused element".to_string()),
            if press_enter { " and pressed Enter" } else { "" }
        ))
    }

    /// Scroll the page (`down`, `up`, `top`, `bottom`).
    pub async fn scroll(&self, direction: &str, amount: Option<i64>) -> Result<String, String> {
        let page = self.active_page().await?;
        let px = amount.unwrap_or(600);
        let js = match direction.trim().to_lowercase().as_str() {
            "up" => format!("window.scrollBy(0, -{});", px),
            "top" => "window.scrollTo(0, 0);".to_string(),
            "bottom" => "window.scrollTo(0, document.documentElement.scrollHeight);".to_string(),
            _ => format!("window.scrollBy(0, {});", px),
        };
        let full_js = format!(
            "(() => {{ {} return {{ y: Math.round(window.scrollY), max: Math.round(document.documentElement.scrollHeight) }}; }})()",
            js
        );
        let val: Value = page
            .evaluate(full_js.as_str())
            .await
            .map_err(|e| format!("Scroll failed: {}", e))?
            .into_value()
            .map_err(|e| format!("Scroll decode failed: {}", e))?;

        let y = val.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
        let max = val.get("max").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(format!("Scrolled {} (scrollY: {}/{})", direction, y, max))
    }

    /// Send a key press (`Enter`, `Tab`, `Escape`, `Backspace`, `ArrowDown`, `ArrowUp`, `Space`).
    pub async fn press_key(&self, key: &str) -> Result<String, String> {
        let page = self.active_page().await?;
        let (key_name, code, vk) = match key.trim().to_lowercase().as_str() {
            "enter" | "return" => ("Enter", "Enter", 13),
            "tab" => ("Tab", "Tab", 9),
            "escape" | "esc" => ("Escape", "Escape", 27),
            "backspace" => ("Backspace", "Backspace", 8),
            "arrowdown" | "down" => ("ArrowDown", "ArrowDown", 40),
            "arrowup" | "up" => ("ArrowUp", "ArrowUp", 38),
            "space" | " " => (" ", "Space", 32),
            other => {
                return Err(format!("Unsupported key '{}'. Use Enter, Tab, Escape, Backspace, ArrowDown, ArrowUp, or Space.", other));
            }
        };

        let mut down = DispatchKeyEventParams::new(DispatchKeyEventType::RawKeyDown);
        down.key = Some(key_name.to_string());
        down.code = Some(code.to_string());
        down.windows_virtual_key_code = Some(vk);
        page.execute(down)
            .await
            .map_err(|e| format!("KeyDown '{}' failed: {}", key_name, e))?;

        if key_name == "Enter" || key_name == " " {
            let mut ch = DispatchKeyEventParams::new(DispatchKeyEventType::Char);
            ch.text = Some(if key_name == "Enter" { "\r".to_string() } else { " ".to_string() });
            let _ = page.execute(ch).await;
        }

        let mut up = DispatchKeyEventParams::new(DispatchKeyEventType::KeyUp);
        up.key = Some(key_name.to_string());
        up.code = Some(code.to_string());
        up.windows_virtual_key_code = Some(vk);
        page.execute(up)
            .await
            .map_err(|e| format!("KeyUp '{}' failed: {}", key_name, e))?;

        Ok(format!("Pressed key '{}'", key_name))
    }

    /// Wait for `ms` milliseconds or until a CSS `selector` appears in the DOM.
    pub async fn wait(&self, ms: Option<u64>, selector: Option<&str>) -> Result<String, String> {
        if let Some(sel) = selector {
            let page = self.active_page().await?;
            let escaped = sel.replace('\'', "\\'");
            let check_js = format!("Boolean(document.querySelector('{}'))", escaped);
            let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);
            loop {
                if let Ok(res) = page.evaluate(check_js.as_str()).await {
                    if res.into_value::<bool>().unwrap_or(false) {
                        return Ok(format!("Selector '{}' appeared", sel));
                    }
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("Timed out waiting 10s for selector '{}'", sel));
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
            }
        }

        let wait_ms = ms.unwrap_or(1000).min(15000);
        tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
        Ok(format!("Waited {}ms", wait_ms))
    }

    /// Capture a PNG screenshot of the active tab and save it to `save_path`.
    pub async fn screenshot(&self, save_path: &Path) -> Result<String, String> {
        let page = self.active_page().await?;
        let bytes = page
            .screenshot(
                ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .full_page(false)
                    .build(),
            )
            .await
            .map_err(|e| format!("Failed to capture screenshot: {}", e))?;

        if let Some(parent) = save_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(save_path, &bytes)
            .map_err(|e| format!("Failed to save screenshot to {}: {}", save_path.display(), e))?;

        Ok(format!(
            "Saved screenshot ({} bytes) to {}",
            bytes.len(),
            save_path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_snapshot_serialization() {
        let snap = BrowserSnapshot {
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            scroll_y: 120.0,
            scroll_height: 2400.0,
            viewport_height: 800.0,
            elements: vec![ElementRef {
                ref_id: 1,
                tag: "button".to_string(),
                role: "button".to_string(),
                label: "Sign In".to_string(),
                href: None,
                x: 200.0,
                y: 150.0,
                width: 90.0,
                height: 36.0,
            }],
        };

        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"ref_id\":1"));
        assert!(!json.contains("\"href\""));
        let decoded: BrowserSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.elements.len(), 1);
        assert_eq!(decoded.elements[0].label, "Sign In");
    }

    #[tokio::test]
    async fn test_browser_control_errors_without_connection() {
        let bc = BrowserControl::new();
        assert!(bc.active_page().await.is_err());
        assert!(bc.navigate("https://example.com").await.is_err());
        assert!(bc.click(Some(1), None, None).await.is_err());
        assert!(bc.press_key("InvalidKeyXYZ").await.is_err());
        // Fetch ws_url on closed port returns Err cleanly (no panic)
        assert!(BrowserControl::fetch_ws_url(59998).await.is_err());
    }

    #[tokio::test]
    async fn test_browser_wait_ms() {
        let bc = BrowserControl::new();
        let msg = bc.wait(Some(50), None).await.unwrap();
        assert_eq!(msg, "Waited 50ms");
    }
}
