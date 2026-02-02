//! Test insertion utilities for the test runner.
//!
//! This module provides functions for converting test descriptors into
//! Bevy entities that can be run by the test runner.

use crate::prelude::*;
use crate::testing::runner::*;
use crate::testing::utils::*;
use send_wrapper::SendWrapper;
use test::TestDesc;
use test::TestDescAndFn;


/// Inserts the provided tests into the [`World`] by cloning.
///
/// # Panics
///
/// Panics if dynamic tests or benches are passed in, see [`test_ext::clone`].
pub fn tests_bundle_borrowed(tests: &[&TestDescAndFn]) -> impl Bundle {
	let tests = tests
		.iter()
		.map(|test| test_ext::clone_static(test))
		.collect();
	tests_bundle(tests)
}

/// Inserts an owned set of tests.
pub fn tests_bundle(tests: Vec<TestDescAndFn>) -> impl Bundle {
	let test_bundles: Vec<_> = tests.into_iter().map(test_bundle).collect();
	(
		// Request::from_cli_args(CliArgs::parse_env()).unwrap_or_exit(),
		Children::spawn(SpawnIter(test_bundles.into_iter())),
	)
}

fn test_bundle(test: TestDescAndFn) -> impl Bundle {
	(test_desc_bundle(test.desc), test_fn_bundle(test.testfn))
}

fn test_fn_bundle(func: test::TestFn) -> impl Bundle {
	match func {
		test::TestFn::StaticTestFn(func) => {
			OnSpawn::insert(TestFunc::new(func))
		}
		test::TestFn::DynTestFn(fn_once) => {
			OnSpawn::insert(NonSendTestFunc::new(fn_once))
		}
		test::TestFn::StaticBenchFn(_) => todo!(),
		test::TestFn::DynBenchFn(_) => todo!(),
		test::TestFn::StaticBenchAsTestFn(_) => todo!(),
		test::TestFn::DynBenchAsTestFn(_) => todo!(),
	}
}
fn test_desc_bundle(desc: test::TestDesc) -> impl Bundle {
	(
		Name::new(desc.name.to_string()),
		FileSpan::new(
			desc.source_file,
			LineCol::new(desc.start_line as u32, desc.start_col as u32),
			LineCol::new(desc.end_line as u32, desc.end_col as u32),
		),
		Test::new(desc.clone()),
		OnSpawn::new(move |entity| try_skip(entity, &desc)),
	)
}
/// Handles the skip cases that are statically known (before filtering).
fn try_skip(entity: &mut EntityWorldMut, desc: &TestDesc) {
	if desc.no_run {
		entity.insert(TestOutcome::Skip(TestSkip::NoRun));
	}
	if desc.compile_fail {
		entity.insert(TestOutcome::Skip(TestSkip::CompileFail));
	}
	if desc.ignore {
		entity.insert(TestOutcome::Skip(TestSkip::Ignore(desc.ignore_message)));
	}
}


/// Per-test parameters that can be configured via attributes.
#[derive(Debug, Clone, Component)]
pub struct TestCaseParams {
	/// Optional timeout override for this specific test.
	pub timeout: Option<Duration>,
}

impl Default for TestCaseParams {
	fn default() -> Self { Self { timeout: None } }
}

impl TestCaseParams {
	/// Creates a new [`TestCaseParams`] with default values.
	pub fn new() -> Self { Self::default() }

	/// Sets a timeout in milliseconds for this test.
	pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
		self.timeout = Some(Duration::from_millis(timeout_ms));
		self
	}
}

/// Component representing a test case.
#[derive(Debug, Component)]
pub struct Test {
	/// The test description.
	desc: TestDesc,
	/// Counts the time elapsed since the test started.
	timer: Stopwatch,
}

impl std::ops::Deref for Test {
	type Target = TestDesc;
	fn deref(&self) -> &Self::Target { &self.desc }
}

impl Test {
	/// Creates a new [`Test`] from a test description.
	pub fn new(desc: TestDesc) -> Self {
		Self {
			desc,
			timer: default(),
		}
	}

	/// Returns the test description.
	pub fn desc(&self) -> &TestDesc { &self.desc }

	/// Returns true if the test should not be run,
	/// ie if `ignore` or `no_run` flag is set.
	pub fn do_not_run(&self) -> bool {
		self.desc.ignore || self.desc.no_run || self.desc.compile_fail
	}

	/// Advances the test timer by the given delta.
	pub fn tick(&mut self, delta: Duration) { self.timer.tick(delta); }

	/// Returns the elapsed time since the test started.
	pub fn elapsed(&self) -> Duration { self.timer.elapsed() }
}

impl TestDescExt for Test {
	fn desc(&self) -> &TestDesc { &self.desc }
	fn desc_mut(&mut self) -> &mut TestDesc { &mut self.desc }
}


/// Component wrapping a static test function.
#[derive(Debug, Copy, Clone, Component)]
pub struct TestFunc(fn() -> Result<(), String>);

impl TestFunc {
	/// Creates a new [`TestFunc`] from a function pointer.
	pub fn new(func: fn() -> Result<(), String>) -> Self { Self(func) }

	/// Runs the test function and returns the result.
	pub fn run(&self) -> Result<(), String> { self.0() }
}

/// Component wrapping a dynamic test function.
///
/// The [`test::TestFn::DynTestFn`] is [`Send`] but not [`Sync`].
/// This type is for running these tests on the main thread.
#[derive(Component)]
pub struct NonSendTestFunc(
	SendWrapper<Box<dyn 'static + FnOnce() -> Result<(), String>>>,
);

impl NonSendTestFunc {
	/// Creates a new [`NonSendTestFunc`] from a boxed function.
	pub fn new(func: impl 'static + FnOnce() -> Result<(), String>) -> Self {
		Self(SendWrapper::new(Box::new(func)))
	}

	/// Runs the test function and returns the result.
	pub fn run(self) -> Result<(), String> { self.0.take()() }
}




#[cfg(test)]
mod tests {
	use super::*;

	fn setup() -> Vec<test::TestDescAndFn> {
		vec![
			test_ext::new("test1", "crates/crate1/file1.rs", || Ok(())),
			test_ext::new("test2", "crates/crate1/file1.rs", || Ok(())),
			test_ext::new("test1", "crates/crate2/file1.rs", || Ok(())),
			test_ext::new("test1", "crates/crate2/dir1/file1.rs", || Ok(())),
			test_ext::new("test2", "crates/crate2/dir1/file1.rs", || Ok(())),
			// test_ext::new("bar", file!(), || Err("poop".into())),
			// test_ext::new("bar", file!(), || Ok(())),
		]
	}

	#[test]
	fn test_tree() {
		let mut world = World::new();
		let root = world.spawn(tests_bundle(setup())).id();
		world
			.component_names_related::<Children>(root)
			.iter_to_string_indented()
			.xpect_snapshot();
	}
}
