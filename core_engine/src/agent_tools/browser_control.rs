use chromiumoxide::Browser;
use serde_json::Value;
use std::error::Error;
use futures_util::StreamExt;

/// Cấu trúc lưu trữ tọa độ của một Element
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ElementRect {
    pub tag_name: String,
    pub inner_text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Công cụ đọc trực tiếp tọa độ từ nhân JS của trình duyệt
pub struct BrowserControl {
    browser: Option<Browser>,
}

impl BrowserControl {
    pub fn new() -> Self {
        Self { browser: None }
    }

    /// Kết nối tới Google Chrome qua cổng CDP (Chrome DevTools Protocol)
    pub async fn connect_cdp(&mut self, ws_url: &str) -> Result<(), Box<dyn Error>> {
        // connect to an existing browser
        let (browser, mut handler) = Browser::connect(ws_url).await?;
        tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });
        self.browser = Some(browser);
        Ok(())
    }

    /// Gửi script JS vào page hiện tại để lấy tất cả các node tương tác được
    pub async fn get_interactive_elements(&self) -> Result<Vec<ElementRect>, Box<dyn Error>> {
        let browser = self.browser.as_ref().ok_or("Browser not connected")?;
        
        let mut pages = browser.pages().await?;
        let mut retries = 0;
        while pages.is_empty() && retries < 5 {
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            pages = browser.pages().await?;
            retries += 1;
        }

        if pages.is_empty() {
            return Err("No pages found after retries".into());
        }

        let mut active_page = None;
        for p in &pages {
            if let Ok(res) = p.evaluate("document.visibilityState").await {
                if let Ok(val) = res.into_value::<String>() {
                    if val == "visible" {
                        active_page = Some(p.clone());
                        break;
                    }
                }
            }
        }
        let page = active_page.unwrap_or_else(|| pages.first().cloned().unwrap());

        // Script JS này quét tất cả các element (a, button, input) và trả về bounding rect
        let script = r#"
            (() => {
                let overlay = document.getElementById('alouette-ai-overlay');
                if (!overlay) {
                    overlay = document.createElement('div');
                    overlay.id = 'alouette-ai-overlay';
                    overlay.style.position = 'fixed';
                    overlay.style.top = '0';
                    overlay.style.left = '0';
                    overlay.style.width = '100vw';
                    overlay.style.height = '100vh';
                    overlay.style.backgroundColor = 'transparent';
                    overlay.style.zIndex = '2147483647';
                    overlay.style.pointerEvents = 'auto'; // Block user interaction
                    overlay.style.boxSizing = 'border-box';
                    // Plasma/Smoke border effect
                    overlay.style.border = '12px solid rgba(255,255,255,0.1)';
                    overlay.style.boxShadow = 'inset 0 0 50px rgba(255, 0, 128, 0.8), inset 0 0 100px rgba(0, 255, 255, 0.8), inset 0 0 150px rgba(255, 255, 0, 0.6)';
                    overlay.style.animation = 'alouette-plasma 4s linear infinite';
                    
                    overlay.innerHTML = `
                        <button id="alouette-close-overlay" style="
                            position: absolute; 
                            bottom: 30px; 
                            right: 30px; 
                            padding: 10px 20px; 
                            background: rgba(239, 68, 68, 0.7); 
                            color: white; 
                            border: 1px solid rgba(255,255,255,0.4); 
                            border-radius: 30px; 
                            cursor: pointer; 
                            font-size: 14px; 
                            font-family: sans-serif;
                            font-weight: bold;
                            backdrop-filter: blur(5px);
                            pointer-events: auto;
                            box-shadow: 0 4px 12px rgba(0,0,0,0.3);
                            transition: all 0.2s ease;
                        " onmouseover="this.style.background='rgba(239, 68, 68, 1)'; this.style.transform='scale(1.05)';" onmouseout="this.style.background='rgba(239, 68, 68, 0.7)'; this.style.transform='scale(1)';">
                            ✨ Dừng AI (Click)
                        </button>
                    `;
                    
                    let style = document.createElement('style');
                    style.innerHTML = '@keyframes alouette-plasma { 0% { filter: hue-rotate(0deg); } 100% { filter: hue-rotate(360deg); } }';
                    document.head.appendChild(style);
                    document.body.appendChild(overlay);

                    document.getElementById('alouette-close-overlay').addEventListener('click', () => {
                        overlay.style.display = 'none';
                    });
                }
                overlay.style.display = 'flex';

                const isVisible = (el) => {
                    const style = window.getComputedStyle(el);
                    return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
                };

                const elements = document.querySelectorAll('a, button, input, textarea, select, [role="button"], [role="link"], [role="menuitem"], [role="tab"], [tabindex]:not([tabindex="-1"]), [contenteditable="true"]');
                const results = [];
                
                // Tạm ẩn overlay để elementFromPoint hoạt động chuẩn
                overlay.style.display = 'none';
                
                // --- Bắt đầu chống phát hiện bot (Stealth) ---
                try {
                    Object.defineProperty(navigator, 'webdriver', { get: () => false });
                } catch(e) {}
                if (!window.chrome) {
                    window.chrome = { runtime: {} };
                }
                try {
                    Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3] });
                    Object.defineProperty(navigator, 'languages', { get: () => ['vi-VN', 'vi', 'en-US', 'en'] });
                } catch(e) {}
                // --- Kết thúc chống phát hiện bot ---
                
                for (let el of elements) {
                    // Giới hạn số lượng phần tử trả về để tránh lỗi sập cổng websocket (vượt quá 64MB)
                    if (results.length >= 800) break;
                    
                    const rect = el.getBoundingClientRect();
                    // Giới hạn kích thước viewport để không lấy các phần tử quá xa ngoài màn hình
                    if (rect.y > window.innerHeight * 3 || rect.y < -window.innerHeight * 3) continue;

                    if (rect.width > 0 && rect.height > 0 && isVisible(el)) {
                        const cx = rect.x + rect.width / 2;
                        const cy = rect.y + rect.height / 2;
                        
                        let topEl = document.elementFromPoint(cx, cy);
                        let isCovered = topEl && topEl !== el && !el.contains(topEl);
                        
                        if (!isCovered) {
                            results.push({
                                tag_name: el.tagName,
                                inner_text: (el.innerText || el.value || el.getAttribute('aria-label') || el.getAttribute('placeholder') || '').trim().substring(0, 80),
                                x: cx,
                                y: cy,
                                width: rect.width,
                                height: rect.height
                            });
                        }
                    }
                }
                
                overlay.style.display = 'flex';
                return results;
            })()
        "#;

        let result = page.evaluate(script).await?;
        let value: Value = result.into_value()?;
        
        let elements: Vec<ElementRect> = serde_json::from_value(value)?;
        Ok(elements)
    }

    /// Ghi nội dung vào một phần tử trên trang web thông qua CSS selector
    pub async fn write_text(&self, css_selector: &str, text: &str) -> Result<(), Box<dyn Error>> {
        let browser = self.browser.as_ref().ok_or("Browser not connected")?;
        
        let mut pages = browser.pages().await?;
        let mut retries = 0;
        while pages.is_empty() && retries < 5 {
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            pages = browser.pages().await?;
            retries += 1;
        }

        if pages.is_empty() {
            return Err("No pages found after retries".into());
        }

        let mut active_page = None;
        for p in &pages {
            if let Ok(res) = p.evaluate("document.visibilityState").await {
                if let Ok(val) = res.into_value::<String>() {
                    if val == "visible" {
                        active_page = Some(p.clone());
                        break;
                    }
                }
            }
        }
        let page = active_page.unwrap_or_else(|| pages.first().cloned().unwrap());

        // Tìm element và focus/click vào nó
        let element = page.find_element(css_selector).await?;
        element.click().await?;
        
        // Xóa nội dung cũ (nếu có) bằng cách chọn tất cả và xóa (hoặc dùng JS)
        let clear_script = format!(
            "document.querySelector('{}').value = '';",
            css_selector.replace("'", "\\'")
        );
        page.evaluate(clear_script.as_str()).await?;

        // Gõ nội dung mới vào
        element.type_str(text).await?;

        Ok(())
    }
}
