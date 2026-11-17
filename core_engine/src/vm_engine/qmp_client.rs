use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VmInputAction {
    Move {
        x: u32,
        y: u32,
        #[serde(default)]
        screen_w: Option<u32>,
        #[serde(default)]
        screen_h: Option<u32>,
    },
    Click {
        x: u32,
        y: u32,
        #[serde(default)]
        button: Option<String>, // "left" | "right" | "middle"
        #[serde(default)]
        double: Option<bool>,
        #[serde(default)]
        screen_w: Option<u32>,
        #[serde(default)]
        screen_h: Option<u32>,
    },
    Scroll {
        #[serde(default)]
        x: Option<u32>,
        #[serde(default)]
        y: Option<u32>,
        direction: String, // "up" | "down"
        #[serde(default)]
        amount: Option<u32>,
        #[serde(default)]
        screen_w: Option<u32>,
        #[serde(default)]
        screen_h: Option<u32>,
    },
    Key {
        key: String, // e.g. "ret", "esc", "ctrl+c", "alt+tab", "super"
        #[serde(default)]
        down: Option<bool>, // None = press & release
    },
    Type {
        text: String,
        #[serde(default)]
        delay_ms: Option<u64>,
    },
    Wait {
        ms: u64,
    },
}

pub struct QmpClient {
    stream: UnixStream,
    buffer: Vec<u8>,
}

