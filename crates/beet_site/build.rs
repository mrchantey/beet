use anyhow::Result;
use beet::exports::*;
use beet::prelude::*;
use sweet::prelude::*;

// runtime env vars: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
// cargo:: output https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script
fn main() -> Result<()> {
	BuildUtils::rerun_if_changed("../beet_design/src/**/*.mockup.rs");
	BuildUtils::rerun_if_changed("../beet_design/public");
	// println!("cargo::warning={}", "🚀🚀 building beet_site");

	// ⚠️ this is a downstream copy of crates/beet_design/build.rs
	// we're actually only using the route paths, maybe we should generate
	// those in design
	let mockups =
		FileGroup::new(AbsPathBuf::new_manifest_rel("../beet_design/src")?)
			.with_filter(GlobFilter::default().with_include("*.mockup.*"))
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(
				MapFuncTokens::default()
					.base_route("/design")
					.replace_route([(".mockup", "")]),
			);


	DefaultBuilder {
		wasm_imports: vec![
			syn::parse_quote!(
				use beet::design as beet_design;
			),
			syn::parse_quote!(
				use beet::prelude::*;
			),
		],
		routes: mockups.funcs,
		..Default::default()
	}
	.build()?;



	Ok(())
}
