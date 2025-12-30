use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
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
		StatusCode::OK.into_endpoint_handler()
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
				let exe_path = query.dyn_segment(&ev, "binary-path")?;
				path_ext::assert_exists(&exe_path)?;
				let cmd_config = CommandConfig::new("wasm-bindgen")
					.arg("--out-dir")
					.arg(sweet_target_dir().to_string_lossy())
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
		OnSpawn::observe(|mut ev: On<GetOutcome>| -> Result {
			let deno_runner_path = deno_runner_path();
			let deno_str = include_str!("./deno.ts");

			// return if the deno file already exists
			if fs_ext::exists(&deno_runner_path)? {
				let runner_hash = fs_ext::hash_file(&deno_runner_path)?;
				let deno_hash = fs_ext::hash_string(deno_str);
				if runner_hash == deno_hash {
					ev.trigger_with_cx(Outcome::Pass);
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
🦖 Sweet uses Deno for the wasm runner 🦖

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
			ev.trigger_with_cx(Outcome::Pass);
			Ok(())
		}),
	)
}
fn run_deno() -> impl Bundle {
	(
		Name::new("Run Deno"),
		OnSpawn::observe(
			|ev: On<GetOutcome>, mut cmd_runner: CommandRunner| -> Result {
				// args will look like this so skip 3
				// sweet test-wasm binary-path *actual-args
				// why doesnt it work with three?
				let args = std::env::args().skip(3).collect::<Vec<_>>();
				// println!("Running with args {:?}", args);
				let child = CommandConfig::new("deno")
					.arg("--allow-read")
					.arg("--allow-net")
					.arg("--allow-env")
					.arg(deno_runner_path().to_string_lossy())
					.env("SWEET_ROOT", std::env::var("SWEET_ROOT")?)
					.args(args);
				cmd_runner.run(ev, child)
			},
		),
	)
}

fn sweet_target_dir() -> PathBuf {
	let mut path = workspace_root();
	path.push("target/sweet");
	path
}

fn deno_runner_path() -> PathBuf {
	let mut path = sweet_target_dir();
	path.push("deno.ts");
	path
}

#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		deno_runner_path()
			.to_string_lossy()
			.replace("\\", "/")
			.xpect_ends_with("target/sweet/deno.ts");
	}
}
