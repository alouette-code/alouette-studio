# ─────────────────────────────────────────────────────────
# Alouette Studio — Makefile
# Tauri-like development workflow cho Flutter + Rust
# ─────────────────────────────────────────────────────────

.PHONY: dev dev-clean build-rust codegen codegen-watch clean help

help:  ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ── Development ──

dev:  ## 🚀 Start dev mode (giống tauri dev) — tự động watch Rust + FRB codegen + Flutter run
	./scripts/dev.sh

dev-clean:  ## 🧹 Clean build + start dev mode
	./scripts/dev.sh --clean

# ── Build Rust ──

build-rust:  ## 🔨 Build Rust library (.so / .dylib / .dll)
	cd flutter_app/rust && cargo build --release

build-rust-debug:  ## 🔨 Build Rust library (debug mode, nhanh hơn)
	cd flutter_app/rust && cargo build

# ── FRB Codegen ──

codegen:  ## 🦀 Run flutter_rust_bridge_codegen (sinh Dart bindings từ Rust API)
	flutter_rust_bridge_codegen generate

codegen-watch:  ## 👀 Watch Rust API changes → auto-regenerate Dart bindings
	flutter_rust_bridge_codegen generate --watch

# ── Flutter ──

run:  ## ▶️ Run Flutter app (không watch Rust)
	cd flutter_app && flutter run

analyze:  ## 🔍 Analyze Dart code
	cd flutter_app && flutter analyze

# ── Clean ──

clean:  ## 🧹 Clean all build artifacts
	cd flutter_app/rust && cargo clean 2>/dev/null || true
	rm -rf target 2>/dev/null || true
	rm -rf flutter_app/lib/src/rust 2>/dev/null || true

# ── Install dependencies ──

install:  ## 📦 Install all dependencies (Rust, Flutter, FRB codegen)
	cargo install flutter_rust_bridge_codegen
	cd flutter_app && flutter pub get
