//! The `beet` command-line interface.
//!
//! Most commands run as routes on a [`CliServer`]-backed [`router`], so
//! `beet --help` lists them and `beet <command>` dispatches. `run-wasm` is
//! handled directly so it can forward trailing args verbatim to the running
//! module (it is the cargo runner for `wasm32-unknown-unknown` targets).
use beet::prelude::*;
use beet_cli::prelude::*;

fn main() -> AppExit {
	let args: Vec<String> = std::env::args().collect();
	// `beet run-wasm <binary> [args..]` — forward trailing args to the module.
	if args.get(1).map(String::as_str) == Some("run-wasm") {
		return run_wasm_command(&args);
	}

	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), ClientAppPlugin))
		.add_systems(Startup, setup)
		.run()
}

/// Spawns the CLI server with every command wired as a route.
fn setup(mut commands: Commands) {
	commands
		.spawn((CliServer::default(), router()))
		.with_children(|parent| {
			parent.spawn(exchange_route(
				"",
				Action::<(), &str>::new_pure(|_| "🫖 the beet cli"),
			));
			parent.spawn(build_wasm_route());
			parent.spawn(exchange_route("export-pdf", ExportPdf));
			#[cfg(feature = "qrcode")]
			parent.spawn(exchange_route("qrcode", QrCode));
		});
}

/// Runs the [`RunWasm`] action directly, forwarding trailing args to the module.
fn run_wasm_command(args: &[String]) -> AppExit {
	let Some(exe_path) = args.get(2).cloned() else {
		eprintln!("usage: beet run-wasm <binary-path> [args..]");
		return AppExit::error();
	};
	let forwarded = args.get(3..).map(<[String]>::to_vec).unwrap_or_default();

	let result = AsyncPlugin::world()
		.spawn(RunWasm::new(exe_path).with_args(forwarded))
		.call_blocking::<(), ()>(());

	match result {
		Ok(()) => AppExit::Success,
		Err(err) => {
			eprintln!("run-wasm failed: {err}");
			AppExit::error()
		}
	}
}
