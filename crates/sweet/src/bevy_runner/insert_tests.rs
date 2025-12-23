use crate::prelude::*;
use beet_core::prelude::*;
use send_wrapper::SendWrapper;
use test::TestDesc;
use test::TestDescAndFn;


/// Insert the provided tests into the [`World`] by cloning.
/// ## Panics
/// Panics if dynamic tests or benches are passed in, see [`test_ext::clone`]
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
		TestRoot,
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
		OnSpawn::new(move |entity| ShouldSkip::insert(entity, &desc)),
	)
}


#[derive(Component)]
pub struct TestRoot;

/// Marker component added to test entities either
/// upon spawn or after applying a filter.
#[derive(Debug, Component)]
pub enum ShouldSkip {
	/// The test has a `#[no_run]` attribute
	NoRun,
	/// The test has a `#[compile_fail]` attribute
	CompileFail,
	/// The test has an `#[ignore]` attribute
	Ignore(Option<&'static str>),
}


impl ShouldSkip {
	fn insert(entity: &mut EntityWorldMut, desc: &TestDesc) {
		if desc.no_run {
			entity.insert(ShouldSkip::NoRun);
		}
		if desc.compile_fail {
			entity.insert(ShouldSkip::CompileFail);
		}
		if desc.ignore {
			entity.insert(ShouldSkip::Ignore(desc.ignore_message));
		}
	}
}

#[derive(Debug, Deref, Component)]
pub struct ShouldPanic {
	message: Option<String>,
}

#[derive(Debug, Component)]
pub struct ShouldCompileFail;

#[derive(Debug, Deref, Component)]
pub struct Test {
	desc: TestDesc,
}

impl Test {
	pub fn new(desc: TestDesc) -> Self { Self { desc } }
	pub fn desc(&self) -> &TestDesc { &self.desc }
	/// Returns true if the test should not be run,
	/// ie if `ignore` or `no_run` flag
	pub fn do_not_run(&self) -> bool {
		self.desc.ignore || self.desc.no_run || self.desc.compile_fail
	}
}


#[derive(Debug, Copy, Clone, Component)]
pub struct TestFunc(fn() -> Result<(), String>);

impl TestFunc {
	pub fn new(func: fn() -> Result<(), String>) -> Self { Self(func) }
	pub fn run(&self) -> Result<(), String> { self.0() }
}

/// The [`test::TestFn::DynTestFn`] is [`Send`] but not [`Sync`]
/// this type is for running these tests on the main thread.
#[derive(Component)]
pub struct NonSendTestFunc(
	SendWrapper<Box<dyn 'static + FnOnce() -> Result<(), String>>>,
);
impl NonSendTestFunc {
	pub fn new(func: impl 'static + FnOnce() -> Result<(), String>) -> Self {
		Self(SendWrapper::new(Box::new(func)))
	}
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
