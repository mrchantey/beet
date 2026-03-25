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
	let args = CliArgs::parse_env();
	let server_kind = args
		.params
		.get("server")
		.map(|val: &String| val.to_lowercase())
		.unwrap_or_else(|| "cli".into());

	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), BeetRouterPlugin))
		.add_systems(Startup, move |mut commands: Commands| {
			match server_kind.as_str() {
				#[cfg(feature = "http_server")]
				"http" => {
					commands.spawn((http_server(3000), content::stack()));
				}
				#[cfg(not(feature = "http_server"))]
				"http" => {
					cross_log_error!(
						"HTTP server requires the `http_server` feature.\n\
						 Run with: cargo run --example router --features http_server -- --server=http"
					);
				}
				"repl" => {
					commands.spawn((repl_server(), content::stack()));
				}
				"cli" | _ => {
					commands.spawn((cli_server(), content::stack()));
				}
			}
		})
		.run()
}
