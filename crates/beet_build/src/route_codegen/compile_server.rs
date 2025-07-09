use crate::prelude::*;
use beet_bevy::prelude::When;
use beet_utils::fs::cargo::CargoBuildCmd;
use bevy::prelude::*;
use std::process::Command;


/// After Codegen, build the router binary and run it once.
pub(crate) fn compile_server(
	_query: Populated<(), Changed<FileExprHash>>,
	mut handle: ResMut<ServerHandle>,
	cmd: When<Res<CargoBuildCmd>>,
) -> Result {
	// if the server is already running, kill it
	// before the snippets are exported because
	// because recompilation means it would contain stale instances.
	if let Some(child) = &mut handle.0 {
		child.kill()?;
	}

	debug!("Building server binary");
	cmd.spawn()?;
	Ok(())
}


/// After compiling server (if required) export the static files.
pub fn export_server_ssg(
	_query: Populated<(), Changed<SourceFileRoot>>,
	cmd: When<Res<CargoBuildCmd>>,
	manifest: When<Res<CargoManifest>>,
) -> Result {
	// run once to export static
	let exe_path = cmd.exe_path(manifest.package_name());
	debug!(
		"Running server binary to generate static files \n{}",
		exe_path.display()
	);
	Command::new(&exe_path)
		.arg("export-static")
		.status()?
		.exit_ok()?;
	Ok(())
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
	cmd: When<Res<CargoBuildCmd>>,
	manifest: When<Res<CargoManifest>>,
) -> Result {
	if let Some(child) = &mut handle.0 {
		child.kill()?;
	}
	// run once to export static
	let exe_path = cmd.exe_path(manifest.package_name());
	debug!(
		"Running server binary to generate static files \n{}",
		exe_path.display()
	);
	let child = Command::new(&exe_path).spawn()?;
	handle.0 = Some(child);

	Ok(())
}
