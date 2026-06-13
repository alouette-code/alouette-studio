#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────
# Alouette Studio — Dev Mode (Tauri)
#
# Usage:
#   ./scripts/dev.sh
#   ./scripts/dev.sh --clean
# ─────────────────────────────────────────────────────────

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

cleanup() {
    echo ""
    echo "⏹  Cleaning up..."
    echo "✅ Done. Goodbye!"
}
trap cleanup EXIT INT TERM

# ── Parse flags ──
for arg in "$@"; do
    case "$arg" in
        --clean) CLEAN=true ;;
    esac
done

if [[ "${CLEAN:-}" == "true" ]]; then
    echo "🧹 Clean build..."
    cd "$ROOT_DIR/tauri_app/src-tauri" && cargo clean 2>/dev/null || true
fi

# ── Màu sắc ──
CYAN='\033[0;36m'
NC='\033[0m'

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  🚀  Alouette Studio — Tauri Dev Mode${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

cd "$ROOT_DIR/tauri_app"
cargo tauri dev
