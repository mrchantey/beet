use crate::prelude::*;
use beet_core::prelude::*;
use beet_rsx::prelude::*;

/// Listen for loggong messages from the router lambda,
/// this command never finishes.
#[construct]
pub fn WatchLambda(pkg_config: Res<PackageConfig>) -> impl Bundle {
	let lambda_name = pkg_config.router_lambda_name();
	(
		Name::new("Watch Lambda"),
		ChildProcess::new("aws")
			.arg("logs")
			.arg("tail")
			.arg(format!("/aws/lambda/{lambda_name}"))
			.arg("--format")
			.arg("short") // detailed,short,json
			.arg("--since")
			.arg("2m")
			.arg("--follow"),
	)
}
