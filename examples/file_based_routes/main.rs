//! # File-based routes example
//!
//! End-to-end demonstration of Beet's file-based routing codegen:
//!
//! - `.rs` page handlers + `.md` content, scanned into route bundles
//! - a server action invoked through a generated, typed client caller
//! - compile-time-checked links via the generated `routes::` module
//! - serving over CLI/HTTP, plus dev commands wired as routes
//!
//! The whole app is a single router: pages, content, and the `add` server
//! action are routes, and so are the `codegen`, `export`, and `call-add` dev
//! commands. The server backend is chosen with `--server` (`cli` or `http`).
//!
//! ## Running
//!
//! ```sh
//! # regenerate the generated/ files from the route sources
//! cargo run --example file_based_routes --features codegen -- codegen
//!
//! # CLI mode: render the home route (default), or a named route
//! cargo run --example file_based_routes
//! cargo run --example file_based_routes -- about
//!
//! # call the `add` server action in-process through the router
//! # (defaults to --a=2 --b=3)
//! cargo run --example file_based_routes -- call-add --a=10 --b=20
//!
//! # static export to examples/file_based_routes/dist
//! cargo run --example file_based_routes -- export
//!
//! # HTTP mode (then curl the routes, eg the `add` server action)
//! cargo run --example file_based_routes -- --server=http
//! ```
use beet::prelude::*;

/// Types shared between server-action handlers and their generated callers.
pub mod shared {
	use serde::Deserialize;
	use serde::Serialize;

	/// Arguments for the `add` server action.
	#[derive(Debug, Serialize, Deserialize)]
	pub struct AddArgs {
		/// First addend.
		pub a: i32,
		/// Second addend.
		pub b: i32,
	}
}

mod generated;

/// Re-exports for the generated code (which imports `crate::prelude`).
pub mod prelude {
	pub use crate::generated::*;
	pub use crate::shared::*;
	pub use beet::prelude::*;
}

use crate::prelude::*;

/// The store backing markdown/content routes.
fn content_store() -> FsStore {
	FsStore::new(WsPathBuf::new("examples/file_based_routes/content"))
}

/// The full site: a router with every collection's routes merged in.
fn site() -> impl Bundle {
	(
		content_store(),
		router(),
		pages_routes(),
		content_routes(),
		actions_routes(),
	)
}

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), ClientAppPlugin))
		.add_systems(Startup, setup)
		.run()
}

/// Spawns the server (`--server=cli|http`) with the site and the dev-command
/// routes layered on the same router.
fn setup(mut commands: Commands) -> Result {
	commands.spawn((server_from_cli()?, site())).with_children(
		|parent| {
			#[cfg(feature = "codegen")]
			parent.spawn(exchange_route("codegen", Codegen));
			parent.spawn(exchange_route("export", Export));
			parent.spawn(exchange_route("call-add", CallAdd));
		},
	);
	Ok(())
}

/// Selects the server backend from `--server`, defaulting to `cli`.
fn server_from_cli() -> Result<OnSpawn> {
	match CliArgs::parse_env()
		.params
		.get("server")
		.map(|server| server.to_lowercase())
		.as_deref()
	{
		None | Some("cli") => CliServer::default().any_bundle().xok(),
		Some("http") => HttpServer::default().any_bundle().xok(),
		Some(other) => {
			bevybail!("invalid --server '{other}', expected 'cli' or 'http'")
		}
	}
}

/// Statically exports every static route to `examples/file_based_routes/dist`.
#[action]
#[derive(Component)]
async fn Export(cx: ActionContext) -> Result<String> {
	let caller = cx.caller.clone();
	let world = cx.world();
	let router = caller
		.with_state::<AncestorQuery, Entity>(|entity, query| {
			query.root_ancestor(entity)
		})
		.await?;
	let out = BlobStore::new(FsStore::new(WsPathBuf::new(
		"examples/file_based_routes/dist",
	)));
	let written = export_static(&world, router, &out).await?;
	Ok(format!("exported {} routes to dist", written.len()))
}

/// Request params for the `call-add` command.
#[derive(Reflect)]
struct CallAddParams {
	/// First addend, defaults to 2.
	a: Option<i32>,
	/// Second addend, defaults to 3.
	b: Option<i32>,
}

/// Calls the `add` server action in-process through the router, with the
/// addends taken from `--a`/`--b` (defaulting to 2 and 3).
#[action]
#[derive(Component)]
#[require(ParamsPartial = ParamsPartial::new::<CallAddParams>())]
async fn CallAdd(cx: ActionContext<Request>) -> Result<String> {
	let a = cx.get_param("a").and_then(|val| val.parse().ok()).unwrap_or(2);
	let b = cx.get_param("b").and_then(|val| val.parse().ok()).unwrap_or(3);
	let caller = cx.caller.clone();
	let world = cx.world();
	let router = caller
		.with_state::<AncestorQuery, Entity>(|entity, query| {
			query.root_ancestor(entity)
		})
		.await?;
	let request = Request::post("add")
		.with_accept(MediaType::Json)
		.with_json_body(&AddArgs { a, b })?;
	let sum: i32 = world
		.entity(router)
		.call::<Request, Response>(request)
		.await?
		.into_result()
		.await?
		.json()
		.await?;
	Ok(format!("add({a}, {b}) = {sum}"))
}

/// Regenerates the `generated/` files from the route sources.
#[cfg(feature = "codegen")]
#[action]
#[derive(Component)]
async fn Codegen(_: ActionContext) -> Result<String> {
	fn dir(sub: &str) -> AbsPathBuf {
		AbsPathBuf::new_workspace_rel(format!("examples/file_based_routes/{sub}"))
			.unwrap()
	}
	fn cg(name: &str) -> CodegenFile {
		CodegenFile::new(dir(&format!("generated/{name}")))
	}

	RouteCodegen::new()
		.add_collection(RouteCollection::new(dir("pages"), cg("pages.rs")))
		.add_collection(RouteCollection::new(dir("content"), cg("content.rs")))
		.add_collection(
			RouteCollection::new(dir("actions"), cg("actions.rs"))
				.with_category(RouteCollectionCategory::Actions)
				.with_server_feature(None::<String>),
		)
		.with_route_tree(cg("route_tree.rs"))
		.with_client_actions(cg("client_actions.rs"))
		.export()
		.await?;
	Ok("regenerated codegen files".into())
}
