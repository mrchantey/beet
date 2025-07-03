use crate::prelude::*;
use beet_bevy::prelude::When;
use beet_fs::cargo::CargoBuildCmd;
use bevy::prelude::*;
use std::process::Command;







/// After Codegen, build the router binary and run it once.
pub fn compile_server(
	_query: Populated<(), Changed<RouteCodegenRoot>>,
	cmd: When<Res<CargoBuildCmd>>,
) -> Result {
	debug!("Building native binary");
	cmd.spawn()?;
	Ok(())
}


/// After compiling server (if required) export the static files.
pub fn export_server_ssg(
	_query: Populated<(), Changed<SourceFileRoot>>,
	cmd: When<Res<CargoBuildCmd>>,
) -> Result {
	// run once to export static
	let exe_path = cmd.exe_path();
	debug!(
		"Running native binary to generate static files \nExecuting {}",
		exe_path.display()
	);
	Command::new(&exe_path)
		.arg("export-static")
		.status()?
		.exit_ok()?;
	Ok(())
}


#[derive(Default, Resource)]
pub(crate) struct ServerHandle(Option<std::process::Child>);

/// Run the server, holding a handle to the process.
pub(crate) fn run_server(
	_query: Populated<(), Changed<RouteCodegenRoot>>,
	mut handle: Local<ServerHandle>,
	cmd: When<Res<CargoBuildCmd>>,
) -> Result {
	if let Some(child) = &mut handle.0 {
		child.kill()?;
	}
	// run once to export static
	let exe_path = cmd.exe_path();
	debug!(
		"Running native binary to generate static files \nExecuting {}",
		exe_path.display()
	);
	let child = Command::new(&exe_path).spawn()?;
	handle.0 = Some(child);

	Ok(())
}
