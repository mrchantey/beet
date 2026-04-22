use crate::style::*;
use beet_core::prelude::*;

/// Stroke color of the text and other foreground elements.
pub const FOREGROUND_COLOR: PropertyDef =
	PropertyDef::new_static::<Color>("color", true);
/// Fill of the background and other surfaces.
pub const BACKGROUND_COLOR: PropertyDef =
	PropertyDef::new_static::<Color>("background-color", false);

// Layout
pub const HEIGHT: PropertyDef =
	PropertyDef::new_static::<Unit>("height", false);
pub const WIDTH: PropertyDef = PropertyDef::new_static::<Unit>("width", false);
/// Applies to both width and height simultaneously.
pub const SIZE: PropertyDef = PropertyDef::new_static::<Unit>("--size", false);
pub const PADDING: PropertyDef =
	PropertyDef::new_static::<Unit>("padding", false);
pub const SPACING: PropertyDef = PropertyDef::new_static::<Unit>("gap", false);

// Shape
pub const SHAPE: PropertyDef =
	PropertyDef::new_static::<Shape>("border-radius", false);

// Elevation
pub const ELEVATION: PropertyDef =
	PropertyDef::new_static::<Elevation>("box-shadow", false);

// Outline
pub const OUTLINE_WIDTH: PropertyDef =
	PropertyDef::new_static::<Unit>("border-width", false);
pub const OUTLINE_OFFSET: PropertyDef =
	PropertyDef::new_static::<Unit>("outline-offset", false);

// Typography (inherited)
pub const FONT: PropertyDef =
	PropertyDef::new_static::<Typeface>("font-family", true);
pub const FONT_SIZE: PropertyDef =
	PropertyDef::new_static::<Unit>("font-size", true);
pub const FONT_WEIGHT: PropertyDef =
	PropertyDef::new_static::<FontWeight>("font-weight", true);
pub const LINE_HEIGHT: PropertyDef =
	PropertyDef::new_static::<Unit>("line-height", true);
pub const TRACKING: PropertyDef =
	PropertyDef::new_static::<Unit>("letter-spacing", true);
