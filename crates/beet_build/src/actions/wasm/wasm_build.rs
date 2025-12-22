use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_flow::prelude::*;

pub fn build_wasm() -> impl Bundle {
	(Name::new("Build Wasm"), Sequence, children![
		cargo_build_wasm(),
		wasm_bindgen(),
		wasm_size(false),
		wasm_opt(),
		wasm_size(true),
	])
}

fn cargo_build_wasm() -> impl Bundle {
	(
		Name::new("Cargo Build Wasm"),
		ContinueRun,
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut cmd_runner: CommandRunner,
			      query: AncestorQuery<&'static CargoBuildCmd>| {
				let cmd = query
					.get(ev.action())
					.cloned()
					.unwrap_or_default()
					.cmd("build")
					.target("wasm32-unknown-unknown")
					.no_default_features()
					.feature("client");

				cmd_runner.run(ev, cmd)
			},
		),
	)
}

/// execute `wasm-bindgen` with inferred locations. The wasm_exe_path
/// should be the path to the output of `cargo build`
fn wasm_bindgen() -> impl Bundle {
	(
		Name::new("Wasm Bindgen"),
		ContinueRun,
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut cmd_runner: CommandRunner,
			      html_constants: Res<HtmlConstants>,
			      manifest: Res<CargoManifest>,
			      config: Res<WorkspaceConfig>,
			      query: AncestorQuery<&'static CargoBuildCmd>| {
				let exe_path = query
					.get(ev.action())
					.cloned()
					.unwrap_or_default()
					.target("wasm32-unknown-unknown")
					.no_default_features()
					.feature("client")
					.exe_path(manifest.package_name());

				let cmd_config = CommandConfig::new("wasm-bindgen")
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
					.arg(exe_path.to_string_lossy());

				cmd_runner.run(ev, cmd_config)
			},
		),
	)
}


fn wasm_size(release_only: bool) -> impl Bundle {
	(
		Name::new("Wasm Size"),
		OnSpawn::observe(
			move |mut ev: On<GetOutcome>,
			      html_constants: Res<HtmlConstants>,
			      query: AncestorQuery<&'static CargoBuildCmd>,
			      ws_config: Res<WorkspaceConfig>|
			      -> Result {
				let is_release =
					query.get(ev.action()).cloned().unwrap_or_default().release;
				if release_only && !is_release {
					// noop
				} else {
					let wasm_file = wasm_file(&html_constants, &ws_config);
					let size = std::fs::metadata(&wasm_file)?.len();
					info!("🌱 wasm size: {} KB", size as usize / 1024);
				}

				ev.trigger_with_cx(Outcome::Pass);

				Ok(())
			},
		),
	)
}

fn wasm_file(
	html_constants: &HtmlConstants,
	ws_config: &WorkspaceConfig,
) -> WsPathBuf {
	ws_config.html_dir.join(format!(
		"{}/{}_bg.wasm",
		&html_constants.wasm_dir.display(),
		&html_constants.wasm_name
	))
}

fn wasm_opt() -> impl Bundle {
	(
		Name::new("Wasm Opt"),
		ContinueRun,
		OnSpawn::observe(
			move |mut ev: On<GetOutcome>,
			      mut cmd_runner: CommandRunner,
			      html_constants: Res<HtmlConstants>,
			      ws_config: Res<WorkspaceConfig>,
			      query: AncestorQuery<&'static CargoBuildCmd>|
			      -> Result {
				let cmd = query
					.get(ev.action())
					.cloned()
					.unwrap_or_default()
					.target("wasm32-unknown-unknown")
					.no_default_features();


				// only optimize in release mode
				if cmd.release {
					let wasm_file = wasm_file(&html_constants, &ws_config);
					let cmd_config = CommandConfig::new("wasm-opt")
						.arg("-Oz")
						.arg("--output")
						.arg(wasm_file.to_string_lossy())
						.arg(wasm_file.to_string_lossy());
					cmd_runner.run(ev, cmd_config)?;
				} else {
					ev.trigger_with_cx(Outcome::Pass);
				}
				Ok(())
			},
		),
	)
}
