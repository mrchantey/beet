//! Lambda + API Gateway + Cloudflare DNS example using the typed provider API.
//!
//! Run with:
//! ```sh
//!   cargo run --example lambda --features=lambda_block
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	App::new()
		.add_plugins((MinimalPlugins, InfraPlugin, LogPlugin {
			// level: Level::TRACE,
			..default()
		}))
		.add_systems(Startup, setup)
		.run();
	Ok(())
}


fn setup(mut commands: Commands) {
	commands.spawn((
		Stack::new("lambda-example").with_backend(LocalBackend::default()),
		LambdaBlock::default(),
		stack_cli(),
		OnSpawn::insert_child(route(
			"run",
			(Sequence::<Request, Response>::default(), children![Foobar]),
		)),
	));
}


#[action]
#[derive(Component)]
// #[reflect(Component)]
async fn Foobar(_cx: ActionContext<Request>) -> Result<Outcome<Response>> {
	println!("FOOBAR");
	Pass(Response::ok()).xok()
}
