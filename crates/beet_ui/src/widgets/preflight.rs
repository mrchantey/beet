//! `Preflight` widget — an opt-in base reset matching Tailwind's Preflight
//! normalize. Web target only: it emits a `<style>` element, so callers drop it
//! into the document `<head>` when they want browser-default normalization
//! without pulling in Tailwind's utility classes.
//!
use beet_core::prelude::*;

/// Emits the bundled Tailwind Preflight reset (`preflight.css`) as a `<style>`
/// element.
#[scene]
pub fn Preflight() -> impl Scene {
	let preflight = include_str!("./preflight.css");
	rsx! {
		<style>{preflight}</style>
	}
}
