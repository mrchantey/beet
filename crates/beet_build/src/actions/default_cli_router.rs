use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

pub fn default_cli_router() -> impl Bundle {
	let build_cmd = beet_site_cmd();

	(CliRouter, InfallibleSequence, children![
		(named_route("compile-lambda", children![
			exact_route_match(),
			compile_lambda(build_cmd.clone()),
			respond_ok(),
		])),
		(named_route("watch-lambda", children![
			exact_route_match(),
			WatchLambda,
			respond_ok(),
		])),
		(named_route("push-assets", children![
			exact_route_match(),
			PushAssets,
			respond_ok(),
		])),
		(named_route("pull-assets", children![
			exact_route_match(),
			PullAssets,
			respond_ok(),
		])),
		(named_route("push-html", children![
			exact_route_match(),
			PushHtml,
			respond_ok(),
		])),
		(named_route("build", children![
			exact_route_match(),
			build_server(build_cmd.clone()),
			respond_ok(),
		])),
		(named_route("run", children![
			exact_route_match(),
			build_server(build_cmd.clone()),
			// kill_server(),
			run_server(build_cmd.clone()),
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

// fn single_action_route(
// 	name: impl AsRef<str>,
// 	children: impl Bundle,
// ) -> impl Bundle {
// 	(named_route(name, children![
// 		exact_route_match(),
// 		build_server(build_cmd.clone()),
// 		respond_ok(),
// 	]))
// }

fn beet_site_cmd() -> CargoBuildCmd {
	CargoBuildCmd::default().package("beet_site")
}

pub fn respond_ok() -> impl Bundle {
	(Name::new("Response"), StatusCode::OK.into_endpoint())
}
