use std::{io::Write, path::PathBuf};

use clap::*;
use rustyline::error::ReadlineError;
use thiserror::Error;
use tokio_serial::{SerialPortBuilderExt, SerialPortInfo, SerialPortType, SerialStream};

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

enum Message {
    Upload,
    Execute,
    Write,
    Print,
    Version,
    Bootloader,
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
    conn: SerialStream,
}

impl Crow {
    fn new() -> Result<Self> {
        let ports = tokio_serial::available_ports()?;

        let crow = ports.iter().find_map(|port| match port {
            SerialPortInfo {
                port_name,
                port_type: SerialPortType::UsbPort(info),
            } if info.vid == 0x0483 && info.pid == 0x5740 => Some(port_name),
            _ => None,
        });

        match crow {
            Some(path) => Ok(Crow {
                conn: tokio_serial::new(path, 115_200).open_native_async()?,
            }),
            None => Err(Error::NotFound),
        }
    }

    fn write_prefix(&mut self, message: Message) -> Result<()> {
        Ok(self.conn.write_all(message.as_bytes().as_slice())?)
    }

    fn write_all(&mut self, chunk: &[u8]) -> Result<()> {
        Ok(self.conn.write_all(chunk)?)
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    app(Cli::parse().command).map_err(std::io::Error::other)
}

fn app(command: Commands) -> Result<()> {
    match command {
        Commands::File { path } => {
            let mut crow = Crow::new()?;

            let contents = std::fs::read_to_string(path)?;
            crow.write_prefix(Message::Upload)?;
            crow.write_all(contents.as_bytes())?;
            Ok(())
        }
        Commands::Repl => {
            let mut crow = Crow::new()?;

            let mut rl = rustyline::DefaultEditor::new()?;

            loop {
                let line = rl.readline(">> ")?;

                if line.as_str() == "exit" {
                    break Ok(());
                }

                crow.write_all(line.as_bytes())?;
            }
        }
        Commands::Remote => {
            todo!()
        }
    }
}
