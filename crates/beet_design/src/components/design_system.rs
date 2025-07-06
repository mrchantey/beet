use crate::prelude::*;
use material_colors::theme::Theme;


/// Entry point for the beet design system.
///
/// Beet's design system is inspired by a few places:
/// - Color: Material Design
/// 	- [`material-colors` crate](https://crates.io/crates/material-colors)
/// - Typography: Starlight
/// - Layout: PicoCSS
#[template]
pub fn DesignSystem(theme: Theme) -> impl Bundle {
	rsx! {
		<ColorScheme theme=theme />
		<Css />
	}
}
