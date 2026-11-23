// SPDX-License-Identifier: AGPL-3.0-or-later
//! One policy for every property suite, without process environment mutation,
//! and the value and near-miss source generators that need `proptest` alone.

use std::any::Any;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use proptest::strategy::Strategy;
use proptest::test_runner::{
    Config, FailurePersistence, PersistedSeed, RngAlgorithm, RngSeed, TestCaseResult, TestError,
    TestRunner,
};

/// An invalid explicit case-count or RNG-seed override.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigOverrideError {
    /// Case counts must be nonzero unsigned 32-bit integers.
    Cases,
    /// Seeds must be unsigned 64-bit integers.
    Seed,
}

/// Regression storage containing only Proptest's replayable seed lines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeedPersistence {
    directory: PathBuf,
}

impl SeedPersistence {
    /// Store one regression file per source file in `directory`.
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    fn path(&self, source: Option<&str>) -> PathBuf {
        self.directory
            .join(
                Path::new(source.unwrap_or("properties.rs"))
                    .file_name()
                    .unwrap_or(std::ffi::OsStr::new("properties.rs")),
            )
            .with_extension("txt")
    }
}

impl FailurePersistence for SeedPersistence {
    fn load_persisted_failures2(&self, source: Option<&'static str>) -> Vec<PersistedSeed> {
        let path = self.path(source);
        match fs::read_to_string(&path) {
            Ok(text) => text
                .lines()
                .map(|line| {
                    line.parse()
                        .expect("regression files contain seed lines only")
                })
                .collect(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("cannot read {}: {error}", path.display()),
        }
    }

    fn save_persisted_failure2(
        &mut self,
        source: Option<&'static str>,
        seed: PersistedSeed,
        _value: &dyn fmt::Debug,
    ) {
        if self.load_persisted_failures2(source).contains(&seed) {
            return;
        }
        fs::create_dir_all(&self.directory).expect("create property regression directory");
        let path = self.path(source);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("open property regression file");
        writeln!(file, "{seed}").expect("persist property regression seed");
    }

    fn box_clone(&self) -> Box<dyn FailurePersistence> {
        Box::new(self.clone())
    }
    fn eq(&self, other: &dyn FailurePersistence) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The suite default, with pure, explicitly scoped overrides.
///
/// Overrides use the same decimal syntax as `PROPTEST_CASES` and
/// `PROPTEST_RNG_SEED`. No environment is changed. Shrinking has at most 256
/// iterations; the fixed seed selects ChaCha rather than Proptest's default RNG.
pub fn config_with_overrides(
    cases: u32,
    case_override: Option<&str>,
    seed_override: Option<&str>,
) -> Result<Config, ConfigOverrideError> {
    let cases = match case_override {
        Some(value) => value.parse().map_err(|_| ConfigOverrideError::Cases)?,
        None => cases,
    };
    if cases == 0 {
        return Err(ConfigOverrideError::Cases);
    }
    let seed = match seed_override {
        Some(value) => value.parse().map_err(|_| ConfigOverrideError::Seed)?,
        None => 0x4653_4c20_5052_4f50,
    };
    Ok(Config {
        cases,
        rng_seed: RngSeed::Fixed(seed),
        rng_algorithm: RngAlgorithm::ChaCha,
        max_shrink_iters: 256,
        max_shrink_time: 0,
        // Cargo runs each test binary in its owning package's directory.
        // Consumers persist beside their own suites, not this crate.
        failure_persistence: Some(Box::new(SeedPersistence::new("tests/proptest-regressions"))),
        ..Config::default()
    })
}

/// Configure a suite from its default and the two documented environment overrides.
///
/// # Panics
/// Panics when an explicitly supplied override has invalid syntax or zero cases.
pub fn config(cases: u32) -> Config {
    config_with_overrides(
        cases,
        std::env::var("PROPTEST_CASES").ok().as_deref(),
        std::env::var("PROPTEST_RNG_SEED").ok().as_deref(),
    )
    .expect("valid property count and seed overrides")
}

/// Execute a suite and print its actual elapsed time exactly once, including failures.
pub fn run<S: Strategy>(
    suite: &'static str,
    mut config: Config,
    strategy: &S,
    property: impl Fn(S::Value) -> TestCaseResult,
) -> Result<(), TestError<S::Value>> {
    if config.source_file.is_none() {
        config.source_file = Some(suite);
    }
    let cases = config.cases;
    let start = Instant::now();
    let result = TestRunner::new(config).run(strategy, property);
    eprintln!(
        "property suite {suite}: {cases} cases, {:?}",
        start.elapsed()
    );
    result
}

pub mod near_miss;
pub mod value;
