use beet::prelude::*;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

/// The bundled Deno wasm runner script, written alongside the `wasm-bindgen`
/// output so its relative `./bindgen.js` import resolves.
const DENO_TS: &str = include_str!("deno.ts");

/// The runner's own params, distinct from the [`BootstrapConfig`] it hands the
/// module: these configure *this* process's hosting of the module, so the module
/// never sees them.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct RunWasmParams {
	/// The host to execute the module in: `deno` (default) or `browser`. Also
	/// read from `BEET_WASM_HOST`, the only channel cargo leaves open to a
	/// configured `runner` (see the justfile's browser test recipe).
	wasm_host: Option<SmolStr>,
	/// Extra flags for the browser host's chromium session, eg
	/// `--chrome-args='--enable-unsafe-webgpu --use-angle=gl'`. Repeatable, and
	/// ignored by the deno host.
	chrome_args: Vec<String>,
}

impl RunWasmParams {
	/// The selected host, `deno` unless `--wasm-host` or `BEET_WASM_HOST` names
	/// otherwise.
	fn host(&self) -> Result<WasmHost> {
		self.wasm_host
			.clone()
			.or_else(|| env_ext::var("BEET_WASM_HOST").ok())
			.map(|host| host.parse::<WasmHost>())
			.transpose()?
			.unwrap_or_default()
			.xok()
	}

	/// The browser host's extra chromium flags, split on whitespace so one flag
	/// can carry several.
	fn chrome_args(&self) -> Vec<String> {
		self.chrome_args
			.iter()
			.flat_map(|value| value.split_whitespace())
			.map(str::to_string)
			.collect()
	}
}

/// The host `beet run-wasm` executes a wasm module in.
///
/// ## Example
///
/// ```
/// # use beet::prelude::*;
/// # use beet_cli::prelude::*;
/// let host: WasmHost = "browser".parse().unwrap();
/// host.xpect_eq(WasmHost::Browser);
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum WasmHost {
	/// The bundled deno runner: a js runtime with fs access and no dom.
	#[default]
	Deno,
	/// A headless browser driven through the webdriver: the host for
	/// `#[beet_core::test(browser)]` dom suites.
	Browser,
}

impl FromStr for WasmHost {
	type Err = BevyError;
	fn from_str(value: &str) -> Result<Self> {
		match value.to_lowercase().as_str() {
			"deno" => Ok(WasmHost::Deno),
			"browser" => Ok(WasmHost::Browser),
			other => bevybail!(
				"invalid --wasm-host `{other}`, expected `deno` or `browser`"
			),
		}
	}
}

impl RunWasm {
	/// The positional this command answers to. The runner hosts the module in a
	/// child runtime, so the `--main`/`--repo` on its argv belong to the module
	/// rather than to this process: the binary declares it as an
	/// [`ArgvPassthrough`] so the entry loader leaves those flags alone.
	pub const COMMAND: &'static str = "run-wasm";
}

impl fmt::Display for WasmHost {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			WasmHost::Deno => write!(f, "deno"),
			WasmHost::Browser => write!(f, "browser"),
		}
	}
}

