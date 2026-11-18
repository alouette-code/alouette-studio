use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use image::imageops::FilterType;
use image::{DynamicImage, ImageOutputFormat, RgbaImage};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use vnc::{PixelFormat, Rect, VncConnector, VncEncoding, VncEvent, X11Event};

#[derive(Debug, Clone, Serialize)]
pub struct DirtyRegion {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct OcrTextBox {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub cx: i32,
    pub cy: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct VmObservation {
    pub frame_seq: u64,
    pub changed: bool,
    pub width: u32,
    pub height: u32,
    pub dirty_regions: Vec<DirtyRegion>,
    pub ocr_boxes: Vec<OcrTextBox>,
    pub ocr_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_base64_png: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_error: Option<String>,
}

impl VmObservation {
    /// Deterministically resolves a target UI element from OCR boxes:
    /// 1. Exact case-sensitive match
    /// 2. Exact case-insensitive match
    /// 3. Substring / multi-word phrase match across adjacent boxes on the same line
    /// 4. Fuzzy match (Levenshtein distance <= 2 for queries >= 4 chars)
    /// Returns `(cx, cy, matched_text)`.
    pub fn find_text_target(&self, query: &str, occurrence: usize) -> Option<(i32, i32, String)> {
        let q = query.trim();
        if q.is_empty() || self.ocr_boxes.is_empty() {
            return None;
        }
        let q_lower = q.to_lowercase();

        // 1. Exact case-sensitive
        let exact: Vec<(i32, i32, String)> = self
            .ocr_boxes
            .iter()
            .filter(|b| b.text == q)
            .map(|b| (b.cx, b.cy, b.text.clone()))
            .collect();
        if let Some(hit) = exact.get(occurrence) {
            return Some(hit.clone());
        }

        // 2. Exact case-insensitive
        let exact_ci: Vec<(i32, i32, String)> = self
            .ocr_boxes
            .iter()
            .filter(|b| b.text.to_lowercase() == q_lower)
            .map(|b| (b.cx, b.cy, b.text.clone()))
            .collect();
        if let Some(hit) = exact_ci.get(occurrence) {
            return Some(hit.clone());
        }

        // 3. Multi-word phrase across adjacent boxes on the same horizontal row
        if q_lower.contains(' ') {
            let phrase_hits = self.find_phrase_matches(&q_lower);
            if let Some(hit) = phrase_hits.get(occurrence) {
                return Some(hit.clone());
            }
        }

        // 4. Case-insensitive substring match on individual boxes
        let substr: Vec<(i32, i32, String)> = self
            .ocr_boxes
            .iter()
            .filter(|b| b.text.to_lowercase().contains(&q_lower))
            .map(|b| (b.cx, b.cy, b.text.clone()))
            .collect();
        if let Some(hit) = substr.get(occurrence) {
            return Some(hit.clone());
        }

        // 5. Fuzzy match (Levenshtein <= 2) for OCR character noise (e.g. 'l' vs '1', 'O' vs '0')
        if q_lower.chars().count() >= 4 {
            let max_dist = if q_lower.chars().count() >= 7 { 2 } else { 1 };
            let mut fuzzy: Vec<(usize, i32, i32, String)> = self
                .ocr_boxes
                .iter()
                .filter_map(|b| {
                    let b_lower = b.text.to_lowercase();
                    let d = levenshtein(&q_lower, &b_lower);
                    if d <= max_dist {
                        Some((d, b.cx, b.cy, b.text.clone()))
                    } else {
                        None
                    }
                })
                .collect();
            fuzzy.sort_by_key(|item| item.0);
            if let Some((_, cx, cy, txt)) = fuzzy.get(occurrence) {
                return Some((*cx, *cy, txt.clone()));
            }
        }

        None
    }

