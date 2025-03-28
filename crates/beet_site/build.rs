use beet::prelude::*;


fn main() {
	println!("building!");
	// panic!("failed");
	let cx = app_cx!();
	// println!("cargo::rerun-if-changed=build.rs");

	let is_wasm = std::env::var("TARGET").unwrap() == "wasm32-unknown-unknown";

	if is_wasm {
		BuildWasmRoutes::new(CodegenFile::new(
			"crates/beet_site/src/codegen/wasm.rs",
			&cx.pkg_name,
		))
		.run()
		.unwrap();
	} else {
		BuildFileRouteTree::new(FuncFilesToRouteTree {
			codgen_file: CodegenFile::new(
				"crates/beet_site/src/codegen/route_tree.rs",
				&cx.pkg_name,
			),
		})
		.with_step(BuildFileRoutes::new(
			"crates/beet_site/src/routes",
			"crates/beet_site/src/codegen/routes.rs",
			&cx.pkg_name,
		))
		.with_step({
			let mut mockups = BuildFileRoutes::mockups(
				"crates/beet_design/src",
				"beet_design",
			);
			mockups.codegen_file.use_beet_tokens =
				"use beet_router::as_beet::*;".into();
			mockups
		})
		.run()
		.unwrap();
	}
}
