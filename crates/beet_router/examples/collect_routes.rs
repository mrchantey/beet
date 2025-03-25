use beet_router::prelude::*;
use sweet::prelude::*;


/// Demonstration of how to collect all files in the 'routes' dir
/// and create a `routes.rs` file containing them all.
pub fn main() {
	let parser = CollectRoutes {
		routes_dir: WorkspacePathBuf::new(
			"crates/beet_router/src/test_site/routes",
		)
		.into_canonical()
		.unwrap(),
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
