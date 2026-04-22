#![cfg_attr(rustfmt, rustfmt_skip)]
use bevy::color::palettes;
use crate::style::*;
use crate::token;

// ── Elevation tokens ──────────────────────────────────────────────────────────

token!(Elevation, ELEVATION_0, "elevation-0");
token!(Elevation, ELEVATION_1, "elevation-1");
token!(Elevation, ELEVATION_2, "elevation-2");
token!(Elevation, ELEVATION_3, "elevation-3");
token!(Elevation, ELEVATION_4, "elevation-4");
token!(Elevation, ELEVATION_5, "elevation-5");

// ── Shape corner radius ref tokens (Length) ───────────────────────────────────

token!(Length, SHAPE_CORNER_NONE,        "shape-corner-none");
token!(Length, SHAPE_CORNER_EXTRA_SMALL, "shape-corner-extra-small");
token!(Length, SHAPE_CORNER_SMALL,       "shape-corner-small");
token!(Length, SHAPE_CORNER_MEDIUM,      "shape-corner-medium");
token!(Length, SHAPE_CORNER_LARGE,       "shape-corner-large");
token!(Length, SHAPE_CORNER_EXTRA_LARGE, "shape-corner-extra-large");
token!(Length, SHAPE_CORNER_FULL,        "shape-corner-full");

// ── Shape sys tokens (composite Shape values) ─────────────────────────────────

// No rounding — sharp corners.
token!(Shape, SHAPE_NONE,        "shape-none");
token!(Shape, SHAPE_EXTRA_SMALL, "shape-extra-small");
token!(Shape, SHAPE_SMALL,       "shape-small");
token!(Shape, SHAPE_MEDIUM,      "shape-medium");
token!(Shape, SHAPE_LARGE,       "shape-large");
token!(Shape, SHAPE_EXTRA_LARGE, "shape-extra-large");
// Full pill / circle rounding.
token!(Shape, SHAPE_FULL,        "shape-full");

// ── Outline / border width tokens ────────────────────────────────────────────

token!(Length, OUTLINE_WIDTH_NONE,   "outline-width-none");
token!(Length, OUTLINE_WIDTH_THIN,   "outline-width-thin");
token!(Length, OUTLINE_WIDTH_MEDIUM, "outline-width-medium");
token!(Length, OUTLINE_WIDTH_THICK,  "outline-width-thick");


pub fn default_elevations() -> TokenStore {
	TokenStore::new()
		.with(ELEVATION_0, Elevation::default())
		.with(ELEVATION_1, Elevation {
			offset_x:     Length::Px(0.0),
			offset_y:     Length::Px(1.0),
			blur_radius:  Length::Px(3.0),
			spread_radius: Length::Px(1.0),
			color: palettes::basic::BLACK.into(),
		})
		.with(ELEVATION_2, Elevation {
			offset_x:     Length::Px(0.0),
			offset_y:     Length::Px(2.0),
			blur_radius:  Length::Px(6.0),
			spread_radius: Length::Px(2.0),
			color: palettes::basic::BLACK.into(),
		})
		.with(ELEVATION_3, Elevation {
			offset_x:     Length::Px(0.0),
			offset_y:     Length::Px(4.0),
			blur_radius:  Length::Px(8.0),
			spread_radius: Length::Px(3.0),
			color: palettes::basic::BLACK.into(),
		})
		.with(ELEVATION_4, Elevation {
			offset_x:     Length::Px(0.0),
			offset_y:     Length::Px(6.0),
			blur_radius:  Length::Px(10.0),
			spread_radius: Length::Px(4.0),
			color: palettes::basic::BLACK.into(),
		})
		.with(ELEVATION_5, Elevation {
			offset_x:     Length::Px(0.0),
			offset_y:     Length::Px(8.0),
			blur_radius:  Length::Px(12.0),
			spread_radius: Length::Px(6.0),
			color: palettes::basic::BLACK.into(),
		})
}

/// Returns a [`TokenStore`] with shape corner-radius ref tokens and composite
/// [`Shape`] sys tokens covering the full MD3 shape scale.
pub fn default_shapes() -> TokenStore {
	TokenStore::new()
		// corner length ref tokens
		.with(SHAPE_CORNER_NONE,        Length::Px(0.0))
		.with(SHAPE_CORNER_EXTRA_SMALL, Length::Px(4.0))
		.with(SHAPE_CORNER_SMALL,       Length::Px(8.0))
		.with(SHAPE_CORNER_MEDIUM,      Length::Px(12.0))
		.with(SHAPE_CORNER_LARGE,       Length::Px(16.0))
		.with(SHAPE_CORNER_EXTRA_LARGE, Length::Px(28.0))
		.with(SHAPE_CORNER_FULL,        Length::Percent(100.0))
		// composite shape sys tokens
		.with(SHAPE_NONE,        Shape { corner: SHAPE_CORNER_NONE,        edge: ShapeEdge::None })
		.with(SHAPE_EXTRA_SMALL, Shape { corner: SHAPE_CORNER_EXTRA_SMALL, edge: ShapeEdge::None })
		.with(SHAPE_SMALL,       Shape { corner: SHAPE_CORNER_SMALL,       edge: ShapeEdge::None })
		.with(SHAPE_MEDIUM,      Shape { corner: SHAPE_CORNER_MEDIUM,      edge: ShapeEdge::None })
		.with(SHAPE_LARGE,       Shape { corner: SHAPE_CORNER_LARGE,       edge: ShapeEdge::None })
		.with(SHAPE_EXTRA_LARGE, Shape { corner: SHAPE_CORNER_EXTRA_LARGE, edge: ShapeEdge::None })
		.with(SHAPE_FULL,        Shape { corner: SHAPE_CORNER_FULL,        edge: ShapeEdge::None })
		// outline width tokens
		.with(OUTLINE_WIDTH_NONE,   Length::Px(0.0))
		.with(OUTLINE_WIDTH_THIN,   Length::Px(1.0))
		.with(OUTLINE_WIDTH_MEDIUM, Length::Px(2.0))
		.with(OUTLINE_WIDTH_THICK,  Length::Px(3.0))
}
