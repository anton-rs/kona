//! Contains the [TestExecutor] trait, which describes the interface for an
//! `ethereum-optimism/tests` test executor.

use anyhow::Result;
use async_trait::async_trait;
use include_directory::File;

/// The [TestExecutor] trait describes the interface for an `ethereum-optimism/tests` test executor.
#[async_trait]
pub(crate) trait TestExecutor {
    /// The type of test fixture to run.
    type Fixture;

    /// Executes all test fixtures.
    ///
    /// ## Returns
    /// - `Ok` - The test fixtures were executed successfully.
    /// - `Err` - An error occurred while executing the test fixtures.
    async fn exec(&self) -> Result<()>;

    /// Executes a given test fixture.
    ///
    /// ## Takes
    /// - `name` - The name of the test fixture.
    /// - `fixture` - The test fixture to run.
    ///
    /// ## Returns
    /// - `Ok` - The test fixture was executed successfully.
    /// - `Err` - An error occurred while executing the test fixture.
    async fn exec_single(&self, name: String, fixture: Self::Fixture) -> Result<()>;

    /// Retrieve the test fixtures to run based on the configuration.
    ///
    /// ## Returns
    /// - `Ok(Vec<(String, Self::Fixture)>)` - A vector of test fixture names and their
    ///   corresponding definitions.
    /// - `Err` - An error occurred while retrieving the test fixtures.
    fn get_selected_fixtures(&self) -> Result<Vec<(String, Self::Fixture)>>;

    /// Retrieve the test fixture files.
    ///
    /// ## Returns
    /// - `Ok(Vec<File>)` - A vector of test fixture files.
    /// - `Err` - An error occurred while retrieving the test fixture files.
    fn get_files() -> Result<Vec<File<'static>>>;
}
