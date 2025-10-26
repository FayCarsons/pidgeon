use super::{crow::Crow, error::Result};
use futures::lock::Mutex;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
enum Message {
    Success {
        request_id: u64,
        contents: String,
    },
    Check,
    Start,
    Affirm,
    Failure {
        request_id: Option<u64>,
        contents: String,
    },
}
use Message::*;

const BUFSIZE: usize = 512 * 512;

struct Server {
    backing_buf: [u8; BUFSIZE],
    conn: TcpStream,
}

impl Server {
    async fn read_bytes(&mut self) -> Result<&[u8]> {
        let len = self.conn.read_u32().await?;
        info!("got len: {len}");

        self.conn
            .read_exact(&mut self.backing_buf[0..len as usize])
            .await?;
        info!("read {len} bytes successfully");

        Ok(&self.backing_buf[0..len as usize])
    }

    async fn write_bytes(&mut self, chunk: &[u8]) -> Result<()> {
        let len = chunk.len();
        debug_assert!(len < u32::MAX as usize);

        self.conn.write_u32(len as u32).await?;
        info!("wrote prefix {len}");
        self.conn.write_all(chunk).await?;
        info!("wrote {len} byte successfully");

        Ok(())
    }

    async fn read_message(&mut self) -> Result<Message> {
        let bytes = self.read_bytes().await?;
        Ok(serde_json::from_slice(bytes)?)
    }

    async fn write_message(&mut self, msg: Message) -> Result<()> {
        let bytes = serde_json::to_vec(&msg)?;
        self.write_bytes(&bytes).await
    }
}

const BACKLOG: u32 = 10;

fn make_conn(port: u16) -> Result<TcpListener> {
    let conn = TcpSocket::new_v4()?;
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    conn.set_reuseaddr(true)?;
    conn.bind(addr)?;

    Ok(conn.listen(BACKLOG)?)
}

async fn read_crow_response(crow: &mut Crow) -> Option<Result<String>> {
    tokio::time::timeout(Duration::from_millis(200), crow.read_line())
        .await
        .ok()
}

async fn handle_conn(server: &mut Server, crow: &mut Crow) -> Result<()> {
    loop {
        // Contents should be a Lua string
        match server.read_message().await? {
            // Content should be a valid lua string
            Success {
                request_id,
                contents,
            } => {
                info!("Got message from LUA client");

                async fn write_chunk(crow: &mut Crow, chunk: &str) -> Result<()> {
                    if chunk.len() >= 64 {
                        crow.write_delimited(chunk).await
                    } else {
                        crow.write_all(chunk).await
                    }
                }

                if let Err(err) = write_chunk(crow, &contents).await {
                    server
                        .write_message(Failure {
                            request_id: Some(request_id),
                            contents: format!("{err}"),
                        })
                        .await?;
                }

                if let Some(response) = read_crow_response(crow).await {
                    let response = match response {
                        Ok(crow_response) => Success {
                            request_id,
                            contents: crow_response,
                        },
                        Err(err) => Failure {
                            request_id: Some(request_id),
                            contents: err.to_string(),
                        },
                    };

                    server.write_message(response).await?;
                }
            }
            Failure { contents, .. } => error!("{contents}"),
            _ => {
                info!("Got nonsense from LUA client");
                server
                    .write_message(Failure {
                        request_id: None,
                        contents: "don't understand".into(),
                    })
                    .await?
            }
        }
    }
}

pub async fn run(crow: Crow, port: u16) -> Result<()> {
    info!("start server");
    let listener = make_conn(port).expect("Failed to create socket");
    info!("open socket");

    let crow = Arc::new(Mutex::new(crow));
    let busy = AtomicBool::new(false);

    loop {
        let (conn, addr) = listener.accept().await?;
        info!("Got connection on {addr:?}");

        let mut server = Server {
            backing_buf: [0; BUFSIZE],
            conn,
        };

        let read = server.conn.read_u32().await?;
        let _ = server
            .conn
            .read_exact(&mut server.backing_buf[0..read as usize])
            .await?;
        let message = serde_json::from_slice(&server.backing_buf[0..read as usize])?;

        match message {
            Start => {
                info!("Got START");
                if busy.load(Ordering::SeqCst) {
                    info!("BUSY - client rejected");
                    server
                        .write_message(Failure {
                            request_id: None,
                            contents: "BUSY".into(),
                        })
                        .await?;
                } else {
                    info!("crow FREE - connection opened");
                    let crow = crow.clone();

                    let _ = tokio::spawn(async move {
                        let mut crow = crow.lock().await;
                        handle_conn(&mut server, &mut crow)
                            .await
                            .expect("FAILED HANDLE CONN")
                    })
                    .await;
                }
            }

            Check => {
                if busy.load(Ordering::SeqCst) {
                    server
                        .write_message(Failure {
                            request_id: None,
                            contents: "BUSY".into(),
                        })
                        .await?
                } else {
                    server.write_message(Affirm).await?
                }
            }
            _ => {
                server
                    .write_message(Failure {
                        request_id: None,
                        contents: "don't understand".into(),
                    })
                    .await?
            }
        }
    }
}
