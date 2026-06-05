use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The `wasm-bindgen` output subdirectory under `out_dir`.
const WASM_DIR: &str = "wasm";
/// The `--out-name` passed to `wasm-bindgen`, ie `main` → `main_bg.wasm`.
const WASM_NAME: &str = "main";

/// Request params for the [`BuildWasm`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct BuildWasmParams {
	/// Build in release mode and optimize the artifact with `wasm-opt -Oz`.
	release: bool,
	/// The cargo package to build, ie `--package my-app`.
	package: Option<String>,
	/// Directory the `wasm-bindgen` output is written under, defaults to `dist`.
	out_dir: Option<String>,
}

/// Compiles a package to wasm and prepares it for the browser.
///
/// Runs `cargo build --target wasm32-unknown-unknown --no-default-features
/// --features client`, then `wasm-bindgen` into `<out_dir>/wasm`, and in release
/// mode `wasm-opt -Oz` over the result, returning the artifact size.
#[action(route = "build-wasm", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<BuildWasmParams>())]
pub async fn BuildWasm(parts: RequestParts) -> Result<String> {
	let params = parts.params().parse_reflect::<BuildWasmParams>()?;
	let mut cargo = CargoBuild::default()
		.with_release(params.release)
		.with_target(BuildTarget::Wasm)
		.with_no_default_features(true)
		.with_features(vec!["client".into()]);
	if let Some(package) = &params.package {
		cargo = cargo.with_package(package.as_str());
	}

	let raw = params.out_dir.as_deref().unwrap_or("dist");
	let out_dir =
		AbsPathBuf::new(raw).unwrap_or_else(|_| AbsPathBuf::new_unchecked(raw));

	// 1. cargo build
	ChildProcess::new("cargo")
		.with_args(cargo.cargo_args())
		.run_async()
		.await?;

	// 2. wasm-bindgen
	let wasm_out = out_dir.join(WASM_DIR);
	ChildProcess::new("wasm-bindgen")
		.with_args([
			"--out-dir".to_string(),
			wasm_out.to_string_lossy().to_string(),
			"--out-name".to_string(),
			WASM_NAME.to_string(),
			"--target".to_string(),
			"web".to_string(),
			"--no-typescript".to_string(),
			cargo.exe_path().to_string_lossy().to_string(),
		])
		.run_async()
		.await?;

	// 3. wasm-opt (release only)
	let wasm_file = wasm_out.join(format!("{WASM_NAME}_bg.wasm"));
	if cargo.release {
		ChildProcess::new("wasm-opt")
			.with_args([
				"-Oz".to_string(),
				"--output".to_string(),
				wasm_file.to_string(),
				wasm_file.to_string(),
			])
			.run_async()
			.await?;
	}

	let size_kb = std::fs::metadata(&wasm_file)
		.map(|meta| meta.len() as usize / 1024)
		.unwrap_or(0);
	let report = format!("🌱 wasm size: {size_kb} KB");
	info!("{report}");
	Ok(report)
}
