use super::error::*;
use futures::StreamExt;
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf};
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
}

pub struct CrowWriter(WriteHalf<SerialStream>);

impl CrowWriter {
    pub async fn write_all(&mut self, chunk: &[u8]) -> Result<()> {
        info!("Writing bytes: {:?}", String::from_utf8_lossy(chunk));

        self.0.write_all(chunk).await?;
        self.0.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn write_script(&mut self, script: &str) -> Result<()> {
        info!("Writing script: {:?}", &script[..256]);

        self.0.write_all(b"^^s").await?;
        self.0.write_all(script.as_bytes()).await?;
        self.0.write_all(b"^^e").await?;
        self.0.write_all(b"\n").await?;

        Ok(())
    }

    pub async fn write_delimited(&mut self, chunk: &[u8]) -> Result<()> {
        info!("Writing chunk of text w/ len > 64b");

        self.0.write_all(b"```").await?;
        self.0.write_all(chunk).await?;
        self.0.write_all(b"```").await?;
        self.0.write_all(b"\n").await?;

        Ok(())
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
}
