//! # Router Example
//!
//! Demonstrates beet's routing system with multiple server backends.
//! Routes are wrapped in a layout template via [`WrapDescendentsList`],
//! so HTML responses automatically include navigation and styling.
//!
//! ## Running the Example
//!
//! ```sh
//! # CLI mode (default) — show root content and exit
//! cargo run --example router
//!
//! # CLI mode — show help for all routes
//! cargo run --example router -- --help
//!
//! # CLI mode — navigate to a scene
//! cargo run --example router -- about
//!
//! # CLI mode — show help scoped to a subcommand
//! cargo run --example router -- counter --help
//!
//! # CLI mode — request HTML output wrapped in the layout template
//! cargo run --example router -- --accept=text/html
//! cargo run --example router -- about --accept=text/html
//!
//! # HTTP mode — start an HTTP server on port 8337
//! cargo run --example router --features http_server
//!
//! # REPL mode — interactive read-eval-print loop
//! cargo run --example router -- --server=repl
//! ```
use beet::prelude::*;
mod content;

fn main() -> AppExit {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin {
				level: Level::TRACE,
				..default()
			},
			RouterAppPlugin,
		))
		.add_systems(Startup, setup)
		.run()
}

fn setup(mut commands: Commands) -> Result {
	commands.spawn((
		server_from_cli()?,
		default_router(),
		SceneToolRenderer::default(),
		content::routes(),
	));
	Ok(())
}


fn server_from_cli() -> Result<OnSpawn> {
	cfg_if! {
		if #[cfg(feature="http_server")]{
			let default_server = "http";
		}else{
			let default_server = "cli";
		}
	};


	match CliArgs::parse_env()
		.params
		.get("server")
		.map(|val: &String| val.to_lowercase())
		.unwrap_or_else(|| default_server.into())
		.as_str()
	{
		// use on_spawn to avoid clobbering children!
		#[cfg(feature = "http_server")]
		"http" => OnSpawn::insert(HttpServer::default()),
		#[cfg(not(feature = "http_server"))]
		"http" => bevybail!("Add the 'http_server' feature for http servers"),
		"repl" => OnSpawn::insert(ReplServer::default()),
		"cli" => OnSpawn::insert(CliServer::default()),
		_ => {
			bevybail!(
				"Invalid server type specified. Accepted options are http,repl,cli"
			);
		}
	}
	.xok()
}
