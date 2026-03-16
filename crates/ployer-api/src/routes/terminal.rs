use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    response::Response,
};
use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecOptions, StartExecResults};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tracing::warn;

use crate::app_state::SharedState;
use crate::auth::validate_token;
use ployer_db::repositories::DeploymentRepository;

#[derive(Deserialize)]
pub struct TerminalQuery {
    token: String,
}

#[derive(Deserialize)]
struct ResizeMsg {
    cols: u16,
    rows: u16,
}

pub async fn terminal_ws_handler(
    ws: WebSocketUpgrade,
    Path(app_id): Path<String>,
    Query(query): Query<TerminalQuery>,
    State(state): State<SharedState>,
) -> Response {
    if validate_token(&query.token, &state.config.auth.jwt_secret).is_err() {
        return ws.on_upgrade(|mut socket| async move {
            let _ = socket
                .send(Message::Text(
                    "\r\nAuthentication failed.\r\n".to_string(),
                ))
                .await;
            let _ = socket.close().await;
        });
    }

    ws.on_upgrade(move |socket| handle_terminal(socket, app_id, state))
}

async fn handle_terminal(socket: WebSocket, app_id: String, state: SharedState) {
    let docker = match &state.docker {
        Some(d) => d.clone(),
        None => {
            let mut s = socket;
            let _ = s.send(Message::Text("\r\nDocker not available.\r\n".to_string())).await;
            return;
        }
    };

    // Find the running container for this app
    let deployment_repo = DeploymentRepository::new(state.db.clone());
    let container_id = match deployment_repo.get_latest_running(&app_id).await {
        Ok(Some(dep)) => match dep.container_id {
            Some(id) => id,
            None => {
                let mut s = socket;
                let _ = s.send(Message::Text("\r\nNo running container found.\r\n".to_string())).await;
                return;
            }
        },
        _ => {
            let mut s = socket;
            let _ = s.send(Message::Text("\r\nApp is not running.\r\n".to_string())).await;
            return;
        }
    };

    // Try bash, fall back to sh if bash isn't in the image
    let start_result = match try_exec(docker.inner(), &container_id, "/bin/bash").await {
        Ok(r) => r,
        Err(_) => match try_exec(docker.inner(), &container_id, "/bin/sh").await {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to start exec for {}: {}", app_id, e);
                let mut s = socket;
                let _ = s.send(Message::Text(format!("\r\nFailed to start terminal: {}\r\n", e))).await;
                return;
            }
        }
    };

    let (exec_output, exec_input) = match start_result {
        StartExecResults::Attached { output, input } => (output, input),
        StartExecResults::Detached => return,
    };

    let (mut ws_tx, mut ws_rx) = socket.split();

    // exec stdout/stderr → WebSocket
    let mut output_task = tokio::spawn(async move {
        let mut output = exec_output;
        while let Some(Ok(log)) = output.next().await {
            let bytes: Vec<u8> = match log {
                bollard::container::LogOutput::StdOut { message } => message.to_vec(),
                bollard::container::LogOutput::StdErr { message } => message.to_vec(),
                bollard::container::LogOutput::Console { message } => message.to_vec(),
                _ => continue,
            };
            if ws_tx.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
    });

    // WebSocket → exec stdin + handle resize
    let docker_inner = docker.inner().clone();
    let exec_id_clone = exec_id.clone();
    let mut input_task = tokio::spawn(async move {
        let mut stdin = exec_input;
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Binary(data) => {
                    if stdin.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Message::Text(text) => {
                    // Resize event: {"cols":80,"rows":24}
                    if let Ok(r) = serde_json::from_str::<ResizeMsg>(&text) {
                        let _ = docker_inner
                            .resize_exec(
                                &exec_id_clone,
                                ResizeExecOptions {
                                    height: r.rows,
                                    width: r.cols,
                                },
                            )
                            .await;
                    } else {
                        // Plain text input
                        if stdin.write_all(text.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut output_task => input_task.abort(),
        _ = &mut input_task => output_task.abort(),
    }
}

/// Create and start an exec session for `shell` with full TTY attached.
/// Returns an error if the shell binary doesn't exist in the container.
async fn try_exec(
    docker: &bollard::Docker,
    container_id: &str,
    shell: &str,
) -> anyhow::Result<StartExecResults> {
    let exec = docker
        .create_exec(
            container_id,
            CreateExecOptions {
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                tty: Some(true),
                cmd: Some(vec![shell]),
                ..Default::default()
            },
        )
        .await?;

    let result = docker
        .start_exec(
            &exec.id,
            Some(StartExecOptions {
                detach: false,
                tty: true,
                ..Default::default()
            }),
        )
        .await?;

    Ok(result)
}
