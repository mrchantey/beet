use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_flow::prelude::*;
use beet_rsx::prelude::*;

#[construct]
pub fn BuildWasm(
	query: AncestorQuery<&'static CargoBuildCmd>,
) -> Result<impl Bundle> {
	let cmd = query
		.get(entity)
		.cloned()
		.unwrap_or_default()
		.target("wasm32-unknown-unknown")
		.no_default_features()
		.feature("client");

	// wasm_opt(&cmd, &html_constants, &config.html_dir)?;

	(Name::new("Build Wasm"), Sequence, children![
		ChildProcess::from_cargo(&cmd),
		WasmBindgen,
		WasmOpt
	])
		.xok()
}

/// execute `wasm-bindgen` with inferred locations. The wasm_exe_path
/// should be the path to the output of `cargo build`
#[construct]
fn WasmBindgen(
	html_constants: Res<HtmlConstants>,
	manifest: Res<CargoManifest>,
	config: Res<WorkspaceConfig>,
	query: AncestorQuery<&'static CargoBuildCmd>,
) -> impl Bundle {
	let exe_path = query
		.get(entity)
		.cloned()
		.unwrap_or_default()
		.target("wasm32-unknown-unknown")
		.no_default_features()
		.feature("client")
		.exe_path(manifest.package_name());

	(
		Name::new("Wasm Bindgen"),
		ChildProcess::new("wasm-bindgen")
			.arg("--out-dir")
			.arg(
				config
					.html_dir
					.join(&html_constants.wasm_dir)
					.to_string_lossy(),
			)
			.arg("--out-name")
			.arg(&html_constants.wasm_name)
			.arg("--target")
			.arg("web")
			// alternatively es modules target: experimental-nodejs-module
			.arg("--no-typescript")
			.arg(exe_path.to_string_lossy()),
	)
}

#[construct]
fn WasmOpt(
	html_constants: Res<HtmlConstants>,
	config: Res<WorkspaceConfig>,
	query: AncestorQuery<&'static CargoBuildCmd>,
) -> impl Bundle {
	let cmd = query
		.get(entity)
		.cloned()
		.unwrap_or_default()
		.target("wasm32-unknown-unknown")
		.no_default_features();

	let wasm_file = config.html_dir.join(format!(
		"{}/{}_bg.wasm",
		&html_constants.wasm_dir.display(),
		&html_constants.wasm_name
	));

	let process = if cmd.release {
		ChildProcess::new("wasm-opt")
			.arg("-Oz")
			.arg("--output")
			.arg(wasm_file.to_string_lossy())
			.arg(wasm_file.to_string_lossy())
	} else {
		ChildProcess::default()
	};


	// let size_before = std::fs::metadata(&wasm_file)?.len();
	(Name::new("Wasm Opt"), process)

	// let size_after = std::fs::metadata(&wasm_file)?.len();
	// debug!(
	// 	"🌱 Reduced wasm binary from {} to {}",
	// 	format!("{} KB", size_before as usize / 1024),
	// 	format!("{} KB", size_after as usize / 1024)
	// );
}
