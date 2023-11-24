use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser};
use tracing::Level;

/// A simple clap boilerplate
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0 [error] - 4 [trace]) - Default: 2 [error,warn,info]", action = ArgAction::Count)]
    v: u8,
}

fn main() -> Result<()> {
    // Parse the command arguments
    let Args { v } = Args::parse();

    // Initialize the tracing subscriber
    init_tracing_subscriber(v)?;

    tracing::error!(target: "example_cli", "Hello, debug!");
    tracing::warn!(target: "example_cli", "Hello, debug!");
    tracing::info!(target: "example_cli", "Hello, debug!");
    tracing::debug!(target: "example_cli", "Hello, debug!");
    tracing::trace!(target: "example_cli", "Hello, debug!");

    Ok(())
}

/// Initializes the tracing subscriber
///
/// # Arguments
/// * `verbosity_level` - The verbosity level (0-4)
///
/// # Returns
/// * `Result<()>` - Ok if successful, Err otherwise.
fn init_tracing_subscriber(verbosity_level: u8) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(match verbosity_level {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            _ => Level::TRACE,
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber).map_err(|e| anyhow!(e))
}
