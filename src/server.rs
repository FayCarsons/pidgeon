use crate::crow::Crow;

use super::error::*;
use axum::{
    Router,
    body::Body,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::Json,
    routing::get,
};
use futures::{SinkExt, lock::Mutex};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Write,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tracing::{error, info};

#[derive(Clone)]
struct AppState {
    crow: Arc<Mutex<Crow>>,
    connected: Arc<AtomicBool>,
}

// We do *not* allocate in this household
#[derive(Clone, Debug, Serialize, Deserialize)]
enum PidgeonResponse<'a> {
    Caw,
    Song { content: &'a str },
    Grumble { reason: &'a str },
}
use PidgeonResponse::*;

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

async fn crow_communication(crow: &mut Crow, chunk: &str) -> Result<Option<String>> {
    crow.write_delimited(chunk).await?;
    crow.try_read_line().await
}

async fn handle_socket(mut socket: WebSocket, AppState { crow, .. }: AppState) {
    // Increment connection count
    info!("Creating websocket connection");

    let mut crow = crow.lock().await;
    let mut error_buffer = String::with_capacity(512);
    let mut got_error = false;

    loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => {
                info!("Received command: {}", text);

                let temp = crow_communication(&mut crow, &text).await;
                let response = match temp {
                    Ok(None) => Caw,
                    Ok(Some(ref content)) => Song { content },
                    Err(e) => {
                        error_buffer
                            .write_fmt(format_args!("{e}"))
                            .expect("Should not happen");
                        let reason = error_buffer.as_str();
                        got_error = true;

                        Grumble { reason }
                    }
                };

                let json = serde_json::to_string(&response).expect("Serialization should not fail");

                if let Err(e) = socket.send(Message::Text(json.into())).await {
                    error!("Websocket write failed: {e}");

                    socket
                        .close()
                        .await
                        .expect("Websocket connection should close");

                    break;
                }

                // the things I do for love...
                if got_error {
                    error_buffer.clear();

                    // no leak pls
                    if error_buffer.len() >= 1024 {
                        error_buffer.shrink_to(256);
                    }

                    got_error = false;
                }
            }

            Some(Err(e)) => {
                error!("Socket read error: {e}");
                socket
                    .close()
                    .await
                    .expect("Wow this really isn't going well..");

                break;
            }
            _ => continue,
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

pub async fn start_websocket_server(crow: Crow) -> Result<()> {
    let state = AppState {
        connected: Arc::new(AtomicBool::new(false)),
        crow: Arc::new(Mutex::new(crow)),
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
