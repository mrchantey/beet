//! `Preflight` widget — an opt-in base reset matching Tailwind's Preflight
//! normalize. Web target only: it emits a `<style>` element, so callers drop it
//! into the document `<head>` when they want browser-default normalization
//! without pulling in Tailwind's utility classes.
//!
//! `preflight.css` is a verbatim copy of upstream and must not be edited. Site
//! overrides go in the adjacent `reset.css`, emitted by [`Reset`] and dropped
//! into the `<head>` immediately after [`Preflight`] so it wins the cascade.
use beet_core::prelude::*;

/// Emits the bundled Tailwind Preflight reset (`preflight.css`) as a `<style>`
/// element.
#[template]
pub fn Preflight() -> impl Bundle {
	let preflight = include_str!("./preflight.css");
	rsx! {
		<style>{preflight}</style>
	}
}

/// Emits the beet web-only reset overrides (`reset.css`) as a `<style>` element.
///
/// Web target only. Load it right after [`Preflight`] in the document `<head>`
/// so its rules layer over the verbatim Tailwind reset (eg restoring prose list
/// markers). It is an escape hatch for browser-only presentation; prefer a token
/// rule or class where one can express the intent.
#[template]
pub fn Reset() -> impl Bundle {
	let reset = include_str!("./reset.css");
	rsx! {
		<style>{reset}</style>
	}
}