/// Runs a `wasm32-unknown-unknown` binary via `wasm-bindgen` + the bundled Deno
/// runner, or with `BEET_WASM_HOST=browser` via a headless chromium (see
/// `run_wasm_browser.rs`, the host for `#[beet_core::test(browser)]` dom
/// tests). Wire it up as the cargo runner:
///
/// ```toml
/// # .cargo/config.toml
/// [target.wasm32-unknown-unknown]
/// runner = "beet run-wasm"
/// ```
///
/// As a route it is served greedily (`run-wasm/*run-wasm-args`): the segments after
/// `run-wasm` rejoin into the binary path, and any query params are forwarded to
/// the running module as flags — the beet wasm test runner reads them back via
/// `Deno.args`. cargo passes an absolute binary path, which `CliArgs` keeps as
/// one intact segment, so the rejoin reproduces it leading-`/` and all. The capture
/// is named `run-wasm-args` to distinguish the route's own path capture from the
/// module args it forwards.
#[action(route = "run-wasm/*run-wasm-args", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<RunWasmParams>())]
pub async fn RunWasm(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();
	let mut cli = parts.to_cli_args();
	// route `run-wasm/*run-wasm-args`: the first segment is the command, the rest
	// are the absolute binary path (split on `/`) followed by any positional module
	// args (eg a libtest name filter cargo appends after the binary).
	let mut segments = core::mem::take(&mut cli.path);
	if segments.is_empty() {
		bevybail!("usage: beet run-wasm <binary-path> [args..]");
	}
	segments.remove(0);
	// the binary path ends at the `.wasm` segment; segments after it are positional
	// module args (the filter), so the path is not over-joined into them.
	let wasm_idx = segments
		.iter()
		.position(|segment| segment.ends_with(".wasm"))
		.ok_or_else(|| {
			bevyhow!("usage: beet run-wasm <binary-path>.wasm [args..]")
		})?;
	let trailing = segments.split_off(wasm_idx + 1);
	let exe_path = segments.join("/");
	// `cli` now holds only the forwarded params; drop the route's own `run-wasm-args`
	// capture so it never leaks into the module's `Deno.args`. The runner's own
	// params and the module's launch knobs are both taken out (the latter
	// delivered explicitly, see [`run_deno`]); what remains (the test-runner
	// flags) flattens to `--key[=value]` flags plus the trailing positionals (eg
	// the filter) the module reads via `Deno.args`.
	cli.params.remove("run-wasm-args");
	let params = cli.params.parse_reflect::<RunWasmParams>()?;
	let host = params.host()?;
	let chrome_args = params.chrome_args();
	cli.params.remove("wasm-host");
	cli.params.remove("chrome-args");
	let bootstrap = BootstrapConfig::take_params(&mut cli.params)?;
	let mut forwarded = cli.into_args();
	forwarded.extend(trailing.into_iter().map(Into::into));

	let runner_dir = bindgen_wasm(Path::new(&exe_path)).await?;
	// `--wasm-host=browser` / `BEET_WASM_HOST=browser`: serve the bindgen
	// output and run the module in a headless browser instead of deno, the
	// host for `#[beet_core::test(browser)]` dom suites
	#[cfg(not(target_arch = "wasm32"))]
	if host == WasmHost::Browser {
		super::run_wasm_browser::run(
			&cx,
			&runner_dir,
			&bootstrap,
			forwarded,
			chrome_args,
		)
		.await?;
		return Response::ok().xok();
	}
	// the browser host (and so the flags' consumer) is native-only
	#[cfg(target_arch = "wasm32")]
	let _ = (host, chrome_args);
	run_deno(&runner_dir, &bootstrap, forwarded).await?;
	// the module's output already streamed via inherited stdio, so the runner's own
	// response carries no body, only a success status.
	Response::ok().xok()
}

/// The directory the runner artifacts (`bindgen*.js`, `deno.ts`) are written to.
fn runner_dir() -> PathBuf {
	fs_ext::workspace_root().join("target/wasm-runner")
}

/// Runs `wasm-bindgen`, returning the runner dir holding `bindgen*.js` +
/// `*_bg.wasm`, the input both hosts serve or import.
async fn bindgen_wasm(exe_path: &Path) -> Result<PathBuf> {
	if !fs_ext::exists(exe_path)? {
		bevybail!("wasm binary does not exist: {}", exe_path.display());
	}
	let runner_dir = runner_dir();
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
	Ok(runner_dir)
}

/// Writes the bundled Deno runner beside the bindgen output and executes the
/// module, forwarding `args` and inheriting stdio so the module's output
/// streams live.
///
/// The module's own [`BootstrapConfig`] is delivered explicitly through
/// [`ChildProcess::with_bootstrap`], which also scrubs the runner's inherited
/// `BEET_*` env: beet spawning beet never passes config ambiently, so a wasm test
/// isolate cannot pick up the host's stage, store or ports by accident.
async fn run_deno(
	runner_dir: &Path,
	bootstrap: &BootstrapConfig,
	args: Vec<String>,
) -> Result {
	// write the bundled Deno runner next to the bindgen output
	assert_deno_installed().await?;
	fs_ext::create_dir_all_async(&runner_dir).await?;
	fs_ext::write_async(runner_dir.join("deno.ts"), DENO_TS).await?;

	// 4. deno run <runner> <args>, inheriting stdio so the module's output
	// (test results, panics, logs) streams to the terminal live and its exit
	// code propagates — essential for a cargo runner.
	let mut deno_args = vec![
		"--allow-read".to_string(),
		// `FsStore` is cross-platform and writes through the runner's fs globals
		// (`Deno.writeFileSync`/`ensureDirSync`), so its `insert`/`store_create`
		// need write access under the runner.
		"--allow-write".to_string(),
		"--allow-net".to_string(),
		"--allow-env".to_string(),
		// the `Script` worker backend spawns each eval in a `permissions: "none"`
		// Worker, which deno still gates behind this flag. Without it the
		// permission descriptor is rejected and the isolate would inherit the
		// runner's own (wide) authority, so the backend refuses to run rather
		// than silently downgrading.
		"--unstable-worker-options".to_string(),
		runner_dir.join("deno.ts").to_string_lossy().to_string(),
	];
	deno_args.extend(args);
	let status = ChildProcess::new("deno")
		.with_envs([(
			"WORKSPACE_ROOT",
			fs_ext::workspace_root().to_string_lossy().to_string(),
		)])
		.with_args(deno_args)
		// appended last, so the config lands among the module's own args
		.with_bootstrap(bootstrap)?
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
