//! The `beet` command-line interface.
//!
//! Every command runs as a route on a [`CliServer`]-backed [`router`], so
//! `beet --help` lists them and `beet <command>` dispatches. `run-wasm` is the
//! cargo runner for `wasm32-unknown-unknown` targets; it is served greedily
//! (`run-wasm/*args`) so the binary path and trailing args are captured and
//! forwarded to the running module.
use beet::prelude::*;
use beet_cli::prelude::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), ClientAppPlugin))
		.add_systems(Startup, setup)
		.run()
}

/// Spawns the CLI server with every command wired as a route.
fn setup(mut commands: Commands) {
	commands
		.spawn((CliServer::default(), default_router()))
		.with_children(|parent| {
			parent.spawn(exchange_route("run-wasm/*args", RunWasm));
			parent.spawn(exchange_route("build-wasm", BuildWasm));
			parent.spawn(exchange_route("export-pdf", ExportPdf));
			#[cfg(feature = "qrcode")]
			parent.spawn(exchange_route("qrcode", QrCode));
		});
}
