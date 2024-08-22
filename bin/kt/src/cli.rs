//! Module for the CLI.

use crate::{dn::DerivationRunner, traits::TestExecutor};
use anyhow::{anyhow, Result};
use clap::{ArgAction, Args, Parser, Subcommand};
use tracing::Level;

/// Main CLI
#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Cli {
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0-4)", action = ArgAction::Count)]
    pub v: u8,
    /// The subcommand to run.
    #[clap(subcommand)]
    pub subcommand: KtSubcommand,
}

/// Subcommands for the CLI.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum KtSubcommand {
    /// Run derivation tests.
    Dn(RunnerCfg),
    /// Run execution tests.
    T8n(RunnerCfg),
}

/// Configuration for the test runner.
#[derive(Debug, Clone, Args)]
pub(crate) struct RunnerCfg {
    /// Specify a test to run.
    #[clap(long, short = 't', help = "Run specific test by name", conflicts_with = "all")]
    pub test: Option<String>,
    /// Runs all tests.
    #[clap(long, short = 'a', help = "Run all tests", conflicts_with = "test")]
    pub all: bool,
}

impl Cli {
    /// Initializes telemtry for the application.
    pub fn init_telemetry(self) -> Result<Self> {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(match self.v {
                0 => Level::ERROR,
                1 => Level::WARN,
                2 => Level::INFO,
                3 => Level::DEBUG,
                _ => Level::TRACE,
            })
            .finish();
        tracing::subscriber::set_global_default(subscriber).map_err(|e| anyhow!(e))?;
        Ok(self)
    }

    /// Parse the CLI arguments and run the command
    pub async fn run(&self) -> Result<()> {
        match &self.subcommand {
            KtSubcommand::Dn(cfg) => {
                let runner = DerivationRunner::new(cfg.clone());
                runner.exec().await
            }
            KtSubcommand::T8n(_cfg) => {
                unimplemented!("Execution test runner is not yet implemented.")
            }
        }
    }
}
