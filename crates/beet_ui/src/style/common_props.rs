#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::FontWeight;
use crate::style::*;
use beet_core::prelude::*;

pub fn token_map()->CssTokenMap{
	CssTokenMap::default()
		.insert(ForegroundColor)
		.insert(BackgroundColor)
		.insert(ColorRoleProps)
		.insert(Font)
		.insert(Height)
		.insert(Width)
		.insert(Padding)
		.insert(Spacing)
		.insert(ShapeProp)
		.insert(ElevationProp)
		.insert(OutlineWidth)
		.insert(OutlineOffset)
		.insert(FontSize)
		.insert(FontWeightProp)
		.insert(LineHeight)
		.insert(Tracking)
}


css_property!(ForegroundColor, Color, "color");
css_property!(BackgroundColor, Color, "background-color");

css_property!(Font, Typography, "font-family");

css_property!(Height, Length, "height");
css_property!(Width, Length, "width");
css_property!(Padding, Length, "padding");
css_property!(Spacing, Length, "gap");
css_property!(
	/// Shape property token, named to avoid conflict with the [`Shape`] type.
	ShapeProp, Shape, "border-radius"
);
css_property!(
	/// Elevation property token, named to avoid conflict with the [`Elevation`] type.
	ElevationProp, Elevation, "box-shadow"
);
css_property!(OutlineWidth, Length, "border-width");
css_property!(OutlineOffset, Length, "outline-offset");
css_property!(FontSize, Length, "font-size");
css_property!(
	/// Font weight property token, named to avoid conflict with the [`FontWeight`] type.
	FontWeightProp, FontWeight, "font-weight"
);
css_property!(LineHeight, Length, "line-height");
css_property!(Tracking, Length, "letter-spacing");