    fn find_phrase_matches(&self, q_lower: &str) -> Vec<(i32, i32, String)> {
        let mut sorted: Vec<&OcrTextBox> = self.ocr_boxes.iter().collect();
        sorted.sort_by(|a, b| a.cy.cmp(&b.cy).then_with(|| a.x.cmp(&b.x)));

        // Group boxes into horizontal lines
        let mut lines: Vec<Vec<&OcrTextBox>> = Vec::new();
        for b in sorted {
            if let Some(last_line) = lines.last_mut() {
                let ref_box = last_line[0];
                let tol = (ref_box.h.max(b.h) / 2).max(10);
                if (b.cy - ref_box.cy).abs() <= tol {
                    last_line.push(b);
                    continue;
                }
            }
            lines.push(vec![b]);
        }

        let mut hits = Vec::new();
        for mut line in lines {
            line.sort_by_key(|b| b.x);
            for start in 0..line.len() {
                let mut combined = String::new();
                let mut min_x = i32::MAX;
                let mut min_y = i32::MAX;
                let mut max_x = i32::MIN;
                let mut max_y = i32::MIN;
                for end in start..line.len() {
                    if !combined.is_empty() {
                        combined.push(' ');
                    }
                    combined.push_str(&line[end].text);
                    min_x = min_x.min(line[end].x);
                    min_y = min_y.min(line[end].y);
                    max_x = max_x.max(line[end].x + line[end].w);
                    max_y = max_y.max(line[end].y + line[end].h);

                    let c_lower = combined.to_lowercase();
                    if c_lower == q_lower || c_lower.contains(q_lower) {
                        hits.push(((min_x + max_x) / 2, (min_y + max_y) / 2, combined.clone()));
                        break;
                    }
                    if c_lower.len() > q_lower.len() + 12 {
                        break;
                    }
                }
            }
        }
        hits
    }
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];
    for (i, &ca) in a_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b_chars.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1)
                .min(curr[j] + 1)
                .min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

struct FrameState {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    frame_seq: u64,
    recent_dirty: Vec<(u64, DirtyRegion)>,
    last_error: Option<String>,
}

impl Default for FrameState {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            rgba: Vec::new(),
            frame_seq: 0,
            recent_dirty: Vec::new(),
            last_error: None,
        }
    }
}

pub struct VmFrameStream {
    state: Arc<RwLock<FrameState>>,
    stop_flag: Arc<AtomicBool>,
}