impl QmpClient {
    /// Connects to a QMP Unix socket and performs the initial capabilities handshake.
    pub fn connect<P: AsRef<Path>>(socket_path: P) -> Result<Self, String> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|e| format!("Failed to connect to QMP socket: {}", e))?;

        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| e.to_string())?;

        let mut client = Self {
            stream,
            buffer: Vec::new(),
        };

        let _greeting = client.read_response()?;
        client.execute("qmp_capabilities", None)?;
        Ok(client)
    }

    fn read_response(&mut self) -> Result<Value, String> {
        let mut chunk = [0u8; 4096];

        loop {
            if let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let line = self.buffer[..pos].to_vec();
                self.buffer.drain(..=pos);

                if line.iter().all(|b| b.is_ascii_whitespace()) {
                    continue;
                }
                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                    return Ok(value);
                }
                continue;
            }

            let bytes_read = self
                .stream
                .read(&mut chunk)
                .map_err(|e| format!("Failed to read from QMP socket: {}", e))?;

            if bytes_read == 0 {
                return Err("QMP socket closed unexpectedly".to_string());
            }

            self.buffer.extend_from_slice(&chunk[..bytes_read]);
        }
    }

    pub fn execute(&mut self, command: &str, arguments: Option<Value>) -> Result<Value, String> {
        let mut req = json!({
            "execute": command
        });

        if let Some(args) = arguments {
            req.as_object_mut()
                .unwrap()
                .insert("arguments".to_string(), args);
        }

        let mut req_str = req.to_string();
        req_str.push('\n');

        self.stream
            .write_all(req_str.as_bytes())
            .map_err(|e| format!("Failed to send QMP command: {}", e))?;

        loop {
            let response = self.read_response()?;
            if let Some(err) = response.get("error") {
                let desc = err
                    .get("desc")
                    .and_then(|d| d.as_str())
                    .unwrap_or("Unknown error");
                return Err(format!("QMP Error: {}", desc));
            }
            if let Some(res) = response.get("return") {
                return Ok(res.clone());
            }
        }
    }

    fn validate_snapshot_name(name: &str) -> Result<(), String> {
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(
                "Invalid snapshot name: only alphanumeric characters, '-', '_', and '.' are allowed."
                    .to_string(),
            );
        }
        Ok(())
    }

    fn run_hmp_checked(&mut self, cmd_line: String) -> Result<(), String> {
        let res = self.execute(
            "human-monitor-command",
            Some(json!({ "command-line": cmd_line })),
        )?;
        if let Some(output) = res.as_str() {
            let trimmed = output.trim();
            if !trimmed.is_empty()
                && (trimmed.to_lowercase().contains("error")
                    || trimmed.to_lowercase().contains("failed"))
            {
                return Err(format!("QEMU Monitor Error: {}", trimmed));
            }
        }
        Ok(())
    }

    pub fn save_snapshot(&mut self, name: &str) -> Result<(), String> {
        Self::validate_snapshot_name(name)?;
        self.run_hmp_checked(format!("savevm {}", name))
    }

    pub fn load_snapshot(&mut self, name: &str) -> Result<(), String> {
        Self::validate_snapshot_name(name)?;
        self.run_hmp_checked(format!("loadvm {}", name))
    }

    pub fn delete_snapshot(&mut self, name: &str) -> Result<(), String> {
        Self::validate_snapshot_name(name)?;
        self.run_hmp_checked(format!("delvm {}", name))
    }

    pub fn system_powerdown(&mut self) -> Result<(), String> {
        self.execute("system_powerdown", None)?;
        Ok(())
    }

    // --- Direct Guest Input Injection via QMP input-send-event ---

    pub fn send_input_events(&mut self, events: &[Value]) -> Result<(), String> {
        if events.is_empty() {
            return Ok(());
        }
        self.execute("input-send-event", Some(json!({ "events": events })))?;
        Ok(())
    }

    fn scale_abs_coord(val: u32, max_dim: u32) -> i64 {
        let denom = max_dim.max(1) as u64;
        let clamped = val.min(max_dim.max(1)) as u64;
        ((clamped * 32767) / denom).min(32767) as i64
    }

    fn abs_move_events(x: u32, y: u32, w: u32, h: u32) -> Vec<Value> {
        vec![
            json!({ "type": "abs", "data": { "axis": "x", "value": Self::scale_abs_coord(x, w) } }),
            json!({ "type": "abs", "data": { "axis": "y", "value": Self::scale_abs_coord(y, h) } }),
        ]
    }

    fn key_event(qcode: &str, down: bool) -> Value {
        json!({
            "type": "key",
            "data": {
                "key": { "type": "qcode", "data": qcode },
                "down": down
            }
        })
    }

    fn btn_event(button: &str, down: bool) -> Value {
        json!({
            "type": "btn",
            "data": {
                "button": button,
                "down": down
            }
        })
    }

    fn normalize_qcode(token: &str) -> Result<&'static str, String> {
        match token.trim().to_lowercase().as_str() {
            "ctrl" | "control" | "lctrl" => Ok("ctrl"),
            "rctrl" => Ok("ctrl_r"),
            "shift" | "lshift" => Ok("shift"),
            "rshift" => Ok("shift_r"),
            "alt" | "lalt" => Ok("alt"),
            "ralt" => Ok("alt_r"),
            "super" | "meta" | "win" | "cmd" => Ok("meta_l"),
            "enter" | "return" | "ret" => Ok("ret"),
            "esc" | "escape" => Ok("esc"),
            "tab" => Ok("tab"),
            "space" => Ok("spc"),
            "backspace" | "bksp" => Ok("backspace"),
            "delete" | "del" => Ok("delete"),
            "insert" | "ins" => Ok("insert"),
            "home" => Ok("home"),
            "end" => Ok("end"),
            "pgup" | "pageup" => Ok("pgup"),
            "pgdn" | "pagedown" => Ok("pgdn"),
            "up" => Ok("up"),
            "down" => Ok("down"),
            "left" => Ok("left"),
            "right" => Ok("right"),
            "capslock" => Ok("caps_lock"),
            "f1" => Ok("f1"),
            "f2" => Ok("f2"),
            "f3" => Ok("f3"),
            "f4" => Ok("f4"),
            "f5" => Ok("f5"),
            "f6" => Ok("f6"),
            "f7" => Ok("f7"),
            "f8" => Ok("f8"),
            "f9" => Ok("f9"),
            "f10" => Ok("f10"),
            "f11" => Ok("f11"),
            "f12" => Ok("f12"),
            "a" => Ok("a"),
            "b" => Ok("b"),
            "c" => Ok("c"),
            "d" => Ok("d"),
            "e" => Ok("e"),
            "f" => Ok("f"),
            "g" => Ok("g"),
            "h" => Ok("h"),
            "i" => Ok("i"),
            "j" => Ok("j"),
            "k" => Ok("k"),
            "l" => Ok("l"),
            "m" => Ok("m"),
            "n" => Ok("n"),
            "o" => Ok("o"),
            "p" => Ok("p"),
            "q" => Ok("q"),
            "r" => Ok("r"),
            "s" => Ok("s"),
            "t" => Ok("t"),
            "u" => Ok("u"),
            "v" => Ok("v"),
            "w" => Ok("w"),
            "x" => Ok("x"),
            "y" => Ok("y"),
            "z" => Ok("z"),
            "0" => Ok("0"),
            "1" => Ok("1"),
            "2" => Ok("2"),
            "3" => Ok("3"),
            "4" => Ok("4"),
            "5" => Ok("5"),
            "6" => Ok("6"),
            "7" => Ok("7"),
            "8" => Ok("8"),
            "9" => Ok("9"),
            "-" | "minus" => Ok("minus"),
            "=" | "equal" => Ok("equal"),
            "[" => Ok("bracket_left"),
            "]" => Ok("bracket_right"),
            "\\" => Ok("backslash"),
            ";" => Ok("semicolon"),
            "'" => Ok("apostrophe"),
            "`" => Ok("grave_accent"),
            "," => Ok("comma"),
            "." => Ok("dot"),
            "/" => Ok("slash"),
            other => Err(format!("Unsupported key token: '{}'", other)),
        }
    }

    fn char_to_qcode(ch: char) -> Option<(&'static str, bool)> {
        match ch {
            'a'..='z' => {
                let s = match ch {
                    'a' => "a", 'b' => "b", 'c' => "c", 'd' => "d", 'e' => "e",
                    'f' => "f", 'g' => "g", 'h' => "h", 'i' => "i", 'j' => "j",
                    'k' => "k", 'l' => "l", 'm' => "m", 'n' => "n", 'o' => "o",
                    'p' => "p", 'q' => "q", 'r' => "r", 's' => "s", 't' => "t",
                    'u' => "u", 'v' => "v", 'w' => "w", 'x' => "x", 'y' => "y",
                    'z' => "z",
                    _ => unreachable!(),
                };
                Some((s, false))
            }
            'A'..='Z' => {
                let lower = ch.to_ascii_lowercase();
                Self::char_to_qcode(lower).map(|(k, _)| (k, true))
            }
            '0' => Some(("0", false)), '1' => Some(("1", false)), '2' => Some(("2", false)),
            '3' => Some(("3", false)), '4' => Some(("4", false)), '5' => Some(("5", false)),
            '6' => Some(("6", false)), '7' => Some(("7", false)), '8' => Some(("8", false)),
            '9' => Some(("9", false)),
            ')' => Some(("0", true)), '!' => Some(("1", true)), '@' => Some(("2", true)),
            '#' => Some(("3", true)), '$' => Some(("4", true)), '%' => Some(("5", true)),
            '^' => Some(("6", true)), '&' => Some(("7", true)), '*' => Some(("8", true)),
            '(' => Some(("9", true)),
            ' ' => Some(("spc", false)),
            '\n' => Some(("ret", false)),
            '\t' => Some(("tab", false)),
            '-' => Some(("minus", false)), '_' => Some(("minus", true)),
            '=' => Some(("equal", false)), '+' => Some(("equal", true)),
            '[' => Some(("bracket_left", false)), '{' => Some(("bracket_left", true)),
            ']' => Some(("bracket_right", false)), '}' => Some(("bracket_right", true)),
            '\\' => Some(("backslash", false)), '|' => Some(("backslash", true)),
            ';' => Some(("semicolon", false)), ':' => Some(("semicolon", true)),
            '\'' => Some(("apostrophe", false)), '"' => Some(("apostrophe", true)),
            '`' => Some(("grave_accent", false)), '~' => Some(("grave_accent", true)),
            ',' => Some(("comma", false)), '<' => Some(("comma", true)),
            '.' => Some(("dot", false)), '>' => Some(("dot", true)),
            '/' => Some(("slash", false)), '?' => Some(("slash", true)),
            _ => None,
        }
    }

    pub fn send_key_combo(&mut self, key_spec: &str, down_only: Option<bool>) -> Result<(), String> {
        let parts: Vec<&str> = key_spec.split('+').map(str::trim).filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err("Empty key specification".to_string());
        }

        let mut qcodes = Vec::with_capacity(parts.len());
        for part in parts {
            qcodes.push(Self::normalize_qcode(part)?);
        }

        if let Some(down) = down_only {
            let events: Vec<Value> = qcodes.iter().map(|q| Self::key_event(q, down)).collect();
            self.send_input_events(&events)?;
            return Ok(());
        }

        let mut events = Vec::with_capacity(qcodes.len() * 2);
        for q in &qcodes {
            events.push(Self::key_event(q, true));
        }
        for q in qcodes.iter().rev() {
            events.push(Self::key_event(q, false));
        }
        self.send_input_events(&events)
    }

    pub fn type_text(&mut self, text: &str, delay_ms: u64) -> Result<(), String> {
        for ch in text.chars() {
            if let Some((qcode, need_shift)) = Self::char_to_qcode(ch) {
                let mut events = Vec::new();
                if need_shift {
                    events.push(Self::key_event("shift", true));
                }
                events.push(Self::key_event(qcode, true));
                events.push(Self::key_event(qcode, false));
                if need_shift {
                    events.push(Self::key_event("shift", false));
                }
                self.send_input_events(&events)?;
                if delay_ms > 0 {
                    std::thread::sleep(Duration::from_millis(delay_ms));
                }
            }
        }
        Ok(())
    }

    /// Executes a batch of GUI actions over this persistent QMP connection.
    pub fn execute_action_batch(
        &mut self,
        actions: &[VmInputAction],
        default_w: u32,
        default_h: u32,
    ) -> Result<usize, String> {
        let mut executed = 0usize;
        for action in actions {
            match action {
                VmInputAction::Move { x, y, screen_w, screen_h } => {
                    let w = screen_w.unwrap_or(default_w);
                    let h = screen_h.unwrap_or(default_h);
                    let events = Self::abs_move_events(*x, *y, w, h);
                    self.send_input_events(&events)?;
                }
                VmInputAction::Click {
                    x,
                    y,
                    button,
                    double,
                    screen_w,
                    screen_h,
                } => {
                    let w = screen_w.unwrap_or(default_w);
                    let h = screen_h.unwrap_or(default_h);
                    let btn = match button.as_deref().unwrap_or("left") {
                        "right" => "right",
                        "middle" => "middle",
                        _ => "left",
                    };
                    let move_ev = Self::abs_move_events(*x, *y, w, h);
                    self.send_input_events(&move_ev)?;
                    std::thread::sleep(Duration::from_millis(20));

                    let clicks = if double.unwrap_or(false) { 2 } else { 1 };
                    for i in 0..clicks {
                        self.send_input_events(&[
                            Self::btn_event(btn, true),
                            Self::btn_event(btn, false),
                        ])?;
                        if i + 1 < clicks {
                            std::thread::sleep(Duration::from_millis(50));
                        }
                    }
                }
                VmInputAction::Scroll {
                    x,
                    y,
                    direction,
                    amount,
                    screen_w,
                    screen_h,
                } => {
                    if let (Some(px), Some(py)) = (x, y) {
                        let w = screen_w.unwrap_or(default_w);
                        let h = screen_h.unwrap_or(default_h);
                        let move_ev = Self::abs_move_events(*px, *py, w, h);
                        self.send_input_events(&move_ev)?;
                    }
                    let wheel_btn = if direction.eq_ignore_ascii_case("up") {
                        "wheel-up"
                    } else {
                        "wheel-down"
                    };
                    let count = amount.unwrap_or(3).min(20);
                    for _ in 0..count {
                        self.send_input_events(&[
                            Self::btn_event(wheel_btn, true),
                            Self::btn_event(wheel_btn, false),
                        ])?;
                    }
                }
                VmInputAction::Key { key, down } => {
                    self.send_key_combo(key, *down)?;
                }
                VmInputAction::Type { text, delay_ms } => {
                    self.type_text(text, delay_ms.unwrap_or(8))?;
                }
                VmInputAction::Wait { ms } => {
                    std::thread::sleep(Duration::from_millis((*ms).min(15_000)));
                }
            }
            executed += 1;
        }
        Ok(executed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abs_coord_scaling_and_key_mapping() {
        assert_eq!(QmpClient::scale_abs_coord(0, 1920), 0);
        assert_eq!(QmpClient::scale_abs_coord(1920, 1920), 32767);
        assert_eq!(QmpClient::scale_abs_coord(960, 1920), 16383);
        assert_eq!(QmpClient::normalize_qcode("Ctrl").unwrap(), "ctrl");
        assert_eq!(QmpClient::normalize_qcode("Enter").unwrap(), "ret");
        assert_eq!(QmpClient::char_to_qcode('A'), Some(("a", true)));
        assert_eq!(QmpClient::char_to_qcode('/'), Some(("slash", false)));
    }
}
