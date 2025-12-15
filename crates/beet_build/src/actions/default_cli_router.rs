use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

pub fn default_cli_router() -> impl Bundle {
	(CliRouter, InfallibleSequence, children![
		(named_route("watch-lambda"), WatchLambda),
		(named_route("build"), Sequence, children![
			exact_route_match(),
			build_server(),
			respond_ok(),
		]),
		(named_route("run"), Sequence, children![
			exact_route_match(),
			build_server(),
			kill_server(),
			run_server(),
			respond_ok()
		])
	])
}

fn named_route(name: impl AsRef<str>) -> impl Bundle {
	let name = name.as_ref();
	(Name::new(name.to_string()), RoutePartial::new(name))
}

pub fn respond_ok() -> impl Bundle {
	(Name::new("Response"), StatusCode::OK.into_endpoint())
}

pub fn beet_site_cmd() -> CargoBuildCmd {
	CargoBuildCmd::default().package("beet_site")
}
