use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

pub fn default_cli_router() -> impl Bundle {
	(
		Name::new("Cli Router"),
		CliRouter,
		InfallibleSequence,
		beet_site_cmd(),
		children![
			(single_action_route(
				"refresh-sst",
				SstCommand::new(SstSubcommand::Refresh)
			)),
			(single_action_route(
				"deploy-sst",
				SstCommand::new(SstSubcommand::Deploy)
			)),
			(single_action_route("compile-wasm", CompileWasm)),
			(single_action_route("compile-lambda", CompileLambda)),
			(single_action_route("watch-lambda", WatchLambda)),
			(single_action_route("push-assets", PushAssets)),
			(single_action_route("pull-assets", PullAssets)),
			(single_action_route("push-html", PushHtml)),
			(single_action_route("build", BuildServer)),
			(single_action_route(
				"parse-files",
				import_and_parse_source_files(),
			)),
			(named_route("run", children![
				exact_route_match(),
				import_and_parse_source_files(),
				BuildServer,
				ExportStaticContent,
				CompileWasm,
				RunServer,
				respond_ok()
			])),
			(named_route("deploy", children![
				exact_route_match(),
				force_remote_service_access(),
				BuildServer,
				respond_ok()
			]))
		],
	)
}

/// When deploying remote service access is the only option
/// that makes sense
fn force_remote_service_access() -> impl Bundle {
	OnSpawn::observe(
		|mut ev: On<GetOutcome>, mut config: ResMut<PackageConfig>| {
			config.service_access = ServiceAccess::Remote;
			ev.trigger_with_cx(Outcome::Pass);
		},
	)
}


fn named_route(name: impl AsRef<str>, children: impl Bundle) -> impl Bundle {
	let name = name.as_ref();
	(
		Name::new(name.to_string()),
		RoutePartial::new(name),
		Sequence,
		children,
	)
}

fn single_action_route(
	name: impl AsRef<str>,
	action: impl Bundle,
) -> impl Bundle {
	named_route(name, children![exact_route_match(), action, respond_ok()])
}

fn beet_site_cmd() -> CargoBuildCmd {
	CargoBuildCmd::default().package("beet_site")
}

fn respond_ok() -> impl Bundle {
	(Name::new("Response"), StatusCode::OK.into_endpoint())
}
