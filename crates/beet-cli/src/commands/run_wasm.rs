use beet::prelude::*;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

/// The bundled Deno wasm runner script, written alongside the `wasm-bindgen`
/// output so its relative `./bindgen.js` import resolves.
const DENO_TS: &str = include_str!("deno.ts");

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

impl WasmHost {
	/// The selected host: `wasm_host` (the `--wasm-host` param) else
	/// `BEET_WASM_HOST`, the only channel cargo leaves open to a configured
	/// `runner` (see the justfile's browser test recipe), else `deno`.
	fn resolve(wasm_host: Option<SmolStr>) -> Result<Self> {
		wasm_host
			.or_else(|| env_ext::var("BEET_WASM_HOST").ok())
			.map(|host| host.parse::<Self>())
			.transpose()?
			.unwrap_or_default()
			.xok()
	}
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
	/// child runtime, so the `--entry`/`--repo` on its argv belong to the module
	/// rather than to this process: the binary declares it as an
	/// [`ArgvPassthrough`] so the entry loader leaves those flags alone.
	pub const COMMAND: &'static str = "run-wasm";

	/// The browser host's extra chromium flags, each `--chrome-args` value
	/// split on whitespace so one flag can carry several.
	fn chrome_flags(chrome_args: Vec<String>) -> Vec<String> {
		chrome_args
			.iter()
			.flat_map(|value| value.split_whitespace())
			.map(str::to_string)
			.collect()
	}

	/// The module's argv: every param `pattern` does not declare (the runner's
	/// own `--wasm-host`/`--chrome-args`, an inherited `--help` and the like)
	/// as a `--key[=value]` flag, then the `trailing` positionals (eg a libtest
	/// filter). The route's own `run-wasm-args` capture is dropped so it never
	/// leaks into `Deno.args`, and so is every launch knob: a launch only one
	/// never reaches the request, and a request rank one (`--port`) reaches the
	/// module through its delivered config rather than twice.
	fn module_args(
		any: &AnyParams,
		pattern: &ParamsPattern,
		trailing: &[SmolStr],
	) -> Vec<String> {
		let mut params = any.undeclared(pattern);
		params.remove("run-wasm-args");
		for knob in BootstrapConfig::arg_names() {
			params.remove(knob.as_str());
		}
		let mut args = CliArgs {
			path: default(),
			params,
		}
		.into_args();
		args.extend(trailing.iter().map(ToString::to_string));
		args
	}
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
/// `run-wasm` rejoin into the binary path, and any params are forwarded to the
/// running module as flags, which the beet wasm test runner reads back via
/// `Deno.args`. cargo passes an absolute binary path, which `CliArgs` keeps as
/// one intact segment, so the rejoin reproduces it leading-`/` and all. The capture
/// is named `run-wasm-args` to distinguish the route's own path capture from the
/// module args it forwards.
///
/// It takes any param ([`AnyParams`]): a param its route does not declare
/// belongs to the module, ie the test runner's `--nocapture`, so a strict CLI
/// refuses none. The module's launch knobs are this process's own
/// [`BootstrapConfig`], delivered through [`ChildProcess::with_bootstrap`]
/// rather than forwarded as flags.
#[action(route = "run-wasm/*run-wasm-args")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn RunWasm(
	/// The host to execute the module in: `deno` (default) or `browser`. Also
	/// read from `BEET_WASM_HOST`.
	#[param]
	wasm_host: Option<SmolStr>,
	/// Extra flags for the browser host's chromium session, eg
	/// `--chrome-args='--enable-unsafe-webgpu --use-angle=gl'`. Repeatable, and
	/// ignored by the deno host.
	#[param]
	chrome_args: Vec<String>,
	any: AnyParams,
	cx: ActionContext<RequestParts>,
) -> Result<Response> {
	// route `run-wasm/*run-wasm-args`: the first segment is the command, the rest
	// are the absolute binary path (split on `/`) followed by any positional module
	// args (eg a libtest name filter cargo appends after the binary).
	let Some((_, segments)) = cx.input.path().split_first() else {
		return Err(HttpError::bad_request(
			"usage: beet run-wasm <binary-path> [args..]",
		)
		.into());
	};
	// the binary path ends at the `.wasm` segment; segments after it are positional
	// module args (the filter), so the path is not over-joined into them.
	let wasm_idx = segments
		.iter()
		.position(|segment| segment.ends_with(".wasm"))
		.ok_or_else(|| {
			HttpError::bad_request(
				"usage: beet run-wasm <binary-path>.wasm [args..]",
			)
		})?;
	let exe_path = segments[..=wasm_idx].join("/");
	let trailing = &segments[wasm_idx + 1..];

	let pattern = cx
		.caller
		.with_state::<ParamsQuery, _>(|entity, query| query.of(entity))
		.await??;
	let forwarded = RunWasm::module_args(&any, &pattern, trailing);
	let bootstrap = BootstrapConfig::get();
	let host = WasmHost::resolve(wasm_host)?;

	let runner_dir = bindgen_wasm(Path::new(&exe_path)).await?;
	// `--wasm-host=browser` / `BEET_WASM_HOST=browser`: serve the bindgen
	// output and run the module in a headless browser instead of deno, the
	// host for `#[beet_core::test(browser)]` dom suites
	if host == WasmHost::Browser {
		super::run_wasm_browser::run(
			&cx,
			&runner_dir,
			bootstrap,
			forwarded,
			RunWasm::chrome_flags(chrome_args),
		)
		.await?;
		return Response::ok().xok();
	}
	run_deno(&runner_dir, bootstrap, forwarded).await?;
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
			fs_ext::WORKSPACE_ROOT,
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

#[cfg(test)]
mod test {
	use super::*;

	/// The module receives only what the route does not declare: the runner's
	/// own flags, its path capture and the launch knobs stay behind, the
	/// test-runner flags and the trailing filter go through.
	#[beet::test]
	fn forwards_only_the_modules_args() {
		let mut world = crate::commands::render_world();
		let entity = world.spawn(RunWasm).flush();
		let pattern = world
			.with_state::<ParamsQuery, _>(|query| query.of(entity))
			.unwrap();
		let parts = RequestParts::from(CliArgs::parse(
			"run-wasm --nocapture --wasm-host=deno --chrome-args=--headless --port=8",
		))
		.with_param("run-wasm-args", "/tmp/module.wasm");
		RunWasm::module_args(&AnyParams::from(&parts), &pattern, &[
			"my_filter".into(),
		])
		.xpect_eq(vec!["--nocapture".to_string(), "my_filter".to_string()]);
	}
}
