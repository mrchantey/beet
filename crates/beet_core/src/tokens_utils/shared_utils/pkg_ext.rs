/// when we need an internal package name for proc macros, ie `beet_core`,
/// determine whether to use that internal name, or if it has been reexported
/// by beet.
///
/// - if its external use `beet`
/// - if its internal and current use `crate`
/// - if its a different internal  use `pkg_name`
///
/// ## Examples and Integration Tests
///
/// Examples and integration tests will need to reexport the crate dependencies.
/// If the macro expands to `crate::prelude::foo`
/// then the example should `use beet_core::prelude;`
/// which allows `crate::prelude` to resolve to `beet_core`.
///
/// Crates upstream of beet, like `beet_site` will not use the internal name
pub fn internal_or_beet(pkg_name: &str) -> syn::Path {
	if !is_internal() {
		syn::parse_str("beet").unwrap()
	} else if pkg_name == std::env::var("CARGO_PKG_NAME").unwrap() {
		syn::parse_str("crate").unwrap()
	} else {
		syn::parse_str(pkg_name).unwrap()
	}
}

/// checks the CARGO_PKG_NAME against a list of internal packages
pub fn is_internal() -> bool {
	const INTERNAL_PKGS: &[&str] = &[
		"beet_agent",
		"beet_build",
		"beet_core",
		"beet_core_macros",
		"beet_design",
		"beet_dom",
		"beet_flow",
		"beet_flow_macros",
		"beet_ml",
		"beet_net",
		"beet_parse",
		"beet_rsx",
		"beet_rsx_macros",
		"beet_rsx_combinator",
		"beet_router",
		"beet_sim",
		"beet_spatial",
		"sweet",
		"sweet_macros",
	];
	let current_pkg = std::env::var("CARGO_PKG_NAME").unwrap();
	INTERNAL_PKGS.contains(&current_pkg.as_str())
}
