use crate::prelude::*;
use beet_core::prelude::*;
use send_wrapper::SendWrapper;
use test::TestDesc;
use test::TestDescAndFn;


/// Insert the provided tests into the [`World`] by cloning.
/// ## Panics
/// Panics if dynamic tests or benches are passed in, see [`test_desc_and_fn_ext::clone`]
pub fn insert_tests_borrowed(world: &mut World, tests: &[&TestDescAndFn]) {
	let tests = tests
		.iter()
		.map(|test| test_ext::clone_static(test))
		.collect();
	insert_tests(world, tests);
}

/// Inserts an owned set of tests.
pub fn insert_tests(world: &mut World, tests: Vec<TestDescAndFn>) -> Entity {
	// let mut app = App::new();
	// app.add_plugins((MinimalPlugins, TestPlugin));
	let root = world.spawn(TestRoot).id();
	for test in tests {
		let mut entity = world.spawn(ChildOf(root));
		insert_test(&mut entity, test);
	}
	root
}

fn insert_test(entity: &mut EntityWorldMut, test: TestDescAndFn) {
	insert_test_desc(entity, test.desc);
	insert_test_fn(entity, test.testfn);
}

fn insert_test_fn(entity: &mut EntityWorldMut, func: test::TestFn) {
	match func {
		test::TestFn::StaticTestFn(func) => entity.insert(TestFunc::new(func)),
		test::TestFn::DynTestFn(fn_once) => {
			entity.insert(NonSendTestFunc::new(fn_once))
		}
		test::TestFn::StaticBenchFn(_) => todo!(),
		test::TestFn::DynBenchFn(_) => todo!(),
		test::TestFn::StaticBenchAsTestFn(_) => todo!(),
		test::TestFn::DynBenchAsTestFn(_) => todo!(),
	};
}
fn insert_test_desc(entity: &mut EntityWorldMut, desc: test::TestDesc) {
	ShouldSkip::insert(entity, &desc);
	entity.insert((
		Name::new(desc.name.to_string()),
		FileSpan::new(
			desc.source_file,
			LineCol::new(desc.start_line as u32, desc.start_col as u32),
			LineCol::new(desc.end_line as u32, desc.end_col as u32),
		),
		Test::new(desc),
	));
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
			// test_desc_and_fn_ext::new("bar", file!(), || Err("poop".into())),
			// test_desc_and_fn_ext::new("bar", file!(), || Ok(())),
		]
	}

	#[test]
	fn test_tree() {
		let mut world = World::new();
		let root = insert_tests(&mut world, setup());
		world
			.component_names_related::<Children>(root)
			.iter_to_string_indented()
			.xpect_snapshot();
	}
}
