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

    /// Thuật toán 2: Phân tích các khối giao diện dựa trên mật độ cạnh Canny
    /// Chia ảnh thành các vùng lưới thích ứng (coarse 16x16px) và gom cụm các ô có cạnh biên.
    pub fn detect_blocks(image: &DynamicImage) -> Vec<BoundingBox> {
        let luma_img = image.to_luma8();
        let (w, h) = luma_img.dimensions();
        if w < 16 || h < 16 {
            return vec![];
        }

        let edges = canny(&luma_img, 50.0, 100.0);
        let tile = 16u32;
        let cols = w.div_ceil(tile) as usize;
        let rows = h.div_ceil(tile) as usize;
        let mut active = vec![false; rows * cols];

        // Mark 16x16 tiles containing enough Canny edge pixels as UI regions
        for r in 0..rows {
            let y0 = (r as u32) * tile;
            let y1 = (y0 + tile).min(h);
            for c in 0..cols {
                let x0 = (c as u32) * tile;
                let x1 = (x0 + tile).min(w);
                let mut edge_pixels = 0u32;
                for y in y0..y1 {
                    for x in x0..x1 {
                        if edges.get_pixel(x, y)[0] > 0 {
                            edge_pixels += 1;
                        }
                    }
                }
                if edge_pixels >= 6 {
                    active[r * cols + c] = true;
                }
            }
        }

        // 4-connected BFS to merge adjacent active tiles into bounding boxes
        let mut visited = vec![false; rows * cols];
        let mut blocks = Vec::new();
        let mut queue = std::collections::VecDeque::new();

        for start_r in 0..rows {
            for start_c in 0..cols {
                let idx = start_r * cols + start_c;
                if !active[idx] || visited[idx] {
                    continue;
                }
                visited[idx] = true;
                queue.push_back((start_r, start_c));
                let (mut min_r, mut max_r, mut min_c, mut max_c) =
                    (start_r, start_r, start_c, start_c);

                while let Some((r, c)) = queue.pop_front() {
                    min_r = min_r.min(r);
                    max_r = max_r.max(r);
                    min_c = min_c.min(c);
                    max_c = max_c.max(c);

                    let neighbors = [
                        r.checked_sub(1).map(|nr| (nr, c)),
                        (r + 1 < rows).then_some((r + 1, c)),
                        c.checked_sub(1).map(|nc| (r, nc)),
                        (c + 1 < cols).then_some((r, c + 1)),
                    ];
                    for (nr, nc) in neighbors.into_iter().flatten() {
                        let nidx = nr * cols + nc;
                        if active[nidx] && !visited[nidx] {
                            visited[nidx] = true;
                            queue.push_back((nr, nc));
                        }
                    }
                }

                let x = (min_c as u32) * tile;
                let y = (min_r as u32) * tile;
                let x2 = (((max_c + 1) as u32) * tile).min(w);
                let y2 = (((max_r + 1) as u32) * tile).min(h);
                blocks.push(BoundingBox {
                    x,
                    y,
                    width: x2 - x,
                    height: y2 - y,
                    text: None,
                });
            }
        }

        blocks
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn detect_blocks_finds_drawn_ui_rectangle() {
        let mut img = RgbImage::from_pixel(128, 128, Rgb([20, 20, 20]));
        // Draw a high-contrast white rectangle simulating a button/card
        for y in 32..64 {
            for x in 32..96 {
                img.put_pixel(x, y, Rgb([240, 240, 240]));
            }
        }
        let blocks = ScreenParser::detect_blocks(&DynamicImage::ImageRgb8(img));
        assert!(
            !blocks.is_empty(),
            "Expected at least one detected UI block around the white rectangle"
        );
    }
}
