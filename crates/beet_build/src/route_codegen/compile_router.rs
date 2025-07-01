use crate::prelude::*;
use beet_fs::cargo::CargoBuildCmd;
use bevy::prelude::*;
use std::process::Command;







/// After Codegen, build the router binary and run it once.
pub fn compile_router(
	cmd: Res<CargoBuildCmd>,
	// any changed child FileExprHash results in changed RouteCodegenRoot
	_query: Populated<(), Changed<RouteCodegenRoot>>,
) -> Result {
	debug!("Building native binary",);
	cmd.spawn()?;

	// run once to export static
	let exe_path = cmd.exe_path();
	debug!(
		"Running native binary to generate static files \nExecuting {}",
		exe_path.display()
	);
	Command::new(&exe_path)
		// .arg("--html-dir")
		// .arg(&self.build_args.html_dir)
		.arg("export-static")
		.status()?
		.exit_ok()?;
	Ok(())
}
