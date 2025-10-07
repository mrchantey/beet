#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;
use beet_flow::prelude::*;
use sweet::prelude::*;


#[action(foo)]
#[derive(Component)]
struct Foo;

fn foo(run: On<Run>, mut cmd: AsyncCommands) {
	let entity = run.event_target();
	cmd.run(async move |world| {
		time_ext::sleep(Duration::from_millis(20)).await;
		world.entity(entity).trigger_payload(Outcome::Pass).await;
	});
}


#[sweet::test]
async fn works() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, BeetFlowPlugin));
	app.world_mut()
		.spawn((Sequence, ExitOnEnd, children![Foo, EndWith(Outcome::Fail)]))
		.trigger_payload(GetOutcome);

	app.run_async().await.xpect_eq(AppExit::error());
}
