//! # Router Example
//!
//! Demonstrates beet's routing system with multiple server backends.
//!
//! ## Running the Example
//!
//! ```sh
//! # CLI mode (default) — show root content and exit
//! cargo run --example router --features stdio_server
//!
//! # CLI mode — show help for all routes
//! cargo run --example router --features stdio_server -- --help
//!
//! # CLI mode — navigate to a scene
//! cargo run --example router --features stdio_server -- about
//!
//! # CLI mode — show help scoped to a subcommand
//! cargo run --example router --features stdio_server -- counter --help
//!
//! # HTTP mode — start an HTTP server on port 3000
//! cargo run --example router --features http_server -- --server=http
//!
//! # REPL mode — interactive read-eval-print loop
//! cargo run --example router --features stdio_server -- --server=repl
//! ```
use beet::prelude::*;
mod content;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), BeetRouterPlugin))
		.add_systems(Startup, setup)
		.run()
}

fn setup(mut commands: Commands) -> Result {
	commands.spawn((
		server_from_cli()?,
		content::stack(),
		OnSpawn::insert_child(mime_render_tool()),
	));
	Ok(())
}


fn server_from_cli() -> Result<OnSpawn> {
	match CliArgs::parse_env()
		.params
		.get("server")
		.map(|val: &String| val.to_lowercase())
		.unwrap_or_else(|| "cli".into())
		.as_str()
	{
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
