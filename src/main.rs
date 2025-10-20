use std::{path::PathBuf, time::Duration};

use clap::*;
use futures::StreamExt;
use rustyline::error::ReadlineError;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf, split},
    time::{error::Elapsed, timeout},
};
use tokio_serial::{SerialPortBuilderExt, SerialPortInfo, SerialPortType, SerialStream};
use tokio_util::codec::{Decoder, Framed, FramedRead, LinesCodec};
use tracing::{Level, error, info, span};

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

#[derive(Debug, Error)]
pub enum Error {
    #[error("Serial Error: {0}")]
    Serial(#[from] tokio_serial::Error),
    #[error("Crow not found")]
    NotFound,
    #[error("IO error '{0}'")]
    IO(#[from] std::io::Error),
    #[error("Repl error '{0}'")]
    Repl(#[from] ReadlineError),
}

type Result<T> = std::result::Result<T, Error>;

/*
| Command | Purpose | Format |
|---------|---------|---------|
| `^^s` | Start script upload mode | `^^s` |
| `^^e` | Execute uploaded script | `^^e` |
| `^^w` | Write script to flash memory | `^^w` |
| `^^p` | Print current user script | `^^p` |
| `^^v` | Get firmware version | `^^v` |
| `^^b` | Enter bootloader mode | `^^b` |
* */

#[derive(Debug, Clone, Copy)]
enum Message {
    Upload,     // ^^s
    Execute,    // ^^e
    Write,      // ^^w
    Print,      // ^^p
    Version,    // ^^v
    Bootloader, // ^^b
}

impl Message {
    fn _parse(s: &[u8]) -> Option<Self> {
        use Message::*;

        match &s[..3] {
            b"^^s" => Some(Upload),
            b"^^e" => Some(Execute),
            b"^^w" => Some(Write),
            b"^^p" => Some(Print),
            b"^^v" => Some(Version),
            b"^^b" => Some(Bootloader),
            _ => None,
        }
    }

    fn as_bytes(&self) -> &[u8; 3] {
        use Message::*;

        match self {
            Upload => b"^^s",
            Execute => b"^^e",
            Write => b"^^b",
            Print => b"^^p",
            Version => b"^^v",
            Bootloader => b"^^b",
        }
    }
}

struct Crow {
    reader: FramedRead<ReadHalf<SerialStream>, LinesCodec>,
    writer: WriteHalf<SerialStream>,
}

impl Crow {
    fn new() -> Result<Self> {
        let ports = tokio_serial::available_ports()?;

        info!("Ports: {:?}", &ports);

        let crow = ports.iter().find_map(|port| match port {
            SerialPortInfo {
                port_name,
                port_type: SerialPortType::UsbPort(info),
            } if info
                .product
                .as_ref()
                .is_some_and(|s| s == "crow: telephone line") =>
            {
                Some(port_name)
            }
            _ => None,
        });

        match crow {
            Some(path) => {
                info!("Found crow: {}", path);

                let port = tokio_serial::new(path, 115_200).open_native_async()?;
                let (reader, writer) = tokio::io::split(port);
                let reader = FramedRead::new(reader, LinesCodec::new());

                Ok(Crow { writer, reader })
            }
            None => Err(Error::NotFound),
        }
    }

    async fn write_message(&mut self, message: Message) -> Result<()> {
        info!("Writing message: {message:?}");

        self.writer.write_all(message.as_bytes().as_slice()).await?;
        self.writer.write_all(b"\n").await?;
        Ok(())
    }

    async fn write_all(&mut self, chunk: &[u8]) -> Result<()> {
        info!("Writing bytes: {:?}", String::from_utf8_lossy(chunk));

        self.writer.write_all(chunk).await?;
        self.writer.write_all(b"\n").await?;
        Ok(())
    }

    async fn write_script(&mut self, script: &str) -> Result<()> {
        info!("Writing script: {:?}", &script[..256]);

        self.write_message(Message::Upload).await?;
        self.writer.write_all(script.as_bytes()).await?;
        self.write_message(Message::Execute).await?;
        self.writer.write_all(b"\n").await?;

        Ok(())
    }

    async fn write_oversize_chunk(&mut self, chunk: &[u8]) -> Result<()> {
        info!("Writing chunk of text w/ len > 64b");

        self.writer.write_all(b"```").await?;
        self.writer.write_all(chunk).await?;
        self.writer.write_all(b"```").await?;
        self.writer.write_all(b"\n").await?;

        Ok(())
    }

    async fn try_read(&mut self) -> Option<String> {
        match timeout(Duration::from_millis(500), self.reader.next()).await {
            Ok(Some(Ok(read))) => {
                info!("Read {read} bytes");

                Some(read)
            }
            Ok(Some(Err(e))) => {
                error!("Crow stumbled over her words: {e:?}");

                None
            }
            Ok(None) => None,
            Err(_) => {
                error!("Crow failed to speak fast enough");

                None
            }
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    app(Cli::parse().command)
        .await
        .map_err(std::io::Error::other)
}

async fn app(command: Commands) -> Result<()> {
    let mut crow = Crow::new()?;

    if let Some(greeting) = crow.try_read().await {
        println!("{greeting}")
    }

    match command {
        Commands::File { path } => {
            let contents = std::fs::read_to_string(path)?;

            // because the crow reads in 64byte chunks by default, when uploadin an entire file we
            // should tell it that it is instead receiving more than that
            crow.write_script(contents.as_str()).await?;

            Ok(())
        }
        Commands::Repl => {
            let mut rl = rustyline::DefaultEditor::new()?;

            loop {
                let line = rl.readline(">> ")?;
                info!("Got line: {line}");

                if line.as_str() == "exit" {
                    break Ok(());
                }

                if line.len() > 64 {
                    crow.write_oversize_chunk(line.as_bytes()).await?;
                } else {
                    crow.write_all(line.as_bytes()).await?;
                }

                if let Some(reply) = crow.try_read().await {
                    println!("{reply}")
                }
            }
        }
        Commands::Remote => {
            todo!()
        }
    }
}
