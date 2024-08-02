use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    kd8n::Cli::parse().init_telemetry()?.run().await
}
