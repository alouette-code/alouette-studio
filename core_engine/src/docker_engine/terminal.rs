use bollard::Docker;
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::io::AsyncWriteExt;

lazy_static::lazy_static! {
    static ref TERMINAL_INPUTS: Arc<Mutex<HashMap<String, mpsc::Sender<String>>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Serialize, Clone)]
pub struct TerminalOutput {
    pub id: String,
    pub exec_id: String,
    pub data: String,
}

pub async fn write_to_terminal(exec_id: &str, data: String) -> Result<(), String> {
    let mut inputs = TERMINAL_INPUTS.lock().await;
    if let Some(sender) = inputs.get_mut(exec_id) {
        let _ = sender.send(data).await;
        Ok(())
    } else {
        Err(format!("Terminal session {} not found", exec_id))
    }
}

pub async fn spawn_terminal<F>(
    docker: &Docker,
    container_id: &str,
    mut on_output: F,
) -> Result<String, String>
where
    F: FnMut(TerminalOutput) + Send + 'static,
{
    // Create exec
    let exec_options = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        attach_stdin: Some(true),
        tty: Some(true),
        cmd: Some(vec!["/bin/sh"]),
        ..Default::default()
    };

    let exec = docker
        .create_exec(container_id, exec_options)
        .await
        .map_err(|e| format!("Failed to create exec: {}", e))?;

    let exec_id = exec.id.clone();
    let container_id_clone = container_id.to_string();
    let exec_id_clone = exec_id.clone();
    
    // Start exec
    let res = docker
        .start_exec(
            &exec.id,
            Some(StartExecOptions {
                detach: false,
                tty: true,
                output_capacity: None,
            }),
        )
        .await
        .map_err(|e| format!("Failed to start exec: {}", e))?;

    if let StartExecResults::Attached { mut output, mut input } = res {
        let (tx, mut rx) = mpsc::channel::<String>(100);
        TERMINAL_INPUTS.lock().await.insert(exec_id.clone(), tx);

        // Spawn a task to handle input and output
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        if let Some(data) = msg {
                            if let Err(_) = input.write_all(data.as_bytes()).await {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    out = output.next() => {
                        if let Some(Ok(log_output)) = out {
                            on_output(TerminalOutput {
                                id: container_id_clone.clone(),
                                exec_id: exec_id_clone.clone(),
                                data: String::from_utf8_lossy(log_output.into_bytes().as_ref()).to_string(),
                            });
                        } else {
                            break;
                        }
                    }
                }
            }
            TERMINAL_INPUTS.lock().await.remove(&exec_id_clone);
        });

        Ok(exec_id)
    } else {
        Err("Failed to attach to exec".to_string())
    }
}
