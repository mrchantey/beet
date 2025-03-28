use beet_router::prelude::*;
use beet_rsx::rsx::BuildStep;
use sweet::prelude::ReadFile;

/// Demonstration of how to collect all files in the 'routes' dir
/// and create a `routes.rs` file containing them all.
pub fn main() {
	let out_file = "crates/beet_router/src/test_site/codegen/routes.rs";
	let mut parser = BuildFileRoutes::http_routes(
		"crates/beet_router/src/test_site/routes",
		out_file,
		"beet_router",
	);
	parser.codegen_file.use_beet_tokens = "use beet_router::as_beet::*;".into();
	parser.run().unwrap();
	let file = ReadFile::to_string(out_file).unwrap();


	// let routes = parser.build_strings().unwrap();
	println!("wrote {}\n{}", out_file, file);
}
