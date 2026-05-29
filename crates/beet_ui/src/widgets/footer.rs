//! Page `<footer>` widget — copyright, build info, and a default slot.
use beet_core::prelude::*;

/// A page `<footer>` displaying the copyright + version + build stage from
/// [`PackageConfig`].
#[scene(system)]
pub fn Footer(pkg_config: Res<PackageConfig>) -> impl Scene {
	let PackageConfig { title, version, stage, .. } = &*pkg_config;

	let current_year = current_year();
	let footer_text = format!("© {title} {current_year}");

	let mut build_text = format!("v{version}");
	#[cfg(debug_assertions)]
	build_text.push_str(" | build=debug");
	if stage != "prod" {
		build_text.push_str(&format!(" | stage={stage}"));
	}

	rsx! {
		<footer id="page-footer" {Classes::new([classes::PRINT_HIDDEN])}>
			<span>{footer_text}</span>
			<slot/>
			<span>{build_text}</span>
		</footer>
	}
}

/// `chrono` is std-only and not in the `scene` feature graph; the footer just
/// needs the year, so we derive it directly via `std::time` to avoid a new dep.
/// Approximation is fine for a footer string (off by at most a day around new
/// year).
fn current_year() -> i32 {
	use std::time::SystemTime;
	use std::time::UNIX_EPOCH;
	let secs = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_secs() as i64)
		.unwrap_or(0);
	1970 + (secs as f64 / (365.2425 * 86400.0)) as i32
}
