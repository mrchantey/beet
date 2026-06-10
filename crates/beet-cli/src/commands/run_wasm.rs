use beet::prelude::*;
use std::path::Path;
use std::path::PathBuf;

/// The bundled Deno wasm runner script, written alongside the `wasm-bindgen`
/// output so its relative `./bindgen.js` import resolves.
const DENO_TS: &str = include_str!("deno.ts");

/// Runs a `wasm32-unknown-unknown` binary via `wasm-bindgen` + the bundled Deno
/// runner. Wire it up as the cargo runner:
///
/// ```toml
/// # .cargo/config.toml
/// [target.wasm32-unknown-unknown]
/// runner = "beet run-wasm"
/// ```
///
/// As a route it is served greedily (`run-wasm/*args`): the path segments after
/// `run-wasm` rejoin into the (absolute) binary path, and any query params are
/// forwarded to the running module as flags — the beet wasm test runner reads
/// them back via `Deno.args`. Rejoining is necessary because cargo passes an
/// absolute path whose `/` separators split into several route segments.
#[action(route = "run-wasm/*args", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn RunWasm(parts: RequestParts) -> Result<String> {
	// segments after the `run-wasm` command rejoin into the absolute binary path.
	let segments = parts.path_from(1);
	if segments.is_empty() {
		bevybail!("usage: beet run-wasm <binary-path> [args..]");
	}
	let exe_path = format!("/{}", segments.join("/"));
	// forwarded flags arrive as query params, re-emitted as `--key[=value]`.
	let forwarded = parts
		.params()
		.iter_all()
		.flat_map(|(key, values)| match values.is_empty() {
			true => vec![format!("--{key}")],
			false => values.iter().map(|value| format!("--{key}={value}")).collect(),
		})
		.collect();
	run_wasm(Path::new(&exe_path), forwarded).await?;
	// the module's output already streamed via inherited stdio
	Ok(String::new())
}

/// The directory the runner artifacts (`bindgen*.js`, `deno.ts`) are written to.
fn runner_dir() -> PathBuf {
	fs_ext::workspace_root().join("target/wasm-runner")
}

/// Runs `wasm-bindgen`, writes the Deno runner, then executes the module,
/// forwarding `args` and inheriting stdio so the module's output streams live.
async fn run_wasm(exe_path: &Path, args: Vec<String>) -> Result {
	if !fs_ext::exists(exe_path)? {
		bevybail!("wasm binary does not exist: {}", exe_path.display());
	}
	let runner_dir = runner_dir();

	// 1. wasm-bindgen → target/wasm-runner/bindgen*.js + *_bg.wasm
	ChildProcess::new("wasm-bindgen")
		.with_args([
			"--out-dir".to_string(),
			runner_dir.to_string_lossy().to_string(),
			"--out-name".to_string(),
			"bindgen".to_string(),
			"--target".to_string(),
			"web".to_string(),
			"--no-typescript".to_string(),
			exe_path.to_string_lossy().to_string(),
		])
		.run_async()
		.await?;

	// 2. write the bundled Deno runner next to the bindgen output
	assert_deno_installed().await?;
	fs_ext::create_dir_all_async(&runner_dir).await?;
	fs_ext::write_async(runner_dir.join("deno.ts"), DENO_TS).await?;

	// 3. deno run <runner> <args>, inheriting stdio so the module's output
	// (test results, panics, logs) streams to the terminal live and its exit
	// code propagates — essential for a cargo runner.
	let mut deno_args = vec![
		"--allow-read".to_string(),
		"--allow-net".to_string(),
		"--allow-env".to_string(),
		runner_dir.join("deno.ts").to_string_lossy().to_string(),
	];
	deno_args.extend(args);
	let status = ChildProcess::new("deno")
		.with_envs([(
			"WORKSPACE_ROOT",
			fs_ext::workspace_root().to_string_lossy().to_string(),
		)])
		.with_args(deno_args)
		.spawn()?
		.status()
		.await?;
	if !status.success() {
		bevybail!("wasm module exited with {status}");
	}
	Ok(())
}

/// Errors with install instructions if Deno is not available.
async fn assert_deno_installed() -> Result {
	let installed = ChildProcess::new("deno")
		.with_args(["--version"])
		.run_async()
		.await
		.is_ok();
	if !installed {
		bevybail!(
			"
🦖 Beet uses Deno for the wasm runner 🦖

Install Deno via:
shell:      curl -fsSL https://deno.land/install.sh | sh
powershell: irm https://deno.land/install.ps1 | iex
website:    https://docs.deno.com/runtime/getting_started/installation/
"
		);
	}
	Ok(())
}
