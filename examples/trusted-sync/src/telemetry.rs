use anyhow::{anyhow, Result};
use reqwest::Url;
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, prelude::*};

pub async fn init(v: u8, addr: Url) -> Result<()> {
    let (loki_layer, task) = tracing_loki::builder()
        .label("environment", "production")
        .map_err(|e| anyhow!(e))?
        .extra_field("pid", format!("{}", std::process::id()))
        .map_err(|e| anyhow!(e))?
        .build_url(addr)
        .map_err(|e| anyhow!(e))?;

    let std_layer = tracing_subscriber::fmt::Layer::default().with_writer(
        std::io::stdout.with_max_level(match v {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            _ => Level::TRACE,
        }),
    );
    let subscriber = tracing_subscriber::registry().with(loki_layer).with(std_layer);
    tracing::subscriber::set_global_default(subscriber).map_err(|e| anyhow!(e))?;
    tokio::spawn(task);
    Ok(())
}
