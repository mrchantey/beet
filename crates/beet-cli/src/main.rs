//! The `beet` command-line interface.
//!
//! Most commands run as routes on a [`CliServer`]-backed [`router`], so
//! `beet --help` lists them and `beet <command>` dispatches. The `run-wasm`
//! command is handled directly so it can forward trailing args to the running
//! module (it is the cargo runner for `wasm32-unknown-unknown` targets).
use beet::prelude::*;

fn main() -> AppExit {
	let args: Vec<String> = std::env::args().collect();
	// `beet run-wasm <binary> [args..]` — forward trailing args to the module.
	if args.get(1).map(String::as_str) == Some("run-wasm") {
		return run_wasm_cmd(&args);
	}

	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), ClientAppPlugin))
		.add_systems(Startup, setup)
		.run()
}

fn setup(mut commands: Commands) {
	commands
		.spawn((CliServer::default(), router()))
		.with_children(|parent| {
			parent.spawn(exchange_route(
				"teapot",
				Action::<(), &str>::new_pure(|_| "🫖 I'm a teapot"),
			));
		});
}

/// Runs a wasm binary via `wasm-bindgen` + the bundled Deno runner.
fn run_wasm_cmd(args: &[String]) -> AppExit {
	let Some(exe_path) = args.get(2).cloned() else {
		eprintln!("usage: beet run-wasm <binary-path> [args..]");
		return AppExit::error();
	};
	let forwarded = args.get(3..).map(<[String]>::to_vec).unwrap_or_default();

	match async_ext::block_on(run_wasm(exe_path, forwarded)) {
		Ok(()) => AppExit::Success,
		Err(err) => {
			eprintln!("run-wasm failed: {err}");
			AppExit::error()
		}
	}
}
