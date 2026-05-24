use beet_core::prelude::*;
use std::path::Path;
use std::path::PathBuf;

/// The bundled Deno wasm runner script.
const DENO_TS: &str = include_str!("deno.ts");

/// Output directory for the wasm runner artifacts.
fn wasm_runner_dir() -> PathBuf {
	fs_ext::workspace_root().join("target/wasm-runner")
}

/// Path to the cached Deno runner script.
fn deno_runner_path() -> PathBuf { wasm_runner_dir().join("deno.ts") }

/// Runs a `wasm32-unknown-unknown` binary via `wasm-bindgen` + the bundled Deno
/// runner. Wire it up as the cargo runner:
///
/// ```toml
/// # .cargo/config.toml
/// [target.wasm32-unknown-unknown]
/// runner = "beet run-wasm"
/// ```
///
/// `args` are forwarded to the running module.
pub async fn run_wasm(
	exe_path: impl AsRef<Path>,
	args: Vec<String>,
) -> Result<()> {
	let exe_path = exe_path.as_ref();
	if !fs_ext::exists(exe_path)? {
		bevybail!("wasm binary does not exist: {}", exe_path.display());
	}

	// 1. wasm-bindgen → target/wasm-runner/bindgen*.js + *_bg.wasm
	let runner_dir = wasm_runner_dir();
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

	// 2. ensure the Deno runner script is present and current
	ensure_deno_runner().await?;

	// 3. deno run <runner> <args>
	let mut deno_args = vec![
		"--allow-read".to_string(),
		"--allow-net".to_string(),
		"--allow-env".to_string(),
		deno_runner_path().to_string_lossy().to_string(),
	];
	deno_args.extend(args);
	ChildProcess::new("deno")
		.with_envs([(
			"WORKSPACE_ROOT",
			fs_ext::workspace_root().to_string_lossy().to_string(),
		)])
		.with_args(deno_args)
		.run_async()
		.await?;
	Ok(())
}

/// Writes the bundled Deno runner script to the cache if missing or stale,
/// erroring with install instructions if Deno is not available.
async fn ensure_deno_runner() -> Result<()> {
	let path = deno_runner_path();
	if fs_ext::exists(&path)?
		&& fs_ext::hash_file(&path)? == fs_ext::hash_string(DENO_TS)
	{
		return Ok(());
	}

	let deno_installed = ChildProcess::new("deno")
		.with_args(["--version"])
		.run_async()
		.await
		.is_ok();
	if !deno_installed {
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

	fs_ext::create_dir_all_async(wasm_runner_dir()).await?;
	fs_ext::write_async(&path, DENO_TS).await?;
	Ok(())
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn runner_path() {
		deno_runner_path()
			.to_string_lossy()
			.replace('\\', "/")
			.xpect_ends_with("target/wasm-runner/deno.ts");
	}
}
