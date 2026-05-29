//! `Stylesheet` widget — emits the active rule set as an inline `<style>`.
//!
//! Web target only: it reads the global [`RuleSet`](crate::style) at scene
//! build via [`StyleQuery`] and bakes the built CSS into a `<style>` element,
//! so callers drop it into the document `<head>` instead of hand-assembling a
//! `<style>{css}</style>` string.
use crate::style::CssBuilder;
use crate::style::FormatVariables;
use crate::style::StyleQuery;
use beet_core::prelude::*;

/// Emits the CSS built from the active rule set as an inline `<style>` element.
///
/// `minify` toggles whitespace stripping (off by default, for readable dev
/// output). Variable names use [`FormatVariables::short`].
#[scene(system)]
pub fn Stylesheet(#[prop] minify: bool, style: StyleQuery) -> impl Scene {
	let css = style
		.build_css(
			&CssBuilder::default()
				.with_minify(minify)
				.with_format_variables(FormatVariables::short()),
		)
		.unwrap_or_else(|err| {
			cross_log!("Stylesheet: failed to build css: {err}");
			String::new()
		});
	rsx! {
		<style>{css}</style>
	}
}
