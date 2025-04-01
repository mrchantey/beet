use crate::prelude::*;
use beet_rsx::as_beet::*;
use material_colors::theme::Theme;



/// Entry point for the beet design system.
///
/// Beet's design system is inspired by a few places:
/// - Color: Material Design
/// 	- [`material-colors` crate](https://crates.io/crates/material-colors)
/// - Typography: Starlight
/// 	-
/// - Layout: PicoCSS
#[derive(Node)]
pub struct DesignSystem {
	pub theme: Theme,
}


fn design_system(props: DesignSystem) -> RsxNode {
	rsx! {
		<ColorScheme theme=props.theme />
		<Css />
	}
}
