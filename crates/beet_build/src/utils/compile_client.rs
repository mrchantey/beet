use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use std::path::Path;
use std::process::Command;

pub fn compile_client(
	_query: Populated<(), Changed<FileExprHash>>,
	html_constants: Res<HtmlConstants>,
	cmd: Res<CargoBuildCmd>,
	manifest: Res<CargoManifest>,
	config: Res<WorkspaceConfig>,
	pkg_config: Res<PackageConfig>,
) -> Result {
	let build_cmd = cmd
		.clone()
		.target("wasm32-unknown-unknown")
		.no_default_features()
		.feature("client");

	Command::new("cargo")
		.args(build_cmd.get_args())
		.envs(pkg_config.envs())
		.xtap(|cmd| {
			debug!("🌱 Building client binary\n{:?}", cmd);
		})
		.status()?
		.exit_ok()?;

	let exe_path = build_cmd.exe_path(manifest.package_name());

	wasm_bindgen(&html_constants, &config.html_dir, &exe_path)?;
	if build_cmd.release {
		wasm_opt(&html_constants, &config.html_dir)?;
	}
	Ok(())
}

/// execute `wasm-bindgen` with inferred locations. The wasm_exe_path
/// should be the path to the output of `cargo build`
fn wasm_bindgen(
	html_constants: &HtmlConstants,
	html_dir: &Path,
	exe_path: &Path,
) -> Result<()> {
	Command::new("wasm-bindgen")
		.arg("--out-dir")
		.arg(html_dir.join(&html_constants.wasm_dir))
		.arg("--out-name")
		.arg(&html_constants.wasm_name)
		.arg("--target")
		.arg("web")
		// alternatively es modules target: experimental-nodejs-module
		.arg("--no-typescript")
		.arg(&exe_path)
		.xtap(|cmd| {
			debug!("🌱 Running wasm-bindgen\n{:?}", cmd);
		})
		.status()?
		.exit_ok()?
		.xok()
}

// TODO wasm opt
fn wasm_opt(html_constants: &HtmlConstants, html_dir: &Path) -> Result<()> {
	let wasm_file = html_dir.join(format!(
		"{}/{}_bg.wasm",
		&html_constants.wasm_dir.display(),
		&html_constants.wasm_name
	));

	let size_before = std::fs::metadata(&wasm_file)?.len();

	Command::new("wasm-opt")
		.arg("-Oz")
		.arg("--output")
		.arg(&wasm_file)
		.arg(&wasm_file)
		.xtap(|cmd| {
			debug!("🌱 Optimizing wasm binary\n{:?}", cmd);
		})
		.status()?
		.exit_ok()?;

	let size_after = std::fs::metadata(&wasm_file)?.len();
	debug!(
		"🌱 Reduced wasm binary from {} to {}",
		format!("{} KB", size_before as usize / 1024),
		format!("{} KB", size_after as usize / 1024)
	);

	Ok(())
}
