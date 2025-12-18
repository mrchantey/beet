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
pub fn BuildServer() -> impl Bundle {
	(
		Name::new("Build Server"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut cmd_params: CommandParams,
			      query: AncestorQuery<&'static CargoBuildCmd>| {
				let config = query
					.get(ev.action())
					.cloned()
					.unwrap_or_default()
					.cmd("build")
					.feature("server-local")
					.no_default_features();

				cmd_params.execute(ev, config)
			},
		),
	)
}

#[construct]
pub fn RunServer() -> impl Bundle {
	(
		Name::new("Run Server"),
		ServerProcess,
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut cmd_params: CommandParams,
			      manifest: Res<CargoManifest>,
			      query: AncestorQuery<&'static CargoBuildCmd>| {
				let cmd = query.get(ev.action()).cloned().unwrap_or_default();

				let exe_path = cmd
					.exe_path(manifest.package_name())
					.to_string_lossy()
					.to_string();
				path_ext::assert_exists(&exe_path)?;

				let config = CommandConfig::new(exe_path);

				cmd_params.execute(ev, config)
			},
		),
	)
}


/// Run the server binary with the `--export-static` flag to retrieve
/// the static content like html pages.
#[construct]
pub fn ExportStaticContent() -> impl Bundle {
	(
		Name::new("Export Static Content"),
		ServerProcess,
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut cmd_params: CommandParams,
			      manifest: Res<CargoManifest>,
			      query: AncestorQuery<&'static CargoBuildCmd>| {
				let exe_path = query
					.get(ev.action())
					.cloned()
					.unwrap_or_default()
					.exe_path(manifest.package_name())
					.to_string_lossy()
					.to_string();
				path_ext::assert_exists(&exe_path)?;

				let config =
					CommandConfig::new(exe_path).arg("--export-static");

				cmd_params.execute(ev, config)
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
