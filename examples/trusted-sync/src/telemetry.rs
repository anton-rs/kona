use anyhow::{anyhow, Result};
use tracing::Level;

pub fn init(v: u8) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(match v {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            _ => Level::TRACE,
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber).map_err(|e| anyhow!(e))
}
