mod crow;
mod error;
mod repl;
mod server;

use clap::*;
use error::*;
use std::path::PathBuf;

use crate::crow::Crow;

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
    Remote,
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
    let crow = Crow::new()?;

    match command {
        File { path } => {
            let (mut reader, mut writer) = crow.split();

            let contents = std::fs::read_to_string(path)?;
            writer.write_script(contents.as_str()).await?;

            let response = reader.read_once().await?;
            println!("{response}");

            Ok(())
        }
        Repl => {
            let (reader, writer) = crow.split();
            let _reader_handle = tokio::spawn(reader.run());

            repl::run(writer).await
        }
        Remote => server::start_websocket_server(crow).await,
    }
}
