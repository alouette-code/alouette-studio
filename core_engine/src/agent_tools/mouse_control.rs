use enigo::{Enigo, Mouse, Keyboard, Settings, Button, Coordinate, Direction, Key};

/// Trình điều khiển chuột và bàn phím cho AI
pub struct MouseControl {
    enigo: Enigo,
}

impl MouseControl {
    pub fn new() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default()).unwrap(),
        }
    }

    /// Di chuyển chuột đến tọa độ màn hình
    pub fn move_to(&mut self, x: i32, y: i32) {
        let _ = self.enigo.move_mouse(x, y, Coordinate::Abs);
    }

    /// Click chuột trái tại vị trí hiện tại
    pub fn click(&mut self) {
        let _ = self.enigo.button(Button::Left, Direction::Click);
    }

    /// Di chuyển chuột đến tọa độ và click chuột trái
    pub fn click_at(&mut self, x: i32, y: i32) {
        self.move_to(x, y);
        // Ngủ một chút để UI kịp phản ứng khi chuột hover qua trước khi click (tùy chọn)
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.click();
    }

    /// Nhập văn bản bằng bàn phím ảo
    pub fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            let _ = self.enigo.text(&ch.to_string());
            std::thread::sleep(std::time::Duration::from_millis(20)); // Chậm lại một chút để tránh làm crash trình duyệt
        }
    }

    /// Nhấn một phím đặc biệt (Enter, Tab, v.v.)
    pub fn press_enter(&mut self) {
        let _ = self.enigo.key(Key::Return, Direction::Click);
    }
}
