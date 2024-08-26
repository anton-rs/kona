//! Kona execution test runner

use crate::{cli::RunnerCfg, traits::TestExecutor};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use executor::OptimismExecutor;
use include_directory::{include_directory, Dir, DirEntry, File};
use op_test_vectors::execution::ExecutionFixture;
use tracing::{debug, info, trace, warn};

mod executor;
mod trie_fetcher;

static TEST_FIXTURES: Dir<'_> = include_directory!("$CARGO_MANIFEST_DIR/tests/fixtures/execution/");

/// The [ExecutionRunner] struct is a test executor for running execution tests.
pub(crate) struct ExecutionRunner {
    cfg: RunnerCfg,
}

impl ExecutionRunner {
    /// Create a new [DerivationRunner] instance.
    pub(crate) fn new(cfg: RunnerCfg) -> Self {
        Self { cfg }
    }
}

#[async_trait]
impl TestExecutor for ExecutionRunner {
    type Fixture = ExecutionFixture;

    async fn exec(&self) -> Result<()> {
        let fixtures = self.get_selected_fixtures()?;
        for (name, fixture) in fixtures {
            self.exec_single(name, fixture).await?;
        }
        Ok(())
    }

    async fn exec_single(&self, name: String, fixture: Self::Fixture) -> Result<()> {
        info!(target: "exec", "Running test: {}", name);
        let mut executor = OptimismExecutor::new(&fixture)?;
        executor.execute_checked()?;
        info!(target: "exec", "Test passed: {}", name);
        Ok(())
    }

    fn get_selected_fixtures(&self) -> Result<Vec<(String, Self::Fixture)>> {
        // Get available execution test fixtures
        let available_tests = Self::get_files()?;
        trace!("Available tests: {:?}", available_tests);

        // Parse execution test fixtures
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
                    serde_json::from_str::<ExecutionFixture>(fixture_str)
                        .map_err(|e| anyhow!(e))?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        trace!("Parsed {} tests", tests.len());

        // Select the tests to run.
        let fixtures = if self.cfg.all {
            tests
        } else if let Some(test) = &self.cfg.test {
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

    fn get_files() -> Result<Vec<File<'static>>> {
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
