use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::BundleFunc;
use beet_router::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The wasm runner, runs the binary passed in at the `binary-path` positional argument
/// using the deno runner.
///
/// To use add the following:
///
/// ```toml
///
/// # .cargo/config.toml
///
/// [target.wasm32-unknown-unknown]
///
/// runner = 'beet run-wasm'
///
/// ```
///
pub fn run_wasm() -> impl Bundle {
	(Name::new("Run Wasm"), Sequence, children![
		wasm_bindgen(),
		init_deno(),
		run_deno(),
		(
			Name::new("Ok"),
			endpoint_action(StatusCode::Ok).bundle_func()
		)
	])
}

fn wasm_bindgen() -> impl Bundle {
	(
		Name::new("Wasm Bindgen"),
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 mut query: RouteQuery,
			 mut cmd_runner: CommandRunner|
			 -> Result {
				let exe_path = query.dyn_segment(ev.target(), "binary-path")?;
				path_ext::assert_exists(&exe_path)?;
				let cmd_config = CommandConfig::new("wasm-bindgen")
					.arg("--out-dir")
					.arg(wasm_runner_target_dir().to_string_lossy())
					.arg("--out-name")
					.arg("bindgen")
					.arg("--target")
					.arg("web")
					// alternatively es modules target: experimental-nodejs-module
					.arg("--no-typescript")
					.arg(exe_path);

				cmd_runner.run(ev, cmd_config)
			},
		),
	)
}
fn init_deno() -> impl Bundle {
	(
		Name::new("Init Deno"),
		OnSpawn::observe(
			|ev: On<GetOutcome>, mut commands: Commands| -> Result {
				let deno_runner_path = deno_runner_path();
				let deno_str = include_str!("./deno.ts");

				// return if the deno file already exists
				if fs_ext::exists(&deno_runner_path)? {
					let runner_hash = fs_ext::hash_file(&deno_runner_path)?;
					let deno_hash = fs_ext::hash_string(deno_str);
					if runner_hash == deno_hash {
						commands
							.entity(ev.target())
							.trigger_target(Outcome::Pass);
						return Ok(());
					}
				};

				let deno_installed =
					match Command::new("deno").arg("--version").status() {
						Ok(val) => val.success(),
						_ => false,
					};
				if !deno_installed {
					bevybail!(
						"
🦖 Beet uses Deno for the wasm runner 🦖

Install Deno via:
shell: 				curl -fsSL https://deno.land/install.sh | sh
powershell: 	irm https://deno.land/install.ps1 | iex
website: 			https://docs.deno.com/runtime/getting_started/installation/

"
					);
				}
				println!("copying deno file to {}", deno_runner_path.display());

				// wasm-bindgen will ensure parent dir exists
				fs::write(deno_runner_path, deno_str)?;
				commands.entity(ev.target()).trigger_target(Outcome::Pass);
				Ok(())
			},
		),
	)
}
fn run_deno() -> impl Bundle {
	(
		Name::new("Run Deno"),
		OnSpawn::observe(
			|ev: On<GetOutcome>, mut cmd_runner: CommandRunner| -> Result {
				// args will look like this so skip 2
				// `test-wasm binary-path ..actual-args`
				let args =
					env_ext::args().into_iter().skip(2).collect::<Vec<_>>();
				let child = CommandConfig::new("deno")
					.env("WORKSPACE_ROOT", env_ext::var("WORKSPACE_ROOT")?)
					.arg("--allow-read")
					.arg("--allow-net")
					.arg("--allow-env")
					.arg(deno_runner_path().to_string_lossy())
					.args(args);
				cmd_runner.run(ev, child)
			},
		),
	)
}

fn wasm_runner_target_dir() -> PathBuf {
	let mut path = workspace_root();
	path.push("target/wasm-runner");
	path
}

fn deno_runner_path() -> PathBuf {
	let mut path = wasm_runner_target_dir();
	path.push("deno.ts");
	path
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn works() {
		deno_runner_path()
			.to_string_lossy()
			.replace("\\", "/")
			.xpect_ends_with("target/wasm-runner/deno.ts");
	}
}
