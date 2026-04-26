pub mod colors;
pub mod geometry;
pub mod material_plugin;
pub mod motion;
pub mod selectors;
pub mod themes;
pub mod tones;
pub mod typography;
pub use geometry::*;
pub use material_plugin::*;
pub use motion::*;
pub use typography::*;

use crate::style::CssTokenMap;

pub fn token_map() -> CssTokenMap {
	CssTokenMap::default()
		.merge(tones::token_map())
		.merge(colors::token_map())
		.merge(geometry::token_map())
	// .merge(motion::token_map())
	// .merge(typography::token_map())
}
