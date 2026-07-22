use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use sha2::{Digest, Sha256};
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use super::manifest::{ExtensionManifest, PublisherInfo};

const SERVERLESS_REGISTRY_CDN: &str = "https://cdn.jsdelivr.net/gh/alouette-code/alouette-extension-registry@main/index.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryIndexItem {
    pub id: String, // e.g. "nhatanh.code-formatter"
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<PublisherInfo>,
    pub icon: Option<String>,
    pub readme_url: Option<String>,
    pub repository: String,
    pub wasm_url: String,
    pub sha256: String,
    pub signature: Option<String>,
    pub permissions: Vec<String>,
}

pub struct ServerlessMarketplaceClient {
    client: reqwest::Client,
}

impl ServerlessMarketplaceClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("AlouetteStudio-Engine/1.0")
                .build()
                .unwrap_or_default(),
        }
    }

    /// Kiểm tra tính hợp lệ của chữ ký số Ed25519 cho file Wasm
    pub fn verify_ed25519_signature(
        public_key_hex: &str,
        signature_hex: &str,
        data_bytes: &[u8],
    ) -> Result<bool> {
        let pub_key_bytes = hex::decode(public_key_hex.trim_start_matches("ed25519:"))
            .context("Invalid public key hex format")?;
        let sig_bytes = hex::decode(signature_hex.trim_start_matches("ed25519_sig:"))
            .context("Invalid signature hex format")?;

        let pub_key_array: [u8; 32] = pub_key_bytes.try_into()
            .map_err(|_| anyhow!("Public key must be 32 bytes"))?;
        let sig_array: [u8; 64] = sig_bytes.try_into()
            .map_err(|_| anyhow!("Signature must be 64 bytes"))?;

        let verifying_key = VerifyingKey::from_bytes(&pub_key_array)
            .map_err(|e| anyhow!("Failed to parse Ed25519 public key: {}", e))?;
        let signature = Signature::from_bytes(&sig_array);

        Ok(verifying_key.verify(data_bytes, &signature).is_ok())
    }

    /// Tải danh mục Extension từ GitHub CDN (jsDelivr Serverless Registry)
    pub async fn fetch_registry_index(&self) -> Result<Vec<RegistryIndexItem>> {
        let resp = self
            .client
            .get(SERVERLESS_REGISTRY_CDN)
            .send()
            .await
            .context("Failed to fetch extension registry index from CDN")?;

        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch CDN index, HTTP status: {}", resp.status()));
        }

        let items: Vec<RegistryIndexItem> = resp.json().await.context("Failed to parse registry index JSON")?;
        Ok(items)
    }

    /// Tải file .wasm trực tiếp từ GitHub Release của Dev và kiểm tra SHA-256 + Ed25519 Signature
    pub async fn download_and_install_wasm(
        &self,
        item: &RegistryIndexItem,
        target_dir: &Path,
    ) -> Result<()> {
        let ext_dir = target_dir.join(&item.id);
        fs::create_dir_all(&ext_dir)?;

        let resp = self.client.get(&item.wasm_url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Failed to download Wasm binary from: {}", item.wasm_url));
        }

        let bytes = resp.bytes().await?;

        // 1. Kiểm tra SHA-256 checksum
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash_result = format!("{:x}", hasher.finalize());

        if !hash_result.eq_ignore_ascii_case(&item.sha256) {
            return Err(anyhow!(
                "SHA-256 mismatch! Expected: {}, Computed: {}",
                item.sha256,
                hash_result
            ));
        }

        // 2. Xác minh chữ ký số Ed25519 (nếu Publisher đã đăng ký Public Key)
        if let (Some(pub_info), Some(sig_hex)) = (&item.publisher, &item.signature) {
            if let Some(pub_key_hex) = &pub_info.public_key {
                let is_sig_valid = Self::verify_ed25519_signature(pub_key_hex, sig_hex, &bytes)?;
                if !is_sig_valid {
                    return Err(anyhow!(
                        "CRITICAL SECURITY ERROR: Ed25519 Digital Signature Verification Failed for extension '{}'! Possible malicious tamper detected.",
                        item.id
                    ));
                }
            }
        }

        // 3. Lưu plugin.wasm
        let wasm_path = ext_dir.join("plugin.wasm");
        fs::write(&wasm_path, &bytes)?;

        // 4. Tạo file proto-extension.json local
        let manifest = ExtensionManifest {
            id: item.id.clone(),
            name: item.name.clone(),
            version: item.version.clone(),
            description: item.description.clone(),
            author: item.author.clone(),
            publisher: item.publisher.clone(),
            repository: Some(item.repository.clone()),
            icon: item.icon.clone(),
            readme_url: item.readme_url.clone(),
            sha256: Some(item.sha256.clone()),
            signature: item.signature.clone(),
            runtime: super::manifest::ExtensionRuntime {
                r#type: "wasm".to_string(),
                entry: "plugin.wasm".to_string(),
                wasm_entry: Some("plugin.wasm".to_string()),
            },
            capabilities: Some(super::manifest::ExtensionCapabilities {
                permissions: item.permissions.clone(),
            }),
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        fs::write(ext_dir.join("proto-extension.json"), manifest_json)?;

        Ok(())
    }

    /// Tính mã checksum SHA-256 của file local
    pub fn calculate_file_sha256(file_path: &Path) -> Result<String> {
        let bytes = fs::read(file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }
}

pub fn zip_extension_dir(src_dir: &Path, dst_file: &Path) -> Result<()> {
    let file = File::create(dst_file)?;
    let mut zip = zip::ZipWriter::new(file);
    let prefix = src_dir;
    let mut buffer = Vec::new();

    fn add_dir_to_zip(
        zip: &mut zip::ZipWriter<File>,
        dir: &Path,
        prefix: &Path,
        buffer: &mut Vec<u8>,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();

            if name == ".git" || name == "node_modules" || name == "target" || name.starts_with(".") {
                continue;
            }

            let name_in_zip = path.strip_prefix(prefix)?.to_str().unwrap().replace("\\", "/");
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);

            if path.is_dir() {
                zip.add_directory(&name_in_zip, options)?;
                add_dir_to_zip(zip, &path, prefix, buffer)?;
            } else {
                zip.start_file(&name_in_zip, options)?;
                let mut f = File::open(&path)?;
                buffer.clear();
                f.read_to_end(buffer)?;
                zip.write_all(buffer)?;
            }
        }
        Ok(())
    }

    add_dir_to_zip(&mut zip, src_dir, prefix, &mut buffer)?;
    zip.finish()?;
    Ok(())
}
