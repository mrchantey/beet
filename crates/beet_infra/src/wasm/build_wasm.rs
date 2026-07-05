use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Request params for the [`BuildWasm`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct BuildWasmParams {
	/// Build in release mode and optimize the artifact with `wasm-opt -Oz`.
	release: bool,
	/// The cargo package to build (`--package`). Omit to build the current
	/// directory's crate, a non-workspace `main.bsx` wasm build.
	package: Option<String>,
	/// The binary target to build (`--bin`).
	bin: Option<String>,
	/// The example target to build, ie `--example my_scene`, overriding `bin`.
	example: Option<String>,
	/// Comma-separated features to activate (`--features`).
	features: Option<String>,
	/// Activate all features (`--all-features`), overriding `features`.
	all_features: bool,
	/// Also activate the crate's `default` feature. Off by default, so the build is
	/// `--no-default-features` like the wasm-safe `web`/`cloudflare` targets.
	default_features: bool,
	/// The output `.wasm` path (required); the sibling `.js` is written alongside
	/// it. The `wasm-bindgen` `<name>_bg.wasm`/`<name>.js` pair is renamed to these
	/// exact names, eg `--out=dist/wasm/min.wasm` yields `min.wasm` + `min.js`.
	out: Option<String>,
}

/// Compiles a package to wasm and prepares it for the browser.
///
/// Target-agnostic: the invoking entry (a `main.bsx`, a justfile recipe) supplies
/// the package, features and `--out` path, so nothing here defaults to a beet
/// binary. With no `--package` it builds the current directory's crate (a
/// non-workspace `main.bsx` build).
///
/// Runs `cargo build --target wasm32-unknown-unknown` (`--no-default-features`
/// plus the selected features), then `wasm-bindgen --target web`, then in release
/// `wasm-opt -Oz`, and finally renames the `<name>_bg.wasm`/`<name>.js` pair to
/// the exact `--out` names (patching the glue's wasm URL to match), returning the
/// artifact size.
#[action(route = "build-wasm", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<BuildWasmParams>())]
pub async fn BuildWasm(parts: RequestParts) -> Result<String> {
	let params = parts.params().parse_reflect::<BuildWasmParams>()?;

	// the cargo build, fully driven by the invoking entry's args.
	let mut cargo = CargoBuild::default()
		.with_release(params.release)
		.with_target(BuildTarget::Wasm)
		.with_no_default_features(!params.default_features)
		.with_all_features(params.all_features);
	// no `--package` builds the current directory's crate (a non-workspace build).
	if let Some(package) = &params.package {
		cargo = cargo.with_package(package.as_str());
	}
	if !params.all_features
		&& let Some(features) = &params.features
	{
		cargo = cargo.with_features(
			features
				.split(',')
				.filter(|feature| !feature.is_empty())
				.map(SmolStr::from)
				.collect(),
		);
	}
	// an `--example` target overrides `--bin`
	if let Some(example) = &params.example {
		cargo = cargo.with_example(example.as_str());
	} else if let Some(bin) = &params.bin {
		cargo = cargo.with_binary(bin.as_str());
	}

	// the requested artifact names, parsed from the required `--out`: the `.wasm`
	// file, its `.js` sibling, and the interim `wasm-bindgen` `<stem>_bg.wasm`.
	let out_raw = params.out.as_deref().ok_or_else(|| {
		bevyhow!("--out is required, eg --out=assets/wasm/beet-min.wasm")
	})?;
	let out_path = std::path::Path::new(out_raw);
	let stem = out_path
		.file_stem()
		.and_then(|stem| stem.to_str())
		.ok_or_else(|| bevyhow!("--out `{out_raw}` has no file stem"))?
		.to_string();
	let wasm_name = out_path
		.file_name()
		.and_then(|name| name.to_str())
		.ok_or_else(|| bevyhow!("--out `{out_raw}` has no file name"))?
		.to_string();
	let dir_raw = out_path
		.parent()
		.map(|dir| dir.to_string_lossy().to_string())
		.unwrap_or_default();
	let out_dir = AbsPathBuf::new(&dir_raw)
		.unwrap_or_else(|_| AbsPathBuf::new_unchecked(&dir_raw));
	let bindgen_wasm = out_dir.join(format!("{stem}_bg.wasm"));
	let out_wasm = out_dir.join(&wasm_name);
	let out_js = out_dir.join(format!("{stem}.js"));

	// 1. cargo build
	ChildProcess::new("cargo")
		.with_args(cargo.cargo_args())
		.run_async()
		.await?;

	// 2. wasm-bindgen → <out_dir>/<stem>_bg.wasm + <out_dir>/<stem>.js
	fs_ext::create_dir_all(&out_dir)?;
	ChildProcess::new("wasm-bindgen")
		.with_args([
			"--out-dir".to_string(),
			out_dir.to_string_lossy().to_string(),
			"--out-name".to_string(),
			stem.clone(),
			"--target".to_string(),
			"web".to_string(),
			"--no-typescript".to_string(),
			cargo.exe_path().to_string_lossy().to_string(),
		])
		.run_async()
		.await?;

	// 3. wasm-opt (release only), in place over the bindgen output
	if cargo.release {
		ChildProcess::new("wasm-opt")
			.with_args([
				"-Oz".to_string(),
				"--output".to_string(),
				bindgen_wasm.to_string_lossy().to_string(),
				bindgen_wasm.to_string_lossy().to_string(),
			])
			.run_async()
			.await?;
	}

	// 4. rename `<stem>_bg.wasm` → the requested `.wasm` and patch the glue's
	// `new URL('<stem>_bg.wasm', import.meta.url)` reference to match, so the
	// `<name>.wasm`/`<name>.js` pair is self-consistent for a static load.
	if bindgen_wasm != out_wasm {
		fs_ext::write(&out_wasm, fs_ext::read(&bindgen_wasm)?)?;
		fs_ext::remove(&bindgen_wasm)?;
		let glue = fs_ext::read_to_string(&out_js)?
			.replace(&format!("{stem}_bg.wasm"), &wasm_name);
		fs_ext::write(&out_js, glue)?;
	}

	let size_kb = std::fs::metadata(&out_wasm)
		.map(|meta| meta.len() as usize / 1024)
		.unwrap_or(0);
	let report = format!("🌱 wasm size: {size_kb} KB ({wasm_name})");
	info!("{report}");
	Ok(report)
}
