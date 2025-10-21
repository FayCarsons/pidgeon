use crate::crow::CrowWriter;

use super::error::*;
use axum::{
    Router,
    body::Body,
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::Json,
    routing::get,
};
use futures::lock::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tracing::{error, info};

#[derive(Clone)]
struct AppState {
    writer: Arc<Mutex<CrowWriter>>,
    connected: Arc<AtomicBool>,
}

// GET /connect - websocket endpoint
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response<Body> {
    if state.connected.load(Ordering::SeqCst) {
        axum::response::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("BUSY"))
            .unwrap()
    } else {
        ws.on_upgrade(move |socket| handle_socket(socket, state))
    }
}

async fn handle_socket(mut socket: WebSocket, AppState { writer, .. }: AppState) {
    // Increment connection count
    info!("Creating websocket connection");

    let mut writer = writer.lock().await;

    while let Some(Ok(msg)) = socket.recv().await {
        if let axum::extract::ws::Message::Text(text) = msg {
            info!("Received command: {}", text);
            if let Err(e) = writer.write_delimited(text.as_bytes()).await {
                error!("Write error: {e:?}")
            }
        }
    }

    info!("Websocket connection closed");
}

// GET /check - status endpoint
async fn check_handler(State(state): State<AppState>) -> Json<&'static str> {
    Json(if state.connected.load(Ordering::SeqCst) {
        "BUSY"
    } else {
        "OK"
    })
}

// GET / - health check
async fn root_handler() -> &'static str {
    "pidgeon websocket server"
}

pub async fn start_websocket_server(tx: CrowWriter) -> Result<()> {
    let state = AppState {
        connected: Arc::new(AtomicBool::new(false)),
        writer: Arc::new(Mutex::new(tx)),
    };

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/connect", get(websocket_handler))
        .route("/check", get(check_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:6666").await?;

    info!("Websocket server listening on http://127.0.0.1:6666");

    axum::serve(listener, app).await?;

    Ok(())
}
