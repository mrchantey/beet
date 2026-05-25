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
/// As a route it is served greedily (`run-wasm/*args`): the first segment after
/// `run-wasm` is the binary, and the remaining positional segments and query
/// params are forwarded to the running module — the beet wasm test runner reads
/// them back via `Deno.args`.
#[action]
#[derive(Component)]
pub async fn RunWasm(parts: RequestParts) -> Result<String> {
	// rebuilds `[run-wasm, <binary>, ..forwarded]`; skip the `run-wasm`
	// command consumed by the route, pop the binary path, forward the rest.
	let mut args = parts.unparse_cli_args().into_iter().skip(1);
	let exe_path = args
		.next()
		.ok_or_else(|| bevyhow!("usage: beet run-wasm <binary-path> [args..]"))?;
	run_wasm(Path::new(&exe_path), args.collect()).await?;
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
