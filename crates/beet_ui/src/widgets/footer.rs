//! Page `<footer>` widget — copyright, build info, and a default slot.
use beet_core::prelude::*;

/// A page `<footer>` displaying the copyright + version + build stage from
/// [`PackageConfig`].
#[scene(system)]
pub fn Footer(pkg_config: Res<PackageConfig>) -> impl Scene {
	let PackageConfig {
		title,
		version,
		stage,
		..
	} = &*pkg_config;

	let current_year = time_ext::current_year();
	let footer_text = format!("© {title} {current_year}");

	let mut build_text = format!("v{version}");
	#[cfg(debug_assertions)]
	build_text.push_str(" | build=debug");
	if stage != "prod" {
		build_text.push_str(&format!(" | stage={stage}"));
	}

	rsx! {
		<footer id="page-footer" {Classes::new([classes::PRINT_HIDDEN])}>
			<span {Classes::new([classes::FOOTER_SIDE])}><slot/></span>
			<span>{footer_text}</span>
			<span {Classes::new([classes::FOOTER_SIDE, classes::TEXT_RIGHT])}>{build_text}</span>
		</footer>
	}
}
