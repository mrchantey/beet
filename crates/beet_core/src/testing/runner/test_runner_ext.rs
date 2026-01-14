use crate::prelude::*;
use crate::testing::runner::*;


/// Run a single test in an isolated Bevy app, returning the outcome
pub async fn run(args: Option<&str>, test: test::TestDescAndFn) -> TestOutcome {
	let mut app = App::new().with_plugins((MinimalPlugins, TestPlugin));

	let args = if let Some(args) = args {
		format!("{args} --quiet")
	} else {
		"--quiet".into()
	};
	app.world_mut().spawn((
		Request::from_cli_str(&args).unwrap(),
		tests_bundle(vec![test]),
	));
	let store = Store::new(None);

	app.add_observer(
		move |ev: On<Insert, TestOutcome>, outcomes: Query<&TestOutcome>| {
			store.set(Some(outcomes.get(ev.entity).unwrap().clone()));
		},
	);
	app.run_async().await;
	store.get().unwrap()
}
