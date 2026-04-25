#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::Color;


token2!(BackgroundColor2, Color, DocumentPath::This);
token2!(ForegroundColor2, Color, DocumentPath::Ancestor);


/// Stroke color of the text and other foreground elements.
pub const FOREGROUND_COLOR: PropertyDef = PropertyDef::new_static::<Color>("color", true);
/// Fill of the background and other surfaces.
pub const BACKGROUND_COLOR: PropertyDef = PropertyDef::new_static::<Color>("background-color", false);

// Layout
pub const HEIGHT: PropertyDef = PropertyDef::new_static::<Length>("height", false);
pub const WIDTH: PropertyDef = PropertyDef::new_static::<Length>("width", false);
/// Applies to both width and height simultaneously.
pub const SIZE: PropertyDef = PropertyDef::new_static::<Length>("--size", false);
pub const PADDING: PropertyDef = PropertyDef::new_static::<Length>("padding", false);
pub const SPACING: PropertyDef = PropertyDef::new_static::<Length>("gap", false);

// Shape
pub const SHAPE: PropertyDef = PropertyDef::new_static::<Shape>("border-radius", false);

// Elevation
pub const ELEVATION: PropertyDef = PropertyDef::new_static::<Elevation>("box-shadow", false);

// Outline
pub const OUTLINE_WIDTH: PropertyDef = PropertyDef::new_static::<Length>("border-width", false);
pub const OUTLINE_OFFSET: PropertyDef = PropertyDef::new_static::<Length>("outline-offset", false);

// Typography (inherited)
pub const FONT: PropertyDef = PropertyDef::new_static::<Typeface>("font-family", true);
pub const FONT_SIZE: PropertyDef = PropertyDef::new_static::<Length>("font-size", true);
pub const FONT_WEIGHT: PropertyDef = PropertyDef::new_static::<FontWeight>("font-weight", true);
pub const LINE_HEIGHT: PropertyDef = PropertyDef::new_static::<Length>("line-height", true);
pub const TRACKING: PropertyDef = PropertyDef::new_static::<Length>("letter-spacing", true);
