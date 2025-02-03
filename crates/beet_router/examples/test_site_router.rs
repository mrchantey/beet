use beet_router_parser::prelude::*;
use sweet::prelude::FsExt;

fn parser() -> BuildRoutesMod {
	let src = FsExt::workspace_root().join("crates/beet_router/src/test_site");
	BuildRoutesMod {
		src,
		file_router_ident: "crate::DefaultFileRouter".into(),
		file_router_tokens: Some(
			r#"
				use crate::prelude::*;
			"#
			.to_string(),
		),
		..Default::default()
	}
}

fn main() {
	let parser = parser();
	// actually writing a file during tests
	parser.build_and_write().unwrap();
	// expect(CompileCheck::file(&routes_file.routes_file())).to_be_ok();
	let routes = parser.build_string().unwrap();
	println!("wrote crates/beet_router/src/test_site_router.rs{}", routes);
}
