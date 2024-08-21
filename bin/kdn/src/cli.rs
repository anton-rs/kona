//! Module for the CLI.

use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser};
use include_directory::{include_directory, Dir, DirEntry, File};
use op_test_vectors::derivation::DerivationFixture;
use tracing::{debug, error, info, trace, warn, Level};

static TEST_FIXTURES: Dir<'_> =
    include_directory!("$CARGO_MANIFEST_DIR/tests/fixtures/derivation/");

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
            .map(|f| {
                let path =
                    f.path().to_str().ok_or_else(|| anyhow!("Failed to convert path to string"))?;
                let fixture_str =
                    f.contents_utf8().ok_or_else(|| anyhow!("Failed to read file contents"))?;
                debug!("Parsing test fixture: {}", path);
                Ok((
                    path.to_string(),
                    serde_json::from_str::<DerivationFixture>(fixture_str)
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

    /// Get a list of available tests in the `tests` git submodule.
    pub fn get_tests() -> Result<Vec<File<'static>>> {
        let mut tests = Vec::with_capacity(TEST_FIXTURES.entries().len());
        for path in TEST_FIXTURES.entries() {
            let DirEntry::File(f) = path else {
                debug!(target: "get_tests", "Skipping non-file: {:?}", path);
                continue;
            };
            tests.push(f.clone());
        }
        Ok(tests)
    }
}
