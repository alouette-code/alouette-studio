use image::DynamicImage;
use imageproc::edges::canny;
use rusty_tesseract::{Args, Image};
use std::collections::HashMap;

/// Tọa độ của một khối nhận diện được trên màn hình
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub text: Option<String>,
}

pub struct ScreenParser;

impl ScreenParser {
    /// Thuật toán 1: Quét tỉ lệ cửa sổ và chia lưới
    /// Trả về mảng các cell lưới với tọa độ
    pub fn create_grid(width: u32, height: u32, rows: u32, cols: u32) -> Vec<BoundingBox> {
        let mut grid = Vec::new();
        let cell_w = width / cols;
        let cell_h = height / rows;

        for r in 0..rows {
            for c in 0..cols {
                grid.push(BoundingBox {
                    x: c * cell_w,
                    y: r * cell_h,
                    width: cell_w,
                    height: cell_h,
                    text: None,
                });
            }
        }
        grid
    }

    /// Thuật toán 2: Phân tích các khối hình vuông do đường viền tạo thành
    pub fn detect_blocks(image: &DynamicImage) -> Vec<BoundingBox> {
        let luma_img = image.to_luma8();
        // Canny edge detection
        let _edges = canny(&luma_img, 50.0, 100.0);
        
        // Trong thực tế, chúng ta sẽ cần thuật toán tìm contours (như find_contours của OpenCV)
        // Hiện tại chỉ trả về mảng rỗng như một placeholder cho logic này.
        // Rust `imageproc` không có find_contours tích hợp sẵn tốt như OpenCV.
        // Ta có thể chia ảnh thành các blocks nhỏ và đếm số lượng viền để tìm vùng có object.
        
        vec![]
    }

    /// Thuật toán 3: Chạy OCR trên một vùng lưới/tọa độ cụ thể để nhận diện chữ
    pub fn recognize_text(image_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let img = Image::from_path(image_path)?;
        
        let args = Args {
            lang: "eng".to_string(), // Có thể đổi thành "vie" nếu cần tiếng Việt
            config_variables: HashMap::new(),
            dpi: Some(150),
            psm: Some(3), // Fully automatic page segmentation
            oem: Some(3), // Default OCR Engine Mode
        };

        let output = rusty_tesseract::image_to_string(&img, &args)?;
        Ok(output.trim().to_string())
    }
}
