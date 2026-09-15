//! Page `<footer>` widget — copyright, build info, and a default slot.
use crate::prelude::*;
use beet_core::prelude::*;

/// A page `<footer>` displaying the copyright + version from [`PackageConfig`],
/// and the launch stage from [`BootstrapConfig`] outside prod.
#[template(system)]
pub fn Footer(pkg_config: Res<PackageConfig>) -> impl Bundle {
	let PackageConfig { title, version, .. } = &*pkg_config;
	let bootstrap = BootstrapConfig::get();

	let current_year = Timestamp::now().civil_date().0;
	let footer_text = format!("© {title} {current_year}");

	// the version and the stage, facts of the launch every process serving
	// or booting the page agrees on; a build profile is the binary's, and a
	// release wasm adopting a debug server's page would have to repaint it
	let mut build_text = format!("v{version}");
	if !bootstrap.is_prod() {
		build_text.push_str(&format!(" | stage={}", bootstrap.stage));
	}

	rsx! {
		<footer id="page-footer" {Classes::new([classes::PRINT_HIDDEN])}>
			<span {Classes::new([classes::FOOTER_SIDE])}><Slot/></span>
			<span>{footer_text}</span>
			<span {Classes::new([classes::FOOTER_SIDE, classes::TEXT_RIGHT])}>{build_text}</span>
		</footer>
	}
}
