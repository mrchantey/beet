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
/// Crates upstream of beet, like `rsx_site` will not use the internal name
pub fn internal_or_beet(pkg_name: &str) -> syn::Path {
	if !is_internal() {
		syn::parse_str("beet").unwrap()
	} else if pkg_name == crate_name() {
		syn::parse_str("crate").unwrap()
	} else {
		syn::parse_str(pkg_name).unwrap()
	}
}
/// Resolve the path to the `bevy` crate for macro output.
///
/// Always reached through a `beet_core`/`beet` re-export (`exports::bevy`), so a
/// crate consuming the macro needs no direct `bevy` dependency — only the
/// `beet_core` (internal) or `beet` (downstream) it already depends on. Mirrors
/// [`internal_or_beet`]: `crate` inside `beet_core`, `beet_core` for other
/// internal crates, `beet` downstream.
pub fn bevy() -> syn::Path {
	let base = if !is_internal() {
		"beet"
	} else if crate_name() == "beet_core" {
		"crate"
	} else {
		"beet_core"
	};
	syn::parse_str(&alloc::format!("{base}::exports::bevy")).unwrap()
}

fn crate_name() -> alloc::string::String {
	std::env::var("CARGO_PKG_NAME").unwrap()
}


/// checks the CARGO_PKG_NAME against a list of internal packages
pub fn is_internal() -> bool {
	const INTERNAL_PKGS: &[&str] = &[
		"beet_thread",
		"beet_build",
		"beet_core",
		"beet_core_macros",
		"beet_core_shared",
		"beet_flow",
		"beet_flow_macros",
		"beet_ml",
		"beet_net",
		"beet_ui",
		"beet_router",
		"beet_spatial",
		"beet_action",
		"beet_infra",
	];
	INTERNAL_PKGS.contains(&crate_name().as_str())
}
