use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::{ContainerSummary, HostConfig, PortBinding};
use bollard::Docker;
use futures_util::TryStreamExt;
use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DockerContainerConfig {
    #[serde(default)]
    pub name: Option<String>,
    pub image: String,
    pub cmd: Option<Vec<String>>,
    pub env: Option<Vec<String>>,
    pub port_bindings: Option<HashMap<String, Option<Vec<PortBinding>>>>,
    pub binds: Option<Vec<String>>,
    pub memory_bytes: Option<i64>,
    pub nano_cpus: Option<i64>,
}

pub async fn list_containers(
    docker: &Docker,
    all: bool,
) -> Result<Vec<ContainerSummary>, String> {
    let filters: HashMap<String, Vec<String>> = HashMap::new();
    let options = Some(ListContainersOptions {
        all,
        filters,
        ..Default::default()
    });
    
    docker
        .list_containers(options)
        .await
        .map_err(|e| format!("Failed to list containers: {}", e))
}

pub async fn create_container(
    docker: &Docker,
    config: DockerContainerConfig,
) -> Result<String, String> {
    let image = config.image.trim().to_string();
    if image.is_empty() {
        return Err("Docker image name cannot be empty.".to_string());
    }

    // Only pull if the image is not already available locally
    if docker.inspect_image(&image).await.is_err() {
        let image_options = Some(CreateImageOptions {
            from_image: image.clone(),
            ..Default::default()
        });

        let mut stream = docker.create_image(image_options, None, None);
        while let Some(info) = stream
            .try_next()
            .await
            .map_err(|e| format!("Failed to pull image '{}': {}", image, e))?
        {
            if let Some(err) = info.error {
                return Err(format!("Failed to pull image '{}': {}", image, err));
            }
        }
    }

    let container_name = config
        .name
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    let options = Some(CreateContainerOptions {
        name: container_name,
        platform: None,
    });

    let clean_cmd = config.cmd.and_then(|parts| {
        let filtered: Vec<String> = parts.into_iter().filter(|s| !s.is_empty()).collect();
        if filtered.is_empty() { None } else { Some(filtered) }
    });

    let host_config = HostConfig {
        binds: config.binds,
        port_bindings: config.port_bindings,
        memory: config.memory_bytes,
        nano_cpus: config.nano_cpus,
        ..Default::default()
    };

    let container_config = Config {
        image: Some(image),
        cmd: clean_cmd,
        env: config.env,
        host_config: Some(host_config),
        ..Default::default()
    };

    let result = docker
        .create_container(options, container_config)
        .await
        .map_err(|e| format!("Failed to create container: {}", e))?;

    Ok(result.id)
}

pub async fn start_container(docker: &Docker, id: &str) -> Result<(), String> {
    docker
        .start_container(id, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| format!("Failed to start container: {}", e))
}

pub async fn stop_container(docker: &Docker, id: &str) -> Result<(), String> {
    let options = Some(StopContainerOptions { t: 10 });
    docker
        .stop_container(id, options)
        .await
        .map_err(|e| format!("Failed to stop container: {}", e))
}

pub async fn remove_container(docker: &Docker, id: &str, force: bool) -> Result<(), String> {
    let options = Some(RemoveContainerOptions {
        v: true,
        force,
        link: false,
    });
    docker
        .remove_container(id, options)
        .await
        .map_err(|e| format!("Failed to remove container: {}", e))
}

pub async fn restart_container(docker: &Docker, id: &str) -> Result<(), String> {
    docker
        .restart_container(id, None)
        .await
        .map_err(|e| format!("Failed to restart container: {}", e))
}
