use crate::style::*;
use beet_core::prelude::*;

/// Stroke color of the text and other foreground elements.
pub const FOREGROUND_COLOR: PropertyDef<Color> =
	PropertyDef::new_static("color", true);
/// Fill of the background and other surfaces.
pub const BACKGROUND_COLOR: PropertyDef<Color> =
	PropertyDef::new_static("background-color", false);
