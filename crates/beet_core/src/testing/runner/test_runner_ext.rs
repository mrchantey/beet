use crate::prelude::*;
use crate::testing::runner::*;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;

/// Run a single test in an isolated Bevy app, returning the outcome
pub async fn run(args: Option<&str>, test: TestDescAndFn) -> TestOutcome {
	run_all(args, vec![test]).await.pop().unwrap()
}

/// Run `tests` as one suite in an isolated Bevy app, returning their outcomes
/// in the order they landed.
pub async fn run_all(
	args: Option<&str>,
	tests: Vec<TestDescAndFn>,
) -> Vec<TestOutcome> {
	let mut app = App::new().with_plugins((MinimalPlugins, TestPlugin));

	let args = if let Some(args) = args {
		format!("{args} --quiet")
	} else {
		"--quiet".into()
	};
	app.world_mut()
		.spawn((TestRunnerConfig::from_cli_str(&args), tests_bundle(tests)));
	let outcomes = Arc::new(Mutex::new(Vec::new()));
	let landed = outcomes.clone();
	app.add_observer(
		move |ev: On<Insert<TestOutcome>>, outcomes: Query<&TestOutcome>| {
			landed
				.lock()
				.unwrap()
				.push(outcomes.get(ev.entity).unwrap().clone());
		},
	);
	app.run_async().await;
	core::mem::take(&mut *outcomes.lock().unwrap())
}
