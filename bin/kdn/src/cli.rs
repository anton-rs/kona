//! Module for the CLI.

use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser};
use op_test_vectors::derivation::DerivationFixture;
use tracing::{debug, error, info, trace, warn, Level};

/// Main CLI
#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Specify a test to run.
    #[clap(long, help = "Run a specific derivation test fixture")]
    pub test: Option<String>,
    /// Runs all tests.
    #[clap(long, help = "Run all derivation tests", conflicts_with = "test")]
    pub all: bool,
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0-4)", action = ArgAction::Count)]
    pub v: u8,
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
        let fixtures = self.get_fixtures()?;
        for (name, fixture) in fixtures {
            self.exec(name, fixture).await?;
        }
        Ok(())
    }

    /// Executes a given derivation test fixture.
    pub async fn exec(&self, name: String, fixture: DerivationFixture) -> Result<()> {
        info!(target: "exec", "Running test: {}", name);
        let pipeline = crate::pipeline::new_runner_pipeline(fixture.clone()).await?;
        match crate::runner::run(pipeline, fixture).await {
            Ok(_) => {
                println!("[PASS] {}", name);
                Ok(())
            }
            Err(e) => {
                error!(target: "exec", "[FAIL] {}", name);
                Err(e)
            }
        }
    }

    /// Get [DerivationFixture]s to run.
    pub fn get_fixtures(&self) -> Result<Vec<(String, DerivationFixture)>> {
        // Get available derivation test fixtures
        let available_tests = Self::get_tests()?;
        trace!("Available tests: {:?}", available_tests);

        // Parse derivation test fixtures
        let tests = available_tests
            .iter()
            .map(|path| {
                let fixture_str = std::fs::read_to_string(path).map_err(|e| anyhow!(e))?;
                debug!("Parsing test fixture: {}", path);
                Ok((
                    path.to_string(),
                    serde_json::from_str::<DerivationFixture>(&fixture_str)
                        .map_err(|e| anyhow!(e))?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        trace!("Parsed {} tests", tests.len());

        // Select the tests to run.
        let fixtures = if self.all {
            tests
        } else if let Some(test) = &self.test {
            let fixture = tests
                .into_iter()
                .find(|(path, _)| {
                    std::path::Path::new(path)
                        .file_name()
                        .map(|f| {
                            f.to_str().unwrap().strip_suffix(".json").unwrap_or(f.to_str().unwrap())
                        })
                        .map(|f| f.ends_with(&test.strip_suffix(".json").unwrap_or(test)))
                        .unwrap_or(false)
                })
                .ok_or_else(|| anyhow!("Test not found"))?;
            vec![fixture]
        } else {
            warn!("No test specified, running all tests");
            tests
        };
        trace!("Selected {} tests", fixtures.len());

        Ok(fixtures)
    }

    /// Get a list of available tests in the `op-test-vectors` git submodule.
    pub fn get_tests() -> Result<Vec<String>> {
        let paths = std::fs::read_dir("../op-test-vectors/fixtures/derivation/").unwrap();
        let mut tests = Vec::with_capacity(paths.size_hint().0);
        for path in paths {
            let path = path.unwrap().path();
            tests.push(path.to_str().unwrap().to_string());
        }
        Ok(tests)
    }
}
