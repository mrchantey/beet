//! Page `<footer>` widget — copyright, build info, and a default slot.
use crate::prelude::*;
use beet_core::prelude::*;

/// A page `<footer>` displaying the copyright + version + build stage from
/// [`PackageConfig`].
#[template(system)]
pub fn Footer(pkg_config: Res<PackageConfig>) -> impl Bundle {
	let PackageConfig {
		title,
		version,
		stage,
		..
	} = &*pkg_config;

	let current_year = time_ext::current_year();
	let footer_text = format!("© {title} {current_year}");

	// the version segment is omitted when no version is set.
	let mut build_text = version
		.as_ref()
		.map(|version| format!("v{version}"))
		.unwrap_or_default();
	#[cfg(debug_assertions)]
	build_text.push_str(" | build=debug");
	if stage != "prod" {
		build_text.push_str(&format!(" | stage={stage}"));
	}
	// trim a leading separator left when the version segment was omitted.
	let build_text = build_text.trim_start_matches(" | ").to_string();

	rsx! {
		<footer id="page-footer" {Classes::new([classes::PRINT_HIDDEN])}>
			<span {Classes::new([classes::FOOTER_SIDE])}><Slot/></span>
			<span>{footer_text}</span>
			<span {Classes::new([classes::FOOTER_SIDE, classes::TEXT_RIGHT])}>{build_text}</span>
		</footer>
	}
}
