// use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

use crate::prelude::TerminalCommand;


pub fn default_cli_router() -> impl Bundle {
	(CliRouter, InfallibleSequence, children![
		EndpointBuilder::default().with_handler(|| { "foobar" }),
		EndpointBuilder::default().with_path("run").with_handler(
			async |request: Request, _cx: EndpointContext| {
				let path = request.parts.uri.path();
				println!("path: {}", path);
				"foobar"
			}
		),
		// (RouteSegments::new("build"), Sequence, children![
		// 	exact_path(),
		// 	build_server()
		// ]),
		(RoutePartial::new("build"), Sequence, children![
			exact_route_match(),
			build_server(),
			StatusCode::OK.into_endpoint()
		])
	])
}


fn beet_site_cmd() -> CargoBuildCmd {
	CargoBuildCmd::default().package("beet_site")
}


fn build_server() -> impl Bundle {
	beet_site_cmd()
		.feature("server")
		.cmd("build")
		.xref()
		.xmap(TerminalCommand::from_cargo)
}
