use anyhow::Result;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const API_BASE: &str = "http://localhost:8080/api/v1";

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateExtensionRequest {
    pub name: String,
    pub description: String,
    pub category: String,
    pub tech_stack: Vec<String>,
    pub visibility: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExtensionData {
    pub id: String,
    pub name: String,
    // Add other fields as needed based on API response
}

pub struct MarketplaceClient {
    pub client: reqwest::Client,
    pub token: Option<String>,
}

impl MarketplaceClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            token: None,
        }
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    pub async fn login(&mut self, email: &str, password: &str) -> Result<String> {
        let resp = self
            .client
            .post(format!("{}/auth/login", API_BASE))
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?
            .error_for_status()?;

        let data: AuthResponse = resp.json().await?;
        self.set_token(data.token.clone());
        Ok(data.token)
    }

    pub async fn fetch_extensions(&self) -> Result<Vec<ExtensionData>> {
        let resp = self
            .client
            .get(format!("{}/extensions", API_BASE))
            .send()
            .await?
            .error_for_status()?;

        // Assuming response is an array or has a data field
        // Adjust according to actual API response
        let data: Vec<ExtensionData> = resp.json().await?;
        Ok(data)
    }

    pub async fn create_extension(&self, req: &CreateExtensionRequest) -> Result<String> {
        let mut request = self.client.post(format!("{}/extensions", API_BASE));
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        let resp = request
            .json(req)
            .send()
            .await?
            .error_for_status()?;

        let json: serde_json::Value = resp.json().await?;
        Ok(json["id"].as_str().unwrap_or("").to_string())
    }

    pub async fn upload_release(
        &self,
        extension_id: &str,
        version: &str,
        changelog: &str,
        zip_path: &Path,
    ) -> Result<()> {
        let mut request = self
            .client
            .post(format!("{}/extensions/{}/releases", API_BASE, extension_id));

        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        let file_bytes = std::fs::read(zip_path)?;
        let part = multipart::Part::bytes(file_bytes)
            .file_name("release.zip")
            .mime_str("application/zip")?;

        let form = multipart::Form::new()
            .text("version", version.to_string())
            .text("changelog", changelog.to_string())
            .part("file", part);

        request.multipart(form).send().await?.error_for_status()?;
        Ok(())
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
        buffer: &mut Vec<u8>
    ) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();

            // Skip ignored directories
            if name == ".git" || name == "node_modules" || name == "__pycache__" || name == "target" || name.starts_with(".") {
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
