mod crow;
mod error;
mod repl;
mod server;

use clap::*;
use error::*;
use std::path::PathBuf;
use tokio_serial::SerialStream;
use tracing::info;

use crate::crow::Crow;

pub const DEFAULT_PORT_STR: &str = "6666";
pub const DEFAULT_PORT: u16 = 6666;

#[derive(Debug, Parser)]
#[command(name = "pidgeon")]
#[command(about = "a terminal interface for the Monome Crow", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    File {
        path: PathBuf,
    },
    Repl,
    Remote {
        #[arg(default_value = DEFAULT_PORT_STR)]
        port: Option<u16>,
    },
    Simulate,
}
use Commands::*;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    app(Cli::parse().command)
        .await
        .map_err(std::io::Error::other)
}

async fn app(command: Commands) -> Result<()> {
    match command {
        File { path } => {
            let crow = Crow::new()?;
            let (mut reader, mut writer) = crow.split();

            let contents = std::fs::read_to_string(path)?;
            writer.write_script(contents.as_str()).await?;

            let response = reader.read_once().await?;
            println!("{response}");

            Ok(())
        }
        Repl => {
            let crow = Crow::new()?;
            let (reader, writer) = crow.split();
            let _reader_handle = tokio::spawn(reader.run());

            repl::run(writer).await
        }
        Remote { port } => server::run(Crow::new()?, port.unwrap_or(DEFAULT_PORT)).await,
        Simulate => {
            let (leader, mut follower) = SerialStream::pair()?;
            let crow = Crow::mock(leader);
            let handle = tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};

                let mut buf = Vec::with_capacity(1024);
                loop {
                    if AsyncReadExt::read(&mut follower, &mut buf).await.is_ok() {
                        info!("Mock crow got: '{}'", String::from_utf8_lossy(&buf));
                        follower
                            .write_all(b"OK")
                            .await
                            .expect("Failed to write dummy stream");
                    }
                }
            });

            server::run(crow, DEFAULT_PORT).await?;
            handle.abort();

            Ok(())
        }
    }
}
