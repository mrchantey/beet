use beet::prelude::*;
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
/// [`RunWasm::args`] are forwarded verbatim to the running module, which is why
/// `main` dispatches this before the router (the router would parse them as
/// request params).
#[derive(Debug, Clone, Component)]
#[require(RunWasmAction)]
pub struct RunWasm {
	/// Path to the compiled `wasm32-unknown-unknown` binary.
	pub exe_path: PathBuf,
	/// Arguments forwarded to the running module.
	pub args: Vec<String>,
}

impl RunWasm {
	/// Creates a runner for `exe_path` with no forwarded arguments.
	pub fn new(exe_path: impl Into<PathBuf>) -> Self {
		Self {
			exe_path: exe_path.into(),
			args: Vec::new(),
		}
	}

	/// Sets the arguments forwarded to the running module.
	pub fn with_args(mut self, args: Vec<String>) -> Self {
		self.args = args;
		self
	}

	/// The directory the runner artifacts (`bindgen*.js`, `deno.ts`) are written to.
	fn runner_dir() -> PathBuf {
		fs_ext::workspace_root().join("target/wasm-runner")
	}

	/// Runs `wasm-bindgen`, writes the Deno runner, then executes the module.
	async fn run(&self) -> Result {
		if !fs_ext::exists(&self.exe_path)? {
			bevybail!("wasm binary does not exist: {}", self.exe_path.display());
		}
		let runner_dir = Self::runner_dir();

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
				self.exe_path.to_string_lossy().to_string(),
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
		deno_args.extend(self.args.iter().cloned());
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
}

/// Reads the [`RunWasm`] state from the caller and runs the module.
///
/// ## Errors
/// Errors if the caller has no [`RunWasm`] component.
#[action]
#[derive(Component)]
pub async fn RunWasmAction(cx: ActionContext) -> Result {
	cx.caller.get_cloned::<RunWasm>().await?.run().await
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
