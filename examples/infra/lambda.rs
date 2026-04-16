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
			"deploy",
			(exchange_sequence(), children![
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|cx: ActionContext<Request>| {
						println!("in sequence!");
						Pass(cx.input)
					},
				),
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|_cx: ActionContext<Request>| {
						Fail(Response::ok().with_body("Sequence complete!"))
					}
				)
			]),
		)),
	));
}
