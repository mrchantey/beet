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
		world.entity(entity).trigger_payload(SUCCESS).await;
	});
}


#[sweet::test]
async fn works() {
	let mut app = App::new();
	app.add_plugins((BeetFlowPlugin, DebugFlowPlugin::default()));
	app.world_mut().spawn((
		Sequence,
		EntityObserver::new(|ev: On<End>, mut commands: Commands| {
			ev.value().xpect_eq(FAILURE);
			commands.write_message(AppExit::Success);
			todo!("exit on end");
		}),
		children![Foo, EndOnRun(FAILURE)],
	));

	app.run_async().await.xpect_eq(AppExit::Success);
}
