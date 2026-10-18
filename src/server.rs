use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use nvml_wrapper::Nvml;
use rust_embed::Embed;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::monitor::{self, Snapshot};
use crate::Args;

#[derive(Embed)]
#[folder = "frontend/dist"]
struct Asset;

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
    /// Handle of the currently running sampler; replaced when a new viewer attaches.
    sampler_gen: Arc<Mutex<CancellationToken>>,
    nvml: Arc<Nvml>,
    interval: Duration,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    // Attachments start (or hand off to) the sampler task.
    spawn_sampler(&state);
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });
    // Receive side only exists to notice disconnects.
    while let Some(Ok(_)) = receiver.next().await {}
    send_task.abort();
}

async fn embed_assets(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => Html("<h1>frontend not built: run `npm run build` in frontend/</h1>")
            .into_response(),
    }
}

/// One sampler per viewer-generation: sleeps, samples, broadcasts.
/// Stops when a newer sampler supersedes it.
fn spawn_sampler(state: &AppState) {
    let nvml = state.nvml.clone();
    let tx = state.tx.clone();
    let interval = state.interval;
    let my_gen = CancellationToken::new();
    let prev = {
        let mut guard = state.sampler_gen.lock().unwrap();
        std::mem::replace(&mut *guard, my_gen.clone())
    };
    prev.cancel();

    tokio::spawn(async move {
        let mut errors: u64 = 0;
        let _ = prev;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {}
                _ = my_gen.cancelled() => return,
            }
            let gpus = monitor::sample(&nvml, &mut errors);
            let snap = Snapshot {
                ts: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                error_count: errors,
                gpus,
            };
            if let Ok(json) = serde_json::to_string(&snap) {
                // Err only when nobody is subscribed; keep looping cheaply.
                let _ = tx.send(json);
            }
        }
    });
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    // Fail fast with a clear message when NVML/driver is unavailable.
    let nvml = match Nvml::init() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("error: failed to initialize NVIDIA Management Library: {e}");
            eprintln!("hint: check that the NVIDIA driver is installed (/dev/nvidia*)");
            std::process::exit(1);
        }
    };
    if nvml.device_count().unwrap_or(0) == 0 {
        eprintln!("error: no NVIDIA GPUs detected via NVML");
        std::process::exit(1);
    }

    // Sampler starts already cancelled: it only runs while viewers exist.
    let state = AppState {
        tx: broadcast::channel::<String>(64).0,
        sampler_gen: Arc::new(Mutex::new(CancellationToken::new())),
        nvml: Arc::new(nvml),
        interval: Duration::from_millis(args.interval.max(250)),
    };
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback(get(embed_assets))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", args.port))
        .await
        .with_context(|| format!("failed to bind port {}", args.port))?;
    eprintln!("GPU monitor listening on http://0.0.0.0:{}", args.port);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
