#!/usr/bin/env bash
# Không dùng set -e để cargo build fail không làm chết script
# Cargo output được show real-time, đầy đủ màu sắc, line numbers
set -uo pipefail

# ─────────────────────────────────────────────────────────
# Alouette Studio — Dev Mode (Tauri-like hot-reload cho Flutter + Rust)
#
# Cơ chế:
#   1. Watcher 1 → flutter_rust_bridge_codegen generate --watch
#      - Theo dõi file .rs trong flutter_app/rust/src/api/
#      - Khi có thay đổi → tự động sinh lại Dart bindings (có log chi tiết)
#
#   2. Watcher 2 → inotifywait/fswatch/poll → cargo build --release
#      - Theo dõi flutter_app/rust/ và core_engine/
#      - Khi có thay đổi → tự động rebuild .so
#      - HIỂN THỊ TOÀN BỘ CARGO OUTPUT real-time
#
#   3. Flutter run — chạy Flutter app, gõ 'r' để hot restart
#
# Usage:
#   ./scripts/dev.sh            # Dev mode
#   ./scripts/dev.sh --clean    # Clean build trước khi chạy
#   ./scripts/dev.sh --no-flutter  # Chỉ watch Rust, không chạy Flutter
#   ./scripts/dev.sh --no-watch    # Chỉ build Rust + chạy Flutter, không watch
#
# ─────────────────────────────────────────────────────────

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUST_DIR="$ROOT_DIR/flutter_app/rust"
FLUTTER_DIR="$ROOT_DIR/flutter_app"
BUILD_LOG="$ROOT_DIR/.rust_build.log"

NO_FLUTTER=false
NO_WATCH=false

cleanup() {
    echo ""
    echo "⏹  Cleaning up watchers..."
    jobs -p 2>/dev/null | xargs kill -9 2>/dev/null || true
    wait 2>/dev/null || true
    rm -f "$BUILD_LOG" 2>/dev/null || true
    echo "✅ Done. Goodbye!"
}
trap cleanup EXIT INT TERM

# ── Parse flags ──
for arg in "$@"; do
    case "$arg" in
        --clean)    CLEAN=true ;;
        --no-flutter) NO_FLUTTER=true ;;
        --no-watch)  NO_WATCH=true ;;
    esac
done

if [[ "${CLEAN:-}" == "true" ]]; then
    echo "🧹 Clean build..."
    cd "$RUST_DIR" && cargo clean 2>/dev/null || true
    rm -rf "$ROOT_DIR/target" 2>/dev/null || true
    rm -rf "$FLUTTER_DIR/lib/src/rust" 2>/dev/null || true
fi

# ── Export cargo env để đảm bảo output luôn có màu + progress ──
export CARGO_TERM_COLOR=always
# Dùng script để fake TTY giữ progress bar của cargo
export TERM=xterm-256color

# ── Màu sắc cho terminal ──
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  🚀  Alouette Studio — Dev Mode${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# ── Step 1: Check prerequisites ──
echo -e "${BLUE}🔍${NC} Checking prerequisites..."

if ! command -v flutter_rust_bridge_codegen &>/dev/null; then
    echo -e "${YELLOW}📦${NC} Installing flutter_rust_bridge_codegen..."
    cargo install flutter_rust_bridge_codegen
fi

if ! command -v cargo &>/dev/null; then
    echo -e "${RED}❌${NC} Rust/Cargo not found! Install from https://rustup.rs"
    exit 1
fi

if ! command -v flutter &>/dev/null; then
    echo -e "${RED}❌${NC} Flutter not found!"
    exit 1
fi

# File watcher detection
if command -v inotifywait &>/dev/null; then
    FILE_WATCHER="inotifywait"
elif command -v fswatch &>/dev/null; then
    FILE_WATCHER="fswatch"
else
    echo -e "${YELLOW}⚠️${NC}  No file watcher found. Install inotify-tools (Linux) or fswatch (macOS)"
    echo "   Ubuntu/Debian: sudo apt install inotify-tools"
    echo "   macOS: brew install fswatch"
    echo "   Falling back to polling mode (every 2s)..."
    FILE_WATCHER="poll"
fi

echo -e "${GREEN}✅${NC} Prerequisites check passed."
echo ""

# ── Step 2: First-time code generation ──
echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}  🦀  Step 1/3: flutter_rust_bridge_codegen (sinh Dart bindings)${NC}"
echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
echo ""
cd "$ROOT_DIR"
flutter_rust_bridge_codegen generate
echo ""
echo -e "${GREEN}✅${NC} Codegen complete."
echo ""

# ── Step 3: Build Rust library first time ──
echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}  🔨  Step 2/3: Building Rust library (lần đầu — sẽ hơi lâu)${NC}"
echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
echo ""
cd "$RUST_DIR"
cargo build --release
echo ""
echo -e "${GREEN}✅${NC} Rust build complete."
echo ""

