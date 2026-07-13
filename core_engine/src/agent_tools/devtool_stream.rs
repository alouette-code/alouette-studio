use chromiumoxide::Browser;
use std::error::Error;

pub struct DevToolStream {
    browser: Option<Browser>,
}

impl DevToolStream {
    pub fn new() -> Self {
        Self { browser: None }
    }

    /// Trả về toàn bộ nội dung HTML DOM hiện tại
    pub async fn get_dom(&self) -> Result<String, Box<dyn Error>> {
        if let Some(browser) = &self.browser {
            let pages = browser.pages().await?;
            if let Some(page) = pages.first() {
                let html = page.evaluate("document.documentElement.outerHTML").await?.into_value::<String>()?;
                return Ok(html);
            }
        }
        Err("Browser not connected or no pages found".into())
    }

    /// Thực thi mã JS trong console
    pub async fn execute_console_script(&self, script: &str) -> Result<String, Box<dyn Error>> {
        if let Some(browser) = &self.browser {
            let pages = browser.pages().await?;
            if let Some(page) = pages.first() {
                let result = page.evaluate(script).await?.into_value::<String>()?;
                return Ok(result);
            }
        }
        Err("Browser not connected or no pages found".into())
    }
}
