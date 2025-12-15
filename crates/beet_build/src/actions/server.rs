use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

/// Marker to denote this process is running the server
/// for a beet application. Killing the associated [`ChildHandle`]
/// on this entity will kill the server.
#[derive(Component)]
pub struct ServerProcess;


pub fn build_server(cmd: CargoBuildCmd) -> impl Bundle {
	(
		Name::new("Build Server"),
		cmd.feature("server-local")
			.no_default_features()
			.cmd("build")
			.xref()
			.xmap(ChildProcess::from_cargo),
	)
}

pub fn run_server(cmd: CargoBuildCmd) -> impl Bundle {
	(
		Name::new("Run Server"),
		ServerProcess,
		OnSpawn::run_insert::<_, _, Result<ChildProcess>, _>(
			move |manifest: Res<CargoManifest>| {
				let exe_path = cmd
					.exe_path(manifest.package_name())
					.to_string_lossy()
					.to_string();
				path_ext::assert_exists(&exe_path)?;

				ChildProcess {
					cmd: exe_path,
					..default()
				}
				.xok()
			},
		),
	)
}


/// Run the server binary with the `--export-static` flag to retrieve
/// the static content like html pages.
pub fn export_static_content(cmd: CargoBuildCmd) -> impl Bundle {
	(
		Name::new("Export Static Content"),
		ServerProcess,
		OnSpawn::run_insert::<_, _, Result<ChildProcess>, _>(
			move |manifest: Res<CargoManifest>| {
				let exe_path = cmd
					.exe_path(manifest.package_name())
					.to_string_lossy()
					.to_string();
				path_ext::assert_exists(&exe_path)?;

				ChildProcess {
					cmd: exe_path,
					args: vec!["--export-static".into()],
					..default()
				}
				.xok()
			},
		),
	)
}

pub fn kill_server() -> impl Bundle {
	(
		Name::new("Kill Server"),
		OnSpawn::observe(
		|mut ev: On<GetOutcome>,
		 mut commands: Commands,
		 query: Query<Entity, (With<ServerProcess>, With<ChildHandle>)>| {
			for entity in query.iter() {
				commands.entity(entity).remove::<ChildHandle>();
			}
			ev.trigger_with_cx(Outcome::Pass);
		},
	))
}
