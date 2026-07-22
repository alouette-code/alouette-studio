use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;
use sha2::{Digest, Sha256};
use wasmtime::*;
use wasmtime_wasi::{WasiCtxBuilder, ResourceTable};
use wasmtime_wasi::preview1::WasiP1Ctx;

pub struct WasmHostState {
    pub wasi: WasiP1Ctx,
    pub resource_table: ResourceTable,
    pub permissions: Vec<String>,
    pub extension_id: String,
}

pub struct WasmExtensionEngine {
    engine: Engine,
}

impl WasmExtensionEngine {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(false);
        config.async_support(true);
        config.consume_fuel(true);

        let engine = Engine::new(&config).context("Failed to initialize Wasmtime engine")?;
        Ok(Self { engine })
    }

    /// Kiểm tra mã hash SHA-256 của file .wasm
    pub fn verify_sha256(file_path: &Path, expected_sha256: &str) -> Result<bool> {
        let bytes = fs::read(file_path).context("Failed to read Wasm file for checksum")?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let result = hasher.finalize();
        let calculated_hash = format!("{:x}", result);

        Ok(calculated_hash.eq_ignore_ascii_case(expected_sha256.trim()))
    }

    /// Nạp và thực thi một hàm trong Wasm Module với môi trường Sandbox Wasmtime
    pub async fn execute_plugin(
        &self,
        extension_id: &str,
        wasm_bytes: &[u8],
        permissions: &[String],
        function_name: &str,
        _param_json: &str,
    ) -> Result<String> {
        let module = Module::new(&self.engine, wasm_bytes)
            .context("Failed to compile Wasm bytecode")?;

        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build_p1();

        let host_state = WasmHostState {
            wasi: wasi_ctx,
            resource_table: ResourceTable::new(),
            permissions: permissions.to_vec(),
            extension_id: extension_id.to_string(),
        };

        let mut store = Store::new(&self.engine, host_state);
        store.set_fuel(10_000_000).context("Failed to set Wasm fuel limit")?;

        let mut linker: Linker<WasmHostState> = Linker::new(&self.engine);
        wasmtime_wasi::preview1::add_to_linker_async(&mut linker, |s: &mut WasmHostState| &mut s.wasi)?;

        // Đăng ký Host API Interceptor
        linker.func_wrap_async(
            "alouette_host",
            "check_permission",
            |mut caller: Caller<'_, WasmHostState>, (perm_ptr, perm_len): (i32, i32)| {
                Box::new(async move {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return 0i32,
                    };
                    let data = memory.data(&caller);
                    let start = perm_ptr as usize;
                    let end = start + perm_len as usize;
                    if end > data.len() {
                        return 0i32;
                    }
                    if let Ok(perm_str) = std::str::from_utf8(&data[start..end]) {
                        if caller.data().permissions.iter().any(|p| p == perm_str) {
                            return 1i32;
                        }
                    }
                    0i32
                })
            },
        )?;

        let instance = linker.instantiate_async(&mut store, &module).await
            .context("Failed to instantiate Wasm module inside sandbox")?;

        // Tìm kiếm và thực thi entry point function
        if let Ok(func) = instance.get_typed_func::<(), ()>(&mut store, function_name) {
            func.call_async(&mut store, ()).await
                .map_err(|e| anyhow!("Wasm execution error/trap: {}", e))?;
            Ok("Wasm execution succeeded".to_string())
        } else {
            Err(anyhow!("Function '{}' not found in Wasm module", function_name))
        }
    }
}
