use anyhow::Result;
use beet::prelude::*;

fn main() -> Result<()> {
	// panic!("not even");
	println!("building!");
	// panic!("failed");
	let cx = app_cx!();
	// println!("cargo::rerun-if-changed=build.rs");

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

		let mockups = FileGroup::new_workspace_rel("crates/beet_design/src")?
			.with_filter(GlobFilter::default().with_include("*.mockup.rs"))
			.pipe(FileGroupToFuncFiles::default())?
			.pipe(FuncFilesToRouteFuncs::mockups())?
			.pipe(RouteFuncsToCodegen::new(
				CodegenFile::new_workspace_rel(
					"crates/beet_design/src/codegen/mockups.rs",
					"beet_design",
				)
				.with_use_beet_tokens("use beet_router::as_beet::*;"),
			))?
			.map(|(_, routes, codegen)| -> Result<_> {
				codegen.build_and_write()?;
				Ok(routes)
			})?;

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
