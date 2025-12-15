use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

pub fn default_cli_router() -> impl Bundle {
	(CliRouter, InfallibleSequence, beet_site_cmd(), children![
		(single_action_route("refresh-sst", SstCommand {
			cmd: SstSubcommand::Refresh
		})),
		(single_action_route("deploy-sst", SstCommand {
			cmd: SstSubcommand::Deploy
		})),
		(single_action_route("compile-wasm", CompileWasm)),
		(single_action_route("compile-lambda", CompileLambda)),
		(single_action_route("watch-lambda", WatchLambda)),
		(single_action_route("push-assets", PushAssets)),
		(single_action_route("pull-assets", PullAssets)),
		(single_action_route("push-html", PushHtml)),
		(single_action_route("build", BuildServer)),
		(named_route("run", children![
			exact_route_match(),
			BuildServer,
			// kill_server(),
			RunServer,
			respond_ok()
		]))
	])
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

pub fn respond_ok() -> impl Bundle {
	(Name::new("Response"), StatusCode::OK.into_endpoint())
}
