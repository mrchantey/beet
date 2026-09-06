//! `ColorSchemeScript` widget — an inline `<script>` that seeds the document
//! `.light-scheme`/`.dark-scheme` class before first paint and persists the
//! user's explicit choice.
//!
//! Web target only — non-web targets carry the scheme class directly on an
//! ancestor element. Drop it into the document `<head>` (eg the `head` slot) so
//! the scheme is applied before the body paints, avoiding a flash of the wrong
//! theme.
use crate::style::material::classes;
use beet_core::prelude::*;

/// Emits the bundled color-scheme seed/persist script as an inline `<script>`.
///
/// The script reads the persisted choice (else the OS `prefers-color-scheme`),
/// toggling the matching class on `<html>`. `setColorScheme("light" | "dark")`
/// becomes available globally for a theme switcher.
#[template]
pub fn ColorSchemeScript() -> impl Bundle {
	// inject the shared class-name constants so the script and the style rules
	// stay in lockstep.
	let body = format!(
		"const LIGHT_SCHEME={:?},DARK_SCHEME={:?};\n{}",
		classes::LIGHT_SCHEME.as_selector(),
		classes::DARK_SCHEME.as_selector(),
		include_str!("./color_scheme.js"),
	);
	rsx! {
		<script>{body}</script>
	}
}
