use super::error::*;
use futures::StreamExt;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use tokio_serial::{SerialPortBuilderExt, SerialPortInfo, SerialPortType, SerialStream};
use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::{error, info};

pub struct Crow(SerialStream);

impl Crow {
    pub fn new() -> Result<Self> {
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

                Ok(Crow(port))
            }
            None => Err(Error::NotFound),
        }
    }

    pub fn split(self) -> (CrowReader, CrowWriter) {
        let (reader, writer) = tokio::io::split(self.0);
        let reader = FramedRead::new(reader, LinesCodec::new());

        (CrowReader(reader), CrowWriter(writer))
    }

    pub async fn write_delimited(&mut self, chunk: &str) -> Result<()> {
        write_delimited(&mut self.0, chunk.as_bytes()).await
    }

    pub async fn write_script(&mut self, chunk: &str) -> Result<()> {
        write_script(&mut self.0, chunk.as_bytes()).await
    }

    pub async fn write_all(&mut self, chunk: &str) -> Result<()> {
        write_all(&mut self.0, chunk.as_bytes()).await
    }

    pub async fn read_line(&mut self) -> Result<String> {
        read_line(&mut self.0).await
    }

    pub async fn try_read_line(&mut self) -> Result<Option<String>> {
        read_line_if_available(&mut self.0).await
    }
}

pub struct CrowWriter(WriteHalf<SerialStream>);

impl CrowWriter {
    pub async fn write_delimited(&mut self, chunk: &str) -> Result<()> {
        write_delimited(&mut self.0, chunk.as_bytes()).await
    }

    pub async fn write_script(&mut self, chunk: &str) -> Result<()> {
        write_script(&mut self.0, chunk.as_bytes()).await
    }

    pub async fn write_all(&mut self, chunk: &str) -> Result<()> {
        write_all(&mut self.0, chunk.as_bytes()).await
    }
}

pub struct CrowReader(FramedRead<ReadHalf<SerialStream>, LinesCodec>);

impl CrowReader {
    pub async fn run(mut self) {
        while let Some(reply) = self.0.next().await {
            match reply {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    println!("Crow couldn't find the words");
                    error!("Read error: {e:?}");
                }
            }
        }
    }

    pub async fn read_once(&mut self) -> Result<String> {
        Ok(self.0.next().await.ok_or(Error::ConnectionClosed)??)
    }
}

// General read/write ops w/ crow's protocol

pub async fn write_all<W>(writer: &mut W, chunk: &[u8]) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    info!("Writing bytes: {:?}", String::from_utf8_lossy(chunk));

    writer.write_all(chunk).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}

pub async fn write_script<W>(writer: &mut W, script: &[u8]) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    info!("Writing script: {:?}", &script[..256]);

    writer.write_all(b"^^s").await?;
    writer.write_all(script).await?;
    writer.write_all(b"^^e").await?;
    writer.write_all(b"\n").await?;

    Ok(())
}

pub async fn write_delimited<W>(writer: &mut W, chunk: &[u8]) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    info!("Writing chunk of text w/ len > 64b");

    writer.write_all(b"```").await?;
    writer.write_all(chunk).await?;
    writer.write_all(b"```").await?;
    writer.write_all(b"\n").await?;

    Ok(())
}

pub async fn read_line<R>(reader: &mut R) -> Result<String>
where
    R: AsyncRead + Unpin,
{
    // I never get to write any fun low-level bullshit bc tokio already has it >:(
    let mut bufreader = BufReader::new(reader);

    let mut buf = String::with_capacity(512);
    bufreader.read_line(&mut buf).await?;

    // shrink her
    buf.shrink_to_fit();

    Ok(buf)
}

pub async fn read_line_if_available<R>(reader: &mut R) -> Result<Option<String>>
where
    R: AsyncRead + Unpin,
{
    let mut bufreader = BufReader::new(reader);

    let mut buf = String::with_capacity(512);

    if let Ok(read) =
        tokio::time::timeout(Duration::from_millis(10), bufreader.read_line(&mut buf)).await
    {
        let _ = read?;
        buf.shrink_to_fit();
        Ok(Some(buf))
    } else {
        Ok(None)
    }
}
