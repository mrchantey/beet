use beet_router::prelude::*;
use beet_rsx::rsx::BuildStep;
use sweet::prelude::ReadFile;

/// Demonstration of how to collect all files in the 'routes' dir
/// and create a `routes.rs` file containing them all.
pub fn main() {
	let parser =
		BuildFileRoutes::new("crates/beet_router/src/test_site", "beet_router");
	parser.run().unwrap();
	let file =
		ReadFile::to_string("crates/beet_router/src/test_site/routes.rs")
			.unwrap();


	// let routes = parser.build_strings().unwrap();
	println!(
		"wrote crates/beet_router/src/test_site/codegen/router.rs\n{:#?}",
		file
	);
}
