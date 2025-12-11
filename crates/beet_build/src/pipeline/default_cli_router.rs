// use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

use crate::prelude::ChildHandle;
use crate::prelude::ChildProcess;
use crate::utils::CargoManifest;


pub fn default_cli_router() -> impl Bundle {
	(CliRouter, InfallibleSequence, children![
		EndpointBuilder::default().with_handler(|| { "foobar" }),
		// (RouteSegments::new("build"), Sequence, children![
		// 	exact_path(),
		// 	build_server()
		// ]),
		(
			Name::new("build"),
			RoutePartial::new("build"),
			Sequence,
			children![
				exact_route_match(),
				build_server(),
				StatusCode::OK.into_endpoint()
			]
		),
		(
			Name::new("run"),
			RoutePartial::new("run"),
			Sequence,
			children![
				exact_route_match(),
				build_server(),
				kill_server(),
				run_server(),
				(Name::new("Response"), StatusCode::OK.into_endpoint())
			]
		)
	])
}


fn beet_site_cmd() -> CargoBuildCmd {
	CargoBuildCmd::default().package("beet_site")
}


fn build_server() -> impl Bundle {
	(
		Name::new("Build Server"),
		beet_site_cmd()
			.feature("server-local")
			.cmd("build")
			.xref()
			.xmap(ChildProcess::from_cargo),
	)
}
fn run_server() -> impl Bundle {
	(
		Name::new("Run Server"),
		ServerProcess,
		OnSpawn::run_insert::<_, _, Result<ChildProcess>, _>(
			|manifest: Res<CargoManifest>| {
				let exe_path = beet_site_cmd()
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

fn kill_server() -> impl Bundle {
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

/// Marker to denote this process is running the server
/// for a beet application. Killing the associated [`ChildHandle`]
/// on this entity will kill the server.
#[derive(Component)]
pub struct ServerProcess;

#[derive(Component)]
pub struct ClientProcess;
