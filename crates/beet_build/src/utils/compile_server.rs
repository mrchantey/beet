use crate::prelude::*;
use beet_core::prelude::*;
use beet_rsx::as_beet::PathExt;
use beet_utils::prelude::CargoBuildCmd;
use beet_utils::utils::PipelineTarget;
use bevy::prelude::*;
use std::process::Command;


/// After Codegen, build the router binary and run it once.
pub(crate) fn compile_server(
	_query: Populated<(), Changed<FileExprHash>>,
	mut handle: ResMut<ServerHandle>,
	build_cmd: Res<CargoBuildCmd>,
	pkg_config: Res<PackageConfig>,
) -> Result {
	// if the server is already running, kill it
	// before the snippets are exported because
	// because recompilation means it would contain stale instances.
	if let Some(child) = &mut handle.0 {
		child.kill()?;
	}

	let build_cmd = build_cmd
		.clone()
		.no_default_features()
		.with_feature("server-local");
	Command::new("cargo")
		.args(build_cmd.get_args())
		.envs(pkg_config.envs())
		.xtap(|cmd| {
			debug!("Building server binary\n{:?}", cmd);
		})
		.status()?
		.exit_ok()?
		.xok()
}


/// After compiling server (if required) export the static files if *any*
/// [`StaticRoot`] has changed.
pub fn export_server_ssg(
	_query: Populated<(), Changed<StaticRoot>>,
	cmd: Res<CargoBuildCmd>,
	manifest: Res<CargoManifest>,
	pkg_config: Res<PackageConfig>,
) -> Result {
	// run once to export static
	let exe_path = cmd.exe_path(manifest.package_name());
	PathExt::assert_exists(&exe_path)?;
	Command::new(&exe_path)
		.envs(pkg_config.envs())
		.arg("--export-static")
		.xtap(|cmd| {
			debug!(
				"Running server binary to generate static files \n{:?}",
				cmd
			);
		})
		.status()?
		.exit_ok()?
		.xok()
}

/// A handle to the server process
// this must be Option so that run_server can take the chile
#[derive(Default, Resource)]
pub(crate) struct ServerHandle(Option<std::process::Child>);

impl Drop for ServerHandle {
	fn drop(&mut self) {
		if let Some(child) = &mut self.0 {
			debug!("Killing server process");
			let _result = child.kill();
		}
	}
}
/// Run the server, holding a handle to the process.
pub(crate) fn run_server(
	_query: Populated<(), Changed<FileExprHash>>,
	mut handle: ResMut<ServerHandle>,
	cmd: Res<CargoBuildCmd>,
	manifest: Res<CargoManifest>,
	pkg_config: Res<PackageConfig>,
) -> Result {
	if let Some(child) = &mut handle.0 {
		child.kill()?;
	}
	// run once to export static
	let exe_path = cmd.exe_path(manifest.package_name());
	PathExt::assert_exists(&exe_path)?;
	let child = Command::new(&exe_path)
		.envs(pkg_config.envs())
		.xtap(|cmd| {
			debug!("Running server \n{:?}", cmd);
		})
		.spawn()?;
	handle.0 = Some(child);

	Ok(())
}
