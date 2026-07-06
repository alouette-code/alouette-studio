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
    pub name: String,
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
    let image_options = Some(CreateImageOptions {
        from_image: config.image.clone(),
        ..Default::default()
    });
    
    let mut stream = docker.create_image(image_options, None, None);
    while let Ok(Some(_)) = stream.try_next().await {}

    let options = Some(CreateContainerOptions {
        name: config.name.clone(),
        platform: None,
    });

    let host_config = HostConfig {
        binds: config.binds,
        port_bindings: config.port_bindings,
        memory: config.memory_bytes,
        nano_cpus: config.nano_cpus,
        ..Default::default()
    };

    let container_config = Config {
        image: Some(config.image),
        cmd: config.cmd,
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
