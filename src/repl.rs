use tracing::info;

use super::{crow::CrowWriter, error::*};

pub async fn run(mut writer: CrowWriter) -> Result<()> {
    let mut rl = rustyline::DefaultEditor::new()?;

    loop {
        let line = rl.readline(">> ")?;
        info!("Got line: {line}");

        if line.as_str() == "exit" {
            break Ok(());
        }

        if line.len() > 64 {
            writer.write_delimited(line.as_bytes()).await?;
        } else {
            writer.write_all(line.as_bytes()).await?;
        }
    }
}
