use enigo::{Enigo, Mouse, Keyboard, Settings, Button, Coordinate, Direction, Key};

/// Trình điều khiển chuột và bàn phím OS (fallback khi CDP không xử lý được)
pub struct MouseControl {
    enigo: Enigo,
}

impl MouseControl {
    pub fn try_new() -> Result<Self, String> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| format!("Failed to initialize OS input controller (Enigo): {:?}", e))?;
        Ok(Self { enigo })
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        let _ = self.enigo.move_mouse(x, y, Coordinate::Abs);
    }

    pub fn click(&mut self) {
        let _ = self.enigo.button(Button::Left, Direction::Click);
    }

    pub fn click_at(&mut self, x: i32, y: i32) {
        self.move_to(x, y);
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.click();
    }

    pub fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            let _ = self.enigo.text(&ch.to_string());
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    pub fn press_enter(&mut self) {
        let _ = self.enigo.key(Key::Return, Direction::Click);
    }
}
