use anyhow::Result;
use beet::prelude::*;

fn main() -> Result<()> {
	println!("cargo::rerun-if-changed=build.rs");
	println!("cargo::rerun-if-changed=src/codegen");
	println!("cargo::rerun-if-changed=../beet_design/src");
	println!("cargo::warning={}", "\n🚀🚀running!\n");
	let cx = app_cx!();
	
	let is_wasm = std::env::var("TARGET").unwrap() == "wasm32-unknown-unknown";

	if is_wasm {
		BuildWasmRoutes::new(CodegenFile::new_workspace_rel(
			"crates/beet_site/src/codegen/wasm.rs",
			&cx.pkg_name,
		))
		.run()?;
	} else {
		let mut routes =
			FileGroup::new_workspace_rel("crates/beet_site/src/routes")?
				.with_filter(
					GlobFilter::default()
						.with_include("*.rs")
						.with_exclude("*mod.rs"),
				)
				.pipe(FileGroupToFuncFiles::default())?
				.pipe(FuncFilesToRouteFuncs::http_routes())?
				.pipe(RouteFuncsToCodegen::new(
					CodegenFile::new_workspace_rel(
						"crates/beet_site/src/codegen/routes.rs",
						&cx.pkg_name,
					),
				))?
				.map(|(_, routes, codegen)| -> Result<_> {
					codegen.build_and_write()?;
					Ok(routes)
				})?;

		// should be identical to crates/beet_design/build.rs
		let mockups = FileGroup::new_workspace_rel("crates/beet_design/src")?
			.with_filter(GlobFilter::default().with_include("*.mockup.rs"))
			.pipe(FileGroupToFuncFiles::default())?
			.pipe(FuncFilesToRouteFuncs::mockups())?
			.map(|(_, routes)| routes);

		routes.extend(mockups);

		routes.pipe(RouteFuncsToTree {
			codgen_file: CodegenFile::new_workspace_rel(
				"crates/beet_site/src/codegen/route_tree.rs",
				&cx.pkg_name,
			),
		})?;
	}
	Ok(())
}
