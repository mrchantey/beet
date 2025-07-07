use crate::prelude::*;
use beet_bevy::prelude::When;
use beet_common::node::HtmlConstants;
use beet_fs::cargo::CargoBuildCmd;
use beet_rsx::prelude::*;
use beet_utils::utils::PipelineTarget;
use bevy::prelude::*;
use std::path::Path;
use std::process::Command;

pub fn compile_client(
	_query: Populated<(), Changed<FileExprHash>>,
	html_constants: When<Res<HtmlConstants>>,
	cmd: When<Res<CargoBuildCmd>>,
	manifest: When<Res<CargoManifest>>,
	config: When<Res<WorkspaceConfig>>,
) -> Result {
	let mut cmd = cmd.clone();
	cmd.target = Some("wasm32-unknown-unknown".to_string());
	let exe_path = cmd.exe_path(manifest.package_name());
	cmd.no_default_features = true;
	cmd.push_feature("client");

	debug!("Building client binary");
	cmd.spawn()?;
	wasm_bindgen(&html_constants, &config.html_dir, &exe_path)?;
	if cmd.release {
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
		.status()?
		.exit_ok()?
		.xok()
}

// TODO wasm opt
fn wasm_opt(html_constants: &HtmlConstants, html_dir: &Path) -> Result<()> {
	debug!("Optimizing wasm binary");
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
		.status()?
		.exit_ok()?;

	let size_after = std::fs::metadata(&wasm_file)?.len();
	trace!(
		"Reduced wasm binary from {} to {}",
		format!("{} KB", size_before as usize / 1024),
		format!("{} KB", size_after as usize / 1024)
	);

	Ok(())
}