# ── Step 4: Start watchers ──
if [[ "$NO_WATCH" == "false" ]]; then
    echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${YELLOW}  👀  Step 3/3: Starting file watchers${NC}"
    echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
    echo ""

    # Watcher 1: FRB codegen watch
    echo -e "${BLUE}👀${NC} Watcher 1: FRB codegen (Rust API → Dart bindings)..."
    cd "$ROOT_DIR"
    flutter_rust_bridge_codegen generate --watch &
    CODEGEN_PID=$!

    # Watcher 2: Rust source → rebuild .so (FULL CARGO OUTPUT, màu sắc đầy đủ)
    echo -e "${BLUE}👀${NC} Watcher 2: Rust source (.rs changes → cargo build)..."
    echo -e "    ${CYAN}Cargo sẽ show chi tiết: file đang compile, errors, warnings, line numbers${NC}"
    echo ""

    run_cargo_build() {
        cd "$RUST_DIR"
        echo ""
        # Chạy cargo trực tiếp — không pipe, không grep — output hiện real-time
        cargo build --release 2>&1
        local EXIT_CODE=$?
        echo ""
        echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        if [ $EXIT_CODE -eq 0 ]; then
            echo -e "${GREEN}  ✅  Rust rebuild complete!${NC}"
            echo -e "  📝  Gõ ${YELLOW}'r'${NC} + Enter trong terminal Flutter để hot restart"
        else
            echo -e "${RED}  ❌  Rust build FAILED (exit code: $EXIT_CODE)${NC}"
            echo -e "  ${RED}  ↑ Xem lỗi bên trên — sửa xong lưu file, watcher sẽ tự build lại${NC}"
        fi
        echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        return $EXIT_CODE
    }

    watch_rust() {
        case "${FILE_WATCHER:-poll}" in
            inotifywait)
                while true; do
                    inotifywait -q -r -e modify,create,delete,move \
                        --exclude 'target/' \
                        "$RUST_DIR/src" \
                        "$ROOT_DIR/core_engine/src" 2>/dev/null
                    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
                    echo -e "${YELLOW}  🔄  Rust file changed! Building...${NC}"
                    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
                    run_cargo_build
                done
                ;;
            fswatch)
                fswatch -o -e "target" "$RUST_DIR/src" "$ROOT_DIR/core_engine/src" | while read -r; do
                    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
                    echo -e "${YELLOW}  🔄  Rust file changed! Building...${NC}"
                    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
                    run_cargo_build
                done
                ;;
            poll)
                while true; do
                    sleep 2
                    local changed=0
                    local so_file="$RUST_DIR/target/release/libalouette_bridge.so"
                    for dir in "$RUST_DIR/src" "$ROOT_DIR/core_engine/src"; do
                        if [ -d "$dir" ]; then
                            if [ ! -f "$so_file" ]; then
                                changed=1
                                break
                            fi
                            local new_mod
                            new_mod=$(find "$dir" -name '*.rs' -newer "$so_file" 2>/dev/null | head -1)
                            if [ -n "$new_mod" ]; then
                                changed=1
                                break
                            fi
                        fi
                    done
                    if [ "$changed" -eq 1 ]; then
                        echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
                        echo -e "${YELLOW}  🔄  Rust file changed! Building...${NC}"
                        echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
                        run_cargo_build
                    fi
                done
                ;;
        esac
    }

    watch_rust &
    BUILD_PID=$!

    echo ""
fi

# ── Step 5: Run Flutter app ──
if [[ "$NO_FLUTTER" == "true" ]]; then
    echo -e "${BLUE}⏸${NC} Chế độ --no-flutter: Chỉ watch Rust, không chạy Flutter."
    echo "   Nhấn Ctrl+C để dừng."
    wait
else
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  🎯  Starting Flutter app...${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "  ${YELLOW}📝  Cheat sheet:${NC}"
    echo -e "  ${YELLOW}•${NC} 'r' + Enter  → Hot restart (nạp .so mới sau rebuild Rust)"
    echo -e "  ${YELLOW}•${NC} 'R' + Enter  → Hot reload (UI change)"
    echo -e "  ${YELLOW}•${NC} 'q' + Enter  → Thoát"
    echo ""
    echo -e "  ${CYAN}⚡  Workflow khi sửa Rust:${NC}"
    echo -e "  1. Sửa file .rs → lưu"
    echo -e "  2. ⏳ Cargo build chạy — ${YELLOW}xem output ngay bên dưới${NC}"
    echo -e "  3. ✅ Build xong → quay lại đây, gõ ${YELLOW}'r'${NC} → Enter"
    echo -e "  4. 🔄 Flutter hot restart → .so mới được nạp"
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo ""

    cd "$FLUTTER_DIR"
    flutter run
fi
