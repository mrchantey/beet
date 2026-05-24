//! # File-based routes example
//!
//! End-to-end demonstration of Beet's file-based routing codegen:
//!
//! - `.rs` page handlers + `.md` content, scanned into route bundles
//! - a server action invoked through a generated, typed client caller
//! - compile-time-checked links via the generated `routes::` module
//! - serving over CLI/HTTP and static-site export
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
//! # HTTP mode
//! cargo run --example file_based_routes -- serve
//!
//! # call the server action through the generated client caller
//! cargo run --example file_based_routes -- call-add
//!
//! # static export to examples/file_based_routes/dist
//! cargo run --example file_based_routes -- export
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
	let arg = std::env::args().nth(1);
	match arg.as_deref() {
		#[cfg(feature = "codegen")]
		Some("codegen") => {
			async_ext::block_on(run_codegen()).unwrap();
			AppExit::Success
		}
		Some("export") => run_export(),
		Some("call-add") => run_call_add(),
		Some("serve") => run_http(),
		// default: CLI server (also handles `-- about`, `-- --help`, etc.)
		_ => run_cli(),
	}
}

/// Runs the router as a CLI server, navigating to the requested route.
fn run_cli() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), ClientAppPlugin))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((CliServer::default(), site()));
		})
		.run()
}

/// Runs the router as an HTTP server on the default port.
fn run_http() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), ClientAppPlugin))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((HttpServer::default(), site()));
		})
		.run()
}

/// Starts an HTTP server in the background and calls the `add` server action
/// through its generated client caller.
fn run_call_add() -> AppExit {
	std::thread::spawn(|| {
		App::new()
			.add_plugins((MinimalPlugins, ClientAppPlugin))
			.add_systems(Startup, |mut commands: Commands| {
				commands.spawn((HttpServer::default(), site()));
			})
			.run();
	});

	let result = async_ext::block_on(async {
		// give the background server a moment to bind
		time_ext::sleep_millis(500).await;
		generated::client_actions::add::post(AddArgs { a: 2, b: 3 }).await
	});

	match result {
		Ok(sum) => {
			println!("client called add(2, 3) = {sum}");
			AppExit::Success
		}
		Err(err) => {
			eprintln!("call-add failed: {err}");
			AppExit::error()
		}
	}
}

/// Statically exports every static route to `examples/file_based_routes/dist`.
fn run_export() -> AppExit {
	let mut world = (AsyncPlugin, ClientAppPlugin).into_world();
	let router = world.spawn(site()).flush();
	let out = BlobStore::new(FsStore::new(WsPathBuf::new(
		"examples/file_based_routes/dist",
	)));

	let written = async_ext::block_on(world.run_async_then(
		async move |world| export_static(&world, router, &out).await,
	));

	match written {
		Ok(paths) => {
			for path in paths {
				println!("exported {path}");
			}
			AppExit::Success
		}
		Err(err) => {
			eprintln!("export failed: {err}");
			AppExit::error()
		}
	}
}

/// Regenerates the `generated/` files from the route sources.
#[cfg(feature = "codegen")]
async fn run_codegen() -> Result<()> {
	fn dir(sub: &str) -> AbsPathBuf {
		AbsPathBuf::new_workspace_rel(format!(
			"examples/file_based_routes/{sub}"
		))
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
		.await
}