impl VmFrameStream {
    /// Starts a persistent async VNC client loop against `127.0.0.1:{vnc_port}`.
    /// Keeps the latest RGBA framebuffer and dirty rects in RAM with zero disk I/O.
    pub fn start(vnc_port: u16) -> Arc<Self> {
        let state = Arc::new(RwLock::new(FrameState::default()));
        let stop_flag = Arc::new(AtomicBool::new(false));

        let state_bg = state.clone();
        let stop_bg = stop_flag.clone();

        let spawn_loop = move || {
            let fut = Self::run_vnc_loop(vnc_port, state_bg, stop_bg);
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(fut);
            } else {
                std::thread::spawn(move || {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        rt.block_on(fut);
                    }
                });
            }
        };

        spawn_loop();

        Arc::new(Self { state, stop_flag })
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Waits until the framebuffer stops changing for `idle_ms` (or until `timeout_ms` elapses).
    /// Returns the stabilized `frame_seq`.
    pub fn wait_for_idle(&self, idle_ms: u64, timeout_ms: u64) -> u64 {
        let idle_dur = Duration::from_millis(idle_ms.clamp(50, 5000));
        let deadline = Instant::now() + Duration::from_millis(timeout_ms.clamp(50, 15_000));
        let mut last_seq = self.current_seq();
        let mut last_change = Instant::now();

        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(40));
            let now_seq = self.current_seq();
            if now_seq != last_seq {
                last_seq = now_seq;
                last_change = Instant::now();
            } else if last_seq > 0 && last_change.elapsed() >= idle_dur {
                return now_seq;
            }
        }
        self.current_seq()
    }

    /// Polls OCR only when `frame_seq` changes until `query` appears on screen or `timeout_ms` expires.
    pub fn wait_for_text(&self, query: &str, timeout_ms: u64) -> Result<VmObservation, String> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms.clamp(100, 15_000));
        let mut last_checked_seq: Option<u64> = None;

        loop {
            let cur_seq = self.current_seq();
            if last_checked_seq != Some(cur_seq) {
                let obs = self.observe(None, 0, true, false, 1024);
                last_checked_seq = Some(obs.frame_seq);
                if obs.find_text_target(query, 0).is_some() {
                    return Ok(obs);
                }
                if Instant::now() >= deadline {
                    return Err(format!(
                        "Timed out after {}ms waiting for text '{}' on VM display. Visible OCR: {}",
                        timeout_ms,
                        query,
                        if obs.ocr_summary.is_empty() {
                            "(none)".to_string()
                        } else {
                            obs.ocr_summary
                        }
                    ));
                }
            }
            if Instant::now() >= deadline {
                let obs = self.observe(None, 0, true, false, 1024);
                return Err(format!(
                    "Timed out after {}ms waiting for text '{}' on VM display. Visible OCR: {}",
                    timeout_ms,
                    query,
                    if obs.ocr_summary.is_empty() {
                        "(none)".to_string()
                    } else {
                        obs.ocr_summary
                    }
                ));
            }
            std::thread::sleep(Duration::from_millis(80));
        }
    }

    pub fn current_resolution(&self) -> (u32, u32) {
        let s = self.state.read();
        if s.width > 0 && s.height > 0 {
            (s.width, s.height)
        } else {
            (1024, 768)
        }
    }

    pub fn current_seq(&self) -> u64 {
        self.state.read().frame_seq
    }

    async fn run_vnc_loop(
        vnc_port: u16,
        state: Arc<RwLock<FrameState>>,
        stop_flag: Arc<AtomicBool>,
    ) {
        let addr = format!("127.0.0.1:{}", vnc_port);
        let mut backoff_ms = 250u64;

        while !stop_flag.load(Ordering::Relaxed) {
            match TcpStream::connect(&addr).await {
                Ok(tcp) => {
                    backoff_ms = 250;
                    let connector = VncConnector::new(tcp)
                        .set_auth_method(async move { Ok(String::new()) })
                        .add_encoding(VncEncoding::Tight)
                        .add_encoding(VncEncoding::Zrle)
                        .add_encoding(VncEncoding::CopyRect)
                        .add_encoding(VncEncoding::Raw)
                        .add_encoding(VncEncoding::DesktopSizePseudo)
                        .allow_shared(true)
                        .set_pixel_format(PixelFormat::rgba());

                    let client_res = match connector.build() {
                        Ok(built) => match built.try_start().await {
                            Ok(started) => started.finish(),
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    };

                    let vnc = match client_res {
                        Ok(c) => {
                            state.write().last_error = None;
                            c
                        }
                        Err(e) => {
                            state.write().last_error = Some(format!("VNC handshake failed: {}", e));
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            backoff_ms = (backoff_ms * 2).min(2000);
                            continue;
                        }
                    };

                    let _ = vnc.input(X11Event::FullRefresh).await;
                    let mut last_refresh = Instant::now();

                    while !stop_flag.load(Ordering::Relaxed) {
                        match tokio::time::timeout(Duration::from_millis(33), vnc.poll_event()).await
                        {
                            Ok(Ok(Some(event))) => {
                                Self::apply_vnc_event(&state, event);
                            }
                            Ok(Ok(None)) => {}
                            Ok(Err(e)) => {
                                state.write().last_error = Some(format!("VNC stream error: {}", e));
                                break;
                            }
                            Err(_) => {
                                // Poll timeout: request incremental frame update (~30Hz)
                            }
                        }

                        if last_refresh.elapsed() >= Duration::from_millis(33) {
                            if vnc.input(X11Event::Refresh).await.is_err() {
                                break;
                            }
                            last_refresh = Instant::now();
                        }
                    }

                    let _ = vnc.close().await;
                }
                Err(e) => {
                    state.write().last_error = Some(format!("VNC connect failed ({}): {}", addr, e));
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(2000);
                }
            }
        }
    }

    fn apply_vnc_event(state: &Arc<RwLock<FrameState>>, event: VncEvent) {
        let mut s = state.write();
        match event {
            VncEvent::SetResolution(screen) => {
                let w = screen.width as u32;
                let h = screen.height as u32;
                if w > 0 && h > 0 && (s.width != w || s.height != h) {
                    s.width = w;
                    s.height = h;
                    s.rgba = vec![0; (w * h * 4) as usize];
                    s.frame_seq += 1;
                    Self::push_dirty(
                        &mut s,
                        DirtyRegion {
                            x: 0,
                            y: 0,
                            width: screen.width,
                            height: screen.height,
                        },
                    );
                }
            }
            VncEvent::DesktopUpdate(update) => {
                if let Some(layout) = update.layout {
                    let w = layout.width as u32;
                    let h = layout.height as u32;
                    if w > 0 && h > 0 && (s.width != w || s.height != h) {
                        s.width = w;
                        s.height = h;
                        s.rgba.resize((w * h * 4) as usize, 0);
                        s.frame_seq += 1;
                    }
                }
            }
            VncEvent::RawImage(rect, data) => {
                Self::blit_rgba_rect(&mut s, rect, &data);
            }
            VncEvent::JpegImage(rect, jpeg_bytes) => {
                if let Ok(img) = image::load_from_memory(&jpeg_bytes) {
                    let rgba = img.to_rgba8();
                    let w = rgba.width().min(rect.width as u32) as u16;
                    let h = rgba.height().min(rect.height as u32) as u16;
                    let adjusted = Rect {
                        x: rect.x,
                        y: rect.y,
                        width: w,
                        height: h,
                    };
                    Self::blit_rgba_rect(&mut s, adjusted, rgba.as_raw());
                }
            }
            VncEvent::Copy(dst, src) => {
                Self::copy_rect(&mut s, dst, src);
            }
            VncEvent::Error(msg) => {
                s.last_error = Some(msg);
            }
            _ => {}
        }
    }

    fn blit_rgba_rect(s: &mut FrameState, rect: Rect, data: &[u8]) {
        if s.width == 0 || s.height == 0 || rect.width == 0 || rect.height == 0 {
            return;
        }
        let fb_w = s.width as usize;
        let fb_h = s.height as usize;
        let rx = rect.x as usize;
        let ry = rect.y as usize;
        let rw = rect.width as usize;
        let rh = rect.height as usize;

        if rx >= fb_w || ry >= fb_h {
            return;
        }

        let copy_w = rw.min(fb_w - rx);
        let copy_h = rh.min(fb_h - ry);
        let row_bytes = copy_w * 4;

        for row in 0..copy_h {
            let src_start = row * rw * 4;
            let src_end = src_start + row_bytes;
            let dst_start = ((ry + row) * fb_w + rx) * 4;
            let dst_end = dst_start + row_bytes;
            if src_end <= data.len() && dst_end <= s.rgba.len() {
                s.rgba[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
            }
        }

        s.frame_seq += 1;
        Self::push_dirty(
            s,
            DirtyRegion {
                x: rect.x,
                y: rect.y,
                width: copy_w as u16,
                height: copy_h as u16,
            },
        );
    }

    fn copy_rect(s: &mut FrameState, dst: Rect, src: Rect) {
        if s.width == 0 || s.height == 0 {
            return;
        }
        let fb_w = s.width as usize;
        let fb_h = s.height as usize;
        let cw = (src.width.min(dst.width)) as usize;
        let ch = (src.height.min(dst.height)) as usize;
        if cw == 0 || ch == 0 {
            return;
        }

        let cw = cw.min(fb_w.saturating_sub(src.x as usize)).min(fb_w.saturating_sub(dst.x as usize));
        let ch = ch.min(fb_h.saturating_sub(src.y as usize)).min(fb_h.saturating_sub(dst.y as usize));
        if cw == 0 || ch == 0 {
            return;
        }

        let mut tmp = vec![0u8; cw * ch * 4];
        for row in 0..ch {
            let s_idx = ((src.y as usize + row) * fb_w + src.x as usize) * 4;
            let t_idx = row * cw * 4;
            tmp[t_idx..t_idx + cw * 4].copy_from_slice(&s.rgba[s_idx..s_idx + cw * 4]);
        }
        for row in 0..ch {
            let d_idx = ((dst.y as usize + row) * fb_w + dst.x as usize) * 4;
            let t_idx = row * cw * 4;
            s.rgba[d_idx..d_idx + cw * 4].copy_from_slice(&tmp[t_idx..t_idx + cw * 4]);
        }

        s.frame_seq += 1;
        Self::push_dirty(
            s,
            DirtyRegion {
                x: dst.x,
                y: dst.y,
                width: cw as u16,
                height: ch as u16,
            },
        );
    }

    fn push_dirty(s: &mut FrameState, region: DirtyRegion) {
        let seq = s.frame_seq;
        s.recent_dirty.push((seq, region));
        if s.recent_dirty.len() > 64 {
            let excess = s.recent_dirty.len() - 64;
            s.recent_dirty.drain(..excess);
        }
    }

    /// Waits up to `wait_change_ms` for the framebuffer sequence to advance beyond `since_seq`,
    /// then produces a lightweight observation from RAM.
    pub fn observe(
        &self,
        since_seq: Option<u64>,
        wait_change_ms: u64,
        include_ocr: bool,
        include_image_base64: bool,
        max_image_dim: u32,
    ) -> VmObservation {
        let target_since = since_seq.unwrap_or(0);
        if wait_change_ms > 0 {
            let deadline = Instant::now() + Duration::from_millis(wait_change_ms.min(5000));
            while Instant::now() < deadline {
                let s = self.state.read();
                if s.width > 0 && s.frame_seq > target_since {
                    break;
                }
                drop(s);
                std::thread::sleep(Duration::from_millis(30));
            }
        }

        let (width, height, rgba, frame_seq, dirty_regions, stream_error) = {
            let s = self.state.read();
            let dirty: Vec<DirtyRegion> = s
                .recent_dirty
                .iter()
                .filter(|(seq, _)| *seq > target_since)
                .map(|(_, r)| r.clone())
                .collect();
            (
                s.width,
                s.height,
                s.rgba.clone(),
                s.frame_seq,
                dirty,
                s.last_error.clone(),
            )
        };

        let changed = frame_seq > target_since;
        if width == 0 || height == 0 || rgba.len() != (width * height * 4) as usize {
            return VmObservation {
                frame_seq,
                changed,
                width,
                height,
                dirty_regions,
                ocr_boxes: Vec::new(),
                ocr_summary: String::new(),
                image_base64_png: None,
                stream_error: stream_error.or_else(|| Some("Framebuffer not ready yet".to_string())),
            };
        }

        let rgba_img = match RgbaImage::from_raw(width, height, rgba) {
            Some(img) => img,
            None => {
                return VmObservation {
                    frame_seq,
                    changed,
                    width,
                    height,
                    dirty_regions,
                    ocr_boxes: Vec::new(),
                    ocr_summary: String::new(),
                    image_base64_png: None,
                    stream_error: Some("Failed to construct RGBA image from buffer".to_string()),
                };
            }
        };
        let dyn_img = DynamicImage::ImageRgba8(rgba_img);

        let (ocr_boxes, ocr_summary) = if include_ocr {
            Self::extract_ocr_boxes(&dyn_img)
        } else {
            (Vec::new(), String::new())
        };

        let image_base64_png = if include_image_base64 {
            let max_dim = max_image_dim.clamp(256, 1920);
            let resized = if width > max_dim || height > max_dim {
                dyn_img.resize(max_dim, max_dim, FilterType::Triangle)
            } else {
                dyn_img
            };
            let mut png_bytes = Cursor::new(Vec::new());
            if resized
                .write_to(&mut png_bytes, ImageOutputFormat::Png)
                .is_ok()
            {
                Some(BASE64.encode(png_bytes.into_inner()))
            } else {
                None
            }
        } else {
            None
        };

        VmObservation {
            frame_seq,
            changed,
            width,
            height,
            dirty_regions,
            ocr_boxes,
            ocr_summary,
            image_base64_png,
            stream_error,
        }
    }

    fn extract_ocr_boxes(img: &DynamicImage) -> (Vec<OcrTextBox>, String) {
        let rgba = img.to_rgba8();
        let tess_dyn = match rusty_tesseract::image::RgbaImage::from_raw(
            rgba.width(),
            rgba.height(),
            rgba.into_raw(),
        ) {
            Some(buf) => rusty_tesseract::image::DynamicImage::ImageRgba8(buf),
            None => return (Vec::new(), String::new()),
        };

        let tess_img = match rusty_tesseract::Image::from_dynamic_image(&tess_dyn) {
            Ok(i) => i,
            Err(_) => return (Vec::new(), String::new()),
        };

        let args = rusty_tesseract::Args {
            lang: "eng".to_string(),
            config_variables: HashMap::new(),
            dpi: Some(150),
            psm: Some(3),
            oem: Some(3),
        };

        match rusty_tesseract::image_to_data(&tess_img, &args) {
            Ok(data_out) => {
                let mut boxes = Vec::new();
                let mut lines = Vec::new();
                for d in data_out.data {
                    let txt = d.text.trim();
                    if txt.is_empty() || d.conf < 35.0 || d.width <= 0 || d.height <= 0 {
                        continue;
                    }
                    let cx = d.left + d.width / 2;
                    let cy = d.top + d.height / 2;
                    lines.push(format!("\"{}\" @ ({}, {})", txt, cx, cy));
                    boxes.push(OcrTextBox {
                        text: txt.to_string(),
                        x: d.left,
                        y: d.top,
                        w: d.width,
                        h: d.height,
                        cx,
                        cy,
                    });
                    if boxes.len() >= 150 {
                        break;
                    }
                }
                (boxes, lines.join(" | "))
            }
            Err(_) => (Vec::new(), String::new()),
        }
    }
}

impl Drop for VmFrameStream {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_text_target_phrase_and_fuzzy() {
        let obs = VmObservation {
            frame_seq: 1,
            changed: true,
            width: 1024,
            height: 768,
            dirty_regions: vec![],
            ocr_boxes: vec![
                OcrTextBox { text: "Sign".into(), x: 100, y: 200, w: 40, h: 20, cx: 120, cy: 210 },
                OcrTextBox { text: "In".into(), x: 146, y: 200, w: 20, h: 20, cx: 156, cy: 210 },
                OcrTextBox { text: "Terminal".into(), x: 10, y: 50, w: 80, h: 20, cx: 50, cy: 60 },
            ],
            ocr_summary: String::new(),
            image_base64_png: None,
            stream_error: None,
        };

        let phrase = obs.find_text_target("sign in", 0).expect("phrase match");
        assert_eq!((phrase.0, phrase.1), (133, 210));

        let fuzzy = obs.find_text_target("Termnal", 0).expect("fuzzy match");
        assert_eq!((fuzzy.0, fuzzy.1), (50, 60));
    }
}
