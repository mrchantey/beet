use crate::style::SwatchGroup;
use crate::style::color_swatch_groups;
use beet::prelude::*;

/// Showcases every Material color role as a swatch, in both the light and dark
/// schemes. Each swatch fills its background with the color-role token and its
/// text with the matching "on" token.
///
/// The swatch colours are token *references* bound by [`color_scheme_rules`],
/// not a web-only `<style>`, so the palette resolves on the terminal too and
/// each `.light-scheme`/`.dark-scheme` wrapper renders its own scheme on both
/// targets.
pub fn get() -> impl Scene {
	rsx! {
		<article>
			<h1>"Color Schemes"</h1>
			<h2>"Light"</h2>
			<div {Classes::new([classes::LIGHT_SCHEME])}>{scheme()}</div>
			<h2>"Dark"</h2>
			<div {Classes::new([classes::DARK_SCHEME])}>{scheme()}</div>
		</article>
	}
}

/// One full set of color-role swatches, grouped by Material role family.
fn scheme() -> impl Scene {
	let groups: Vec<_> =
		color_swatch_groups().into_iter().map(swatch_group).collect();
	rsx! { <div>{groups}</div> }
}

/// A titled, wrapping row of swatches for one colour-role family.
fn swatch_group(group: SwatchGroup) -> impl Scene {
	let title = group.title.to_string();
	let boxes: Vec<_> = group
		.swatches
		.into_iter()
		.map(|swatch| {
			let label = swatch.label.to_string();
			rsx! {
				<div {Classes::new([
					crate::style::classes::COLOR_BOX,
					ClassName::string(swatch.role),
				])}>
					{label}
				</div>
			}
		})
		.collect();
	rsx! {
		<div>
			<h3>{title}</h3>
			<div {Classes::new([crate::style::classes::COLOR_GROUP])}>{boxes}</div>
		</div>
	}
}
