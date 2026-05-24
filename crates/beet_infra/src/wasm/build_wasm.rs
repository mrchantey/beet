use crate::prelude::*;
use beet_core::prelude::*;

/// Compiles a package to wasm and prepares it for the browser.
///
/// Runs `cargo build --target wasm32-unknown-unknown --no-default-features
/// --features client`, then `wasm-bindgen` into `<out_dir>/<wasm_dir>`, and in
/// release mode `wasm-opt -Oz` over the result, logging the artifact size.
pub async fn build_wasm(
	cargo: CargoBuildCmd,
	pkg_name: Option<String>,
	out_dir: AbsPathBuf,
	constants: &HtmlConstants,
) -> Result<()> {
	let cargo = cargo
		.cmd("build")
		.target("wasm32-unknown-unknown")
		.no_default_features()
		.feature("client");

	// 1. cargo build
	ChildProcess::new(cargo.program.clone())
		.with_args(cargo.get_args().into_iter().map(str::to_string))
		.run_async()
		.await?;

	// 2. wasm-bindgen
	let exe_path = cargo.exe_path(pkg_name.as_deref());
	let wasm_out = out_dir.join(constants.wasm_dir.to_string());
	ChildProcess::new("wasm-bindgen")
		.with_args([
			"--out-dir".to_string(),
			wasm_out.to_string_lossy().to_string(),
			"--out-name".to_string(),
			constants.wasm_name.clone(),
			"--target".to_string(),
			"web".to_string(),
			"--no-typescript".to_string(),
			exe_path.to_string_lossy().to_string(),
		])
		.run_async()
		.await?;

	// 3. wasm-opt (release only)
	let wasm_file = wasm_out.join(format!("{}_bg.wasm", constants.wasm_name));
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

	if let Ok(meta) = std::fs::metadata(&wasm_file) {
		info!("🌱 wasm size: {} KB", meta.len() as usize / 1024);
	}
	Ok(())
}
