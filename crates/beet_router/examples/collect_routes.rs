use beet_router::prelude::*;
use sweet::prelude::FsExt;


/// Demonstration of how to collect all files in the 'routes' dir
/// and create a `routes.rs` file containing them all.
pub fn main() {
	let routes_dir =
		FsExt::workspace_root().join("crates/beet_router/src/test_site/routes");
	let parser = CollectRoutes {
		routes_dir,
		route_type: "crate::prelude::StaticRoute".into(),
		file_router_tokens: Some(
			r#"
				use crate::prelude::*;
			"#
			.to_string(),
		),
		..Default::default()
	};
	parser.build_and_write().unwrap();
	let routes = parser.build_strings().unwrap();
	println!(
		"wrote crates/beet_router/src/test_site_router.rs\n{:#?}",
		routes
	);
}
