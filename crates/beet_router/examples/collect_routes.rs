use beet_router_parser::prelude::*;
use sweet::prelude::FsExt;


/// Demonstration of how to collect all files in the 'routes' dir
/// and create a `routes.rs` file containing them all.
pub fn main() {
	let src = FsExt::workspace_root().join("crates/beet_router/src/test_site");
	let parser = CollectRoutes {
		src,
		file_router_ident: "crate::DefaultFileRouter".into(),
		file_router_tokens: Some(
			r#"
				use crate::prelude::*;
			"#
			.to_string(),
		),
		..Default::default()
	};
	parser.build_and_write().unwrap();
	let routes = parser.build_string().unwrap();
	println!(
		"wrote crates/beet_router/src/test_site_router.rs\n{}",
		routes
	);
}
