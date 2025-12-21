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

#[derive(Component)]
pub struct TestRoot;
pub fn start_test_runner(
	mut commands: Commands,
	query: Query<Entity, With<TestRoot>>,
) -> Result {
	commands.entity(query.single()?).trigger_target(GetOutcome);
	Ok(())
}

fn spawn_test_tree(world: &mut World, tests: Vec<TestDescAndFn>) {
	let root = world.spawn((TestRoot, Sequence, ExitOnEnd)).id();
	for test in tests {
		let mut entity = world.spawn(ChildOf(root));
		println!("spawning test");
		test_desc_and_fn_ext::insert(&mut entity, test);
	}
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
	runner.run(ev, || func.run());
	Ok(())
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
	runner.run(ev, move || func.run());
	Ok(())
}

#[derive(SystemParam)]
struct TestRunner<'w, 's> {
	commands: Commands<'w, 's>,
}

impl TestRunner<'_, '_> {
	fn run(
		&mut self,
		mut ev: On<GetOutcome>,
		func: impl FnOnce() -> Result<(), String>,
	) {
		match func() {
			Ok(_) => {
				ev.trigger_with_cx(Outcome::Pass);
			}
			Err(err) => {
				ev.trigger_with_cx(Outcome::Fail);
			}
		}
		// ev.trigger_with_cx(Outcome::Pass);
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use test::TestDescAndFn;

	fn setup() -> Vec<TestDescAndFn> {
		vec![
			test_desc_and_fn_ext::new("foo", file!(), || Ok(())),
			test_desc_and_fn_ext::new("bar", file!(), || Ok(())),
			// test_desc_and_fn_ext::new("bar", file!(), || Err("poop".into())),
			// test_desc_and_fn_ext::new("bar", file!(), || Ok(())),
		]
	}

	#[test]
	fn works() { super::run(setup()); }
}
