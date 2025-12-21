use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use bevy::ecs::system::NonSendMarker;
use send_wrapper::SendWrapper;
use test::TestDescAndFn;


/// Entry point for the sweet test runner
/// ## Panics
/// Panics if dynamic tests or benches are passed in, see [`test_desc_and_fn_ext::clone`]
pub fn run_static(tests: &[&TestDescAndFn]) {
	let tests = tests
		.iter()
		.map(|test| test_desc_and_fn_ext::clone(test))
		.collect();
	run(tests);
}
/// Runs an owned set of tests.
pub fn run(tests: Vec<TestDescAndFn>) {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TestPlugin));
	spawn_test_tree(app.world_mut(), tests);
	app.run();
}


fn insert_test_desc_and_fn(entity: &mut EntityWorldMut, test: TestDescAndFn) {
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
	entity.insert((
		Name::new(desc.name.to_string()),
		TestType(desc.test_type),
		FileSpan::new(
			desc.source_file,
			LineCol::new(desc.start_line as u32, desc.start_col as u32),
			LineCol::new(desc.end_line as u32, desc.end_col as u32),
		),
	));
	if desc.ignore {
		entity.insert(ShouldIgnore {
			message: desc.ignore_message.map(|s| s.to_string()),
		});
	}
	match desc.should_panic {
		test::ShouldPanic::No => {}
		test::ShouldPanic::Yes => {
			entity.insert(ShouldPanic { message: None });
		}
		test::ShouldPanic::YesWithMessage(msg) => {
			entity.insert(ShouldPanic {
				message: msg.to_string().xsome(),
			});
		}
	}
	if desc.no_run {
		entity.insert(ShouldNotRun);
	}
	if desc.compile_fail {
		entity.insert(ShouldCompileFail);
	}
}




#[derive(Component)]
pub struct TestRoot;

#[derive(Debug, Deref, Component)]
pub struct ShouldIgnore {
	message: Option<String>,
}
#[derive(Debug, Deref, Component)]
pub struct ShouldPanic {
	message: Option<String>,
}

#[derive(Debug, Component)]
pub struct ShouldNotRun;

#[derive(Debug, Component)]
pub struct ShouldCompileFail;

#[derive(Debug, Deref, Component)]
pub struct TestType(test::TestType);

/// Inserted on failing tests, providing
/// the error message
#[derive(Debug, Deref, Component)]
#[component(storage = "SparseSet")]
pub struct TestFail(String);

impl Default for TestType {
	fn default() -> Self { Self(test::TestType::UnitTest) }
}

// pub name: TestName,
// pub ignore: bool,
// pub ignore_message: Option<&'static str>,
// pub source_file: &'static str,
// pub start_line: usize,
// pub start_col: usize,
// pub end_line: usize,
// pub end_col: usize,
// pub should_panic: options::ShouldPanic,
// pub compile_fail: bool,
// pub no_run: bool,
// pub test_type: TestType,


pub fn start_test_runner(
	mut commands: Commands,
	query: Query<Entity, With<TestRoot>>,
) -> Result {
	commands.entity(query.single()?).trigger_target(GetOutcome);
	Ok(())
}

fn spawn_test_tree(world: &mut World, tests: Vec<TestDescAndFn>) -> Entity {
	let root = world.spawn((TestRoot, Sequence, ExitOnEnd)).id();
	for test in tests {
		let mut entity = world.spawn(ChildOf(root));
		insert_test_desc_and_fn(&mut entity, test);
	}
	root
}


#[action(run_send_test_func)]
#[derive(Debug, Copy, Clone, Component)]
pub struct TestFunc(fn() -> Result<(), String>);

impl TestFunc {
	pub fn new(func: fn() -> Result<(), String>) -> Self { Self(func) }
	pub fn run(&self) -> Result<(), String> { self.0() }
}
fn run_send_test_func(
	ev: On<GetOutcome>,
	mut runner: TestRunner,
	query: Query<&TestFunc>,
) -> Result {
	let func = query.get(ev.action())?;
	runner.run(ev, || func.run())
}

/// The [`test::TestFn::DynTestFn`] is [`Send`] but not [`Sync`]
/// this type is for running these tests on the main thread.
#[action(run_non_send_test_func)]
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
fn run_non_send_test_func(
	ev: On<GetOutcome>,
	// ensure this observer runs on main thread
	_: NonSendMarker,
	mut commands: Commands,
	mut runner: TestRunner,
	mut query: Query<&mut NonSendTestFunc>,
) -> Result {
	commands.entity(ev.action()).remove::<NonSendTestFunc>();
	let func = std::mem::replace(
		query.get_mut(ev.action())?.as_mut(),
		// safe because we remove the component immediately
		NonSendTestFunc::new(|| unreachable!("test func already taken")),
	);
	runner.run(ev, move || func.run())
}

#[derive(SystemParam)]
struct TestRunner<'w, 's> {
	commands: Commands<'w, 's>,
	query: Query<
		'w,
		's,
		(
			&'static Name,
			&'static TestType,
			&'static FileSpan,
			Option<&'static ShouldPanic>,
			Option<&'static ShouldNotRun>,
			Option<&'static ShouldCompileFail>,
			Option<&'static ShouldIgnore>,
		),
	>,
}


impl TestRunner<'_, '_> {
	fn run(
		&mut self,
		mut ev: On<GetOutcome>,
		func: impl FnOnce() -> Result<(), String>,
	) -> Result {
		let (
			_name,
			_test_type,
			_file_span,
			_should_panic,
			should_ignore,
			should_not_run,
			should_compile_fail,
		) = self.query.get(ev.action())?;
		if should_ignore.is_some()
			|| should_not_run.is_some()
			|| should_compile_fail.is_some()
		{
			ev.trigger_with_cx(Outcome::Pass);
			return Ok(());
		}
		match func() {
			Ok(_) => {
				ev.trigger_with_cx(Outcome::Pass);
			}
			Err(err) => {
				self.commands.entity(ev.action()).insert(TestFail(err));
				ev.trigger_with_cx(Outcome::Fail);
			}
		}
		Ok(())
		// ev.trigger_with_cx(Outcome::Pass);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn setup() -> Vec<test::TestDescAndFn> {
		vec![
			test_desc_and_fn_ext::new(
				"test1",
				"crates/crate1/file1.rs",
				|| Ok(()),
			),
			test_desc_and_fn_ext::new(
				"test2",
				"crates/crate1/file1.rs",
				|| Ok(()),
			),
			test_desc_and_fn_ext::new(
				"test1",
				"crates/crate2/file1.rs",
				|| Ok(()),
			),
			test_desc_and_fn_ext::new(
				"test1",
				"crates/crate2/dir1/file1.rs",
				|| Ok(()),
			),
			test_desc_and_fn_ext::new(
				"test2",
				"crates/crate2/dir1/file1.rs",
				|| Ok(()),
			),
			// test_desc_and_fn_ext::new("bar", file!(), || Err("poop".into())),
			// test_desc_and_fn_ext::new("bar", file!(), || Ok(())),
		]
	}

	#[test]
	#[ignore]
	fn test_tree() {
		let mut world = World::new();
		let root = spawn_test_tree(&mut world, setup());
		world
			.component_names_related::<Children>(root)
			.iter_to_string_indented()
			.xpect_snapshot();
	}
	
	#[test]
	fn runs() { run(setup()); }
}
