//! # Router Serde Example
//!
//! Mirrors [`cli`](./cli.rs), but persists the entire route world to
//! disk via [`TemplateStore`]. On first run the world is written to
//! `examples/router/router_serde.json`, and is loaded from that file
//! on subsequent runs. Pass `--new` to overwrite the file with a
//! fresh copy.
//!
//! Every runtime component — [`CliServer`], the [`router`] bundle, the
//! middleware and the [`ExchangeScript`] markers — is `Reflect`, so the
//! world round-trips with no post-load patching.
//!
//! ## Running the Example
//!
//! ```sh
//! # visit the home route (first run also writes the serde file)
//! cargo run --example router_serde
//!
//! # visit the /foo route
//! cargo run --example router_serde -- foo
//!
//! # invoke the scripted greeter via a typed query struct
//! cargo run --example router_serde -- greet --name=world
//!
//! # invoke the scripted greeter via the raw request parts
//! cargo run --example router_serde -- greet-request --name=world
//!
//! # delete and regenerate the serde file
//! cargo run --example router_serde -- --new
//! ```
use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

const WORLD_SERDE_FILE: &str = "examples/router/router_serde.json";

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, ClientAppPlugin))
		// only the example-specific [`GreetRequest`] instantiations need
		// registering — ClientAppPlugin's RouterPlugin / ActionPlugin
		// cover the hierarchy and unit-input Script types.
		.register_type::<Script<QueryParams<GreetRequest>, String>>()
		.register_type::<ExchangeScript<QueryParams<GreetRequest>, String, _, _>>(
		)
		.add_systems(Startup, setup)
		.run()
}

/// Query params for the scripted greet route, exposed to the rhai
/// script as `input.name`.
#[derive(Serialize, Deserialize, TypePath)]
struct GreetRequest {
	name: String,
}

fn setup(async_commands: AsyncCommands) {
	let blob = FsStore::new(WsPathBuf::default())
		.blob(SmolPath::new(WORLD_SERDE_FILE));
	let new_world = CliArgs::parse_env().params.contains_key("new");

	async_commands.run(async move |world: AsyncWorld| {
		if new_world {
			blob.remove().await.ok();
		}
		TemplateStore::load_or_create(world.clone(), blob, async |_| {
			route_bundle().xok()
		})
		.await?;
		Ok(())
	});
}

fn route_bundle() -> impl Bundle {
	(
		CliServer::default(),
		(default_router(), children![
			(
				Script::<(), String>::rhai(r#""hello world""#),
				ExchangeScript::<(), String>::default(),
				PathPartial::new(""),
			),
			(
				Script::<(), String>::rhai(r#""hello foo""#),
				ExchangeScript::<(), String>::default(),
				PathPartial::new("foo"),
			),
			(
				Script::<QueryParams<GreetRequest>, String>::rhai(
					r#""hello " + input.name"#,
				),
				ExchangeScript::<QueryParams<GreetRequest>, String, _, _>::default(),
				PathPartial::new("greet"),
			),
			// same idea, but the script receives the full [`RequestParts`]
			// and digs out the `name` query parameter itself.
			(
				Script::<RequestParts, String>::rhai(
					r#""hello " + input.url.params.name[0]"#,
				),
				ExchangeScript::<RequestParts, String, _, _>::default(),
				PathPartial::new("greet-request"),
			),
		]),
	)
}
