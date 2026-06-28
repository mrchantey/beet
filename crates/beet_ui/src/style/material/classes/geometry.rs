//! Card, shape and elevation classes and their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const CARD_FILLED: ClassName = ClassName::new_static("card-filled");
pub const CARD_ELEVATED: ClassName = ClassName::new_static("card-elevated");
pub const CARD_OUTLINED: ClassName = ClassName::new_static("card-outlined");

pub const SHAPE_NONE: ClassName = ClassName::new_static("shape-none");
pub const SHAPE_EXTRA_SMALL: ClassName = ClassName::new_static("shape-xs");
pub const SHAPE_SMALL: ClassName = ClassName::new_static("shape-sm");
pub const SHAPE_MEDIUM: ClassName = ClassName::new_static("shape-md");
pub const SHAPE_LARGE: ClassName = ClassName::new_static("shape-lg");
pub const SHAPE_EXTRA_LARGE: ClassName = ClassName::new_static("shape-xl");
pub const SHAPE_FULL: ClassName = ClassName::new_static("shape-full");

pub const ELEVATION_0: ClassName = ClassName::new_static("elevation-0");
pub const ELEVATION_1: ClassName = ClassName::new_static("elevation-1");
pub const ELEVATION_2: ClassName = ClassName::new_static("elevation-2");
pub const ELEVATION_3: ClassName = ClassName::new_static("elevation-3");
pub const ELEVATION_4: ClassName = ClassName::new_static("elevation-4");
pub const ELEVATION_5: ClassName = ClassName::new_static("elevation-5");

// ── Cards ─────────────────────────────────────────────────────────────────────

/// Filled card - container with the highest surface elevation.
///
/// Uses surface-container-highest background, no shadow. A distinct surface tone
/// (lifted off the page's `Background`) on both the web and the terminal.
pub fn card_filled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_FILLED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHighest).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
}

/// Elevated card - container with shadow elevation.
///
/// Surface container with level 1 shadow for subtle lift.
pub fn card_elevated() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_ELEVATED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerLow).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation1).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
}

/// Outlined card - container with visible border.
///
/// Surface background with outline border, no shadow.
pub fn card_outlined() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_OUTLINED))
		.with_token(common_props::BackgroundColor,colors::Surface).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::OutlineWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
}

// ── Shape utility classes ─────────────────────────────────────────────────────

/// No border radius.
pub fn shape_none() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_NONE))
		.with_token(ShapeProps,geometry::ShapeNone).unwrap()
}

/// Extra small border radius (4px).
pub fn shape_extra_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_EXTRA_SMALL))
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
}

/// Small border radius (8px).
pub fn shape_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_SMALL))
		.with_token(ShapeProps,geometry::ShapeSmall).unwrap()
}

/// Medium border radius (12px).
pub fn shape_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_MEDIUM))
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
}

/// Large border radius (16px).
pub fn shape_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_LARGE))
		.with_token(ShapeProps,geometry::ShapeLarge).unwrap()
}

/// Extra large border radius (28px).
pub fn shape_extra_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_EXTRA_LARGE))
		.with_token(ShapeProps,geometry::ShapeExtraLarge).unwrap()
}

/// Full border radius (pill/circular).
pub fn shape_full() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_FULL))
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
}

// ── Elevation utility classes ─────────────────────────────────────────────────

/// No elevation shadow.
pub fn elevation_0() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_0))
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Level 1 elevation shadow.
pub fn elevation_1() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_1))
		.with_token(common_props::ElevationProp,geometry::Elevation1).unwrap()
}

/// Level 2 elevation shadow.
pub fn elevation_2() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_2))
		.with_token(common_props::ElevationProp,geometry::Elevation2).unwrap()
}

/// Level 3 elevation shadow.
pub fn elevation_3() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_3))
		.with_token(common_props::ElevationProp,geometry::Elevation3).unwrap()
}

/// Level 4 elevation shadow.
pub fn elevation_4() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_4))
		.with_token(common_props::ElevationProp,geometry::Elevation4).unwrap()
}

/// Level 5 elevation shadow.
pub fn elevation_5() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_5))
		.with_token(common_props::ElevationProp,geometry::Elevation5).unwrap()
}
