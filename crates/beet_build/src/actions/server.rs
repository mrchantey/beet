use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_rsx::prelude::*;

/// Marker to denote this process is running the server
/// for a beet application. Killing the associated [`ChildHandle`]
/// on this entity will kill the server.
#[derive(Component)]
pub struct ServerProcess;

#[construct]
pub fn BuildServer(
	entity: Entity,
	query: AncestorQuery<&'static CargoBuildCmd>,
) -> impl Bundle {
	(
		Name::new("Build Server"),
		query
			.get(entity)
			.cloned()
			.unwrap_or_default()
			.feature("server-local")
			.no_default_features()
			.cmd("build")
			.xref()
			.xmap(ChildProcess::from_cargo),
	)
}

#[construct]
pub fn RunServer(
	entity: Entity,
	query: AncestorQuery<&'static CargoBuildCmd>,
) -> impl Bundle {
	let cmd = query.get(entity).cloned().unwrap_or_default();
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
#[construct]
pub fn ExportStaticContent(
	entity: Entity,
	query: AncestorQuery<&'static CargoBuildCmd>,
) -> impl Bundle {
	let cmd = query.get(entity).cloned().unwrap_or_default();

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

#[construct]
pub fn KillServer() -> impl Bundle {
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
