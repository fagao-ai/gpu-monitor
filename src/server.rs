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
    window_sec: u64,
    /// Live viewer count + handle of the running sampler task.
    viewers: Arc<Mutex<Viewers>>,
    nvml: Arc<Nvml>,
    interval: Duration,
}

#[derive(Default)]
struct Viewers {
    count: usize,
    sampler: Option<CancellationToken>,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    // Reference-counted sampler: start on first viewer, stop on last.
    {
        let mut v = state.viewers.lock().unwrap();
        v.count += 1;
        if v.sampler.is_none() {
            v.sampler = Some(start_sampler(&state));
        }
    }

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

    let mut stop: Option<CancellationToken> = None;
    {
        let mut v = state.viewers.lock().unwrap();
        v.count = v.count.saturating_sub(1);
        if v.count == 0 {
            stop = v.sampler.take();
        }
    }
    if let Some(token) = stop {
        token.cancel();
    }
}

async fn embed_assets(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            Html("<h1>frontend not built: run `npm run build` in frontend/</h1>").into_response()
        }
    }
}

/// Starts the sampler task; caller stores its token and cancels it when the
/// last viewer leaves. Errors reset per session (fresh viewer -> fresh count).
fn start_sampler(state: &AppState) -> CancellationToken {
    let nvml = state.nvml.clone();
    let tx = state.tx.clone();
    let interval = state.interval;
    let window_sec2 = state.window_sec;
    let token = CancellationToken::new();
    let my_gen = token.clone();

    tokio::spawn(async move {
        let mut errors: u64 = 0;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {}
                _ = my_gen.cancelled() => return,
            }
            let gpus = monitor::sample(&nvml, &mut errors);
            let snap = Snapshot {
                window_sec: window_sec2,
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
    token
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

    let state = AppState {
        tx: broadcast::channel::<String>(64).0,
        window_sec: args.window,
        viewers: Arc::new(Mutex::new(Viewers::default())),
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
