use iced::futures::SinkExt;
use iced::Subscription;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::oneshot;
use zeroize::Zeroize;

const SOCKET_ENV_VAR: &str = "RHINCO_TV_ASKPASS_SOCKET";
const SOCKET_FILENAME: &str = "rhinco-tv-askpass.sock";

const ASKPASS_SCRIPT: &str = r#"#!/bin/sh
SOCKET_PATH="${RHINCO_TV_ASKPASS_SOCKET:-/run/user/$(id -u)/rhinco-tv-askpass.sock}"
PROMPT="${1:-Password: }"

PYTHON_BIN=$(command -v python3 || command -v python)
if [ -z "$PYTHON_BIN" ]; then
  exit 1
fi

exec "$PYTHON_BIN" - "$SOCKET_PATH" "$PROMPT" <<'PY'
import json
import socket
import sys

socket_path = sys.argv[1]
prompt = sys.argv[2] if len(sys.argv) > 2 else "Password:"

payload = json.dumps({"prompt": prompt}).encode("utf-8")

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
try:
    sock.connect(socket_path)
    sock.sendall(payload)
    sock.shutdown(socket.SHUT_WR)
    data = sock.recv(65536)
finally:
    try:
        sock.close()
    except Exception:
        pass

try:
    response = json.loads(data.decode("utf-8") or "{}")
except Exception:
    sys.exit(1)

if response.get("cancelled"):
    sys.exit(1)

password = response.get("password") or ""
sys.stdout.write(password)
PY
"#;

#[derive(Debug, Clone)]
pub enum AskpassEvent {
    PasswordRequest {
        prompt: String,
        responder: Arc<Mutex<Option<oneshot::Sender<Option<String>>>>>,
    },
}

#[derive(Debug, Deserialize)]
struct AskpassRequest {
    prompt: String,
}

#[derive(Debug, Serialize)]
struct AskpassResponse {
    password: Option<String>,
    cancelled: bool,
}

pub struct AskpassServer {
    socket_path: PathBuf,
    listener: UnixListener,
}

impl AskpassServer {
    pub fn bind() -> io::Result<Self> {
        let socket_path = get_socket_path();

        if let Some(parent) = socket_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if socket_path.exists() {
            let _ = fs::remove_file(&socket_path);
        }

        let listener = UnixListener::bind(&socket_path)?;
        fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))?;

        Ok(Self {
            socket_path,
            listener,
        })
    }

    async fn accept(&self) -> io::Result<(UnixStream, tokio::net::unix::SocketAddr)> {
        self.listener.accept().await
    }
}

impl Drop for AskpassServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

pub fn askpass_subscription() -> Subscription<AskpassEvent> {
    Subscription::run(|| {
        iced::stream::channel(
            32,
            |output: iced::futures::channel::mpsc::Sender<AskpassEvent>| async move {
                use tracing::warn;

                let server = match AskpassServer::bind() {
                    Ok(server) => {
                        tracing::info!(path = %server.socket_path.display(), "Askpass socket bound");
                        server
                    }
                    Err(err) => {
                        warn!(?err, "Failed to bind askpass socket");
                        return;
                    }
                };

                loop {
                    tracing::debug!("Waiting for askpass connection...");
                    let (stream, _addr) = match server.accept().await {
                        Ok(result) => result,
                        Err(err) => {
                            warn!(?err, "Askpass socket accept failed");
                            break;
                        }
                    };

                    tracing::info!("Askpass connection received");
                    let mut output_clone = output.clone();
                    if let Err(err) = handle_connection(stream, &mut output_clone).await {
                        warn!(?err, "Askpass socket handler failed");
                    }
                }
            },
        )
    })
}

pub fn get_askpass_script_path() -> io::Result<PathBuf> {
    let runtime_dir = runtime_dir();
    let script_name = format!("rhinco-tv-askpass-{}.sh", uuid::Uuid::new_v4());
    let script_path = runtime_dir.join(script_name);

    if let Some(parent) = script_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_script(&script_path)?;
    Ok(script_path)
}

pub fn get_socket_path() -> PathBuf {
    if let Some(path) = std::env::var_os(SOCKET_ENV_VAR) {
        return PathBuf::from(path);
    }

    runtime_dir().join(SOCKET_FILENAME)
}

fn runtime_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir);
    }

    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/run/user/{}", uid))
}

fn write_script(path: &Path) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    use std::io::Write;
    file.write_all(ASKPASS_SCRIPT.as_bytes())?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

async fn handle_connection(
    mut stream: UnixStream,
    output: &mut iced::futures::channel::mpsc::Sender<AskpassEvent>,
) -> io::Result<()> {
    tracing::debug!("Reading askpass request...");
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).await?;
    tracing::info!(bytes = buffer.len(), content = %String::from_utf8_lossy(&buffer), "Received askpass request");

    let request: AskpassRequest = match serde_json::from_slice::<AskpassRequest>(&buffer) {
        Ok(request) => {
            tracing::info!(prompt = %request.prompt, "Parsed askpass request");
            request
        }
        Err(err) => {
            tracing::warn!(?err, "Failed to parse askpass request");
            send_response(
                &mut stream,
                AskpassResponse {
                    password: None,
                    cancelled: true,
                },
            )
            .await?;
            return Ok(());
        }
    };

    let (sender, receiver) = oneshot::channel();
    let responder = Arc::new(Mutex::new(Some(sender)));
    if output
        .send(AskpassEvent::PasswordRequest {
            prompt: request.prompt,
            responder,
        })
        .await
        .is_err()
    {
        send_response(
            &mut stream,
            AskpassResponse {
                password: None,
                cancelled: true,
            },
        )
        .await?;
        return Ok(());
    }

    let response = match receiver.await {
        Ok(Some(password)) => AskpassResponse {
            password: Some(password),
            cancelled: false,
        },
        _ => AskpassResponse {
            password: None,
            cancelled: true,
        },
    };

    send_response(&mut stream, response).await?;
    Ok(())
}

async fn send_response(stream: &mut UnixStream, mut response: AskpassResponse) -> io::Result<()> {
    let payload = serde_json::to_vec(&response).map_err(io::Error::other)?;
    stream.write_all(&payload).await?;
    if let Some(ref mut password) = response.password {
        password.zeroize();
    }
    Ok(())
}
