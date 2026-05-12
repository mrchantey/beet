#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::FontWeight;
use crate::style::AlignSelf;
use crate::style::Display;
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
css_property!(BackgroundColor, Color, TokenInheritance::NotInherited, "background-color");
css_property!(DecorationColor, Color, "text-decoration-color");
css_property!(TextAlignProp, TextAlign, "text-align");
css_property!(TextStyleProp, TextStyle, "text-style");
css_property!(DecorationLineProp, DecorationLine, "text-decoration-line");
css_property!(DecorationStyleProp, DecorationStyle, "text-decoration-style");

css_property!(Font, Typography, "font-family");

css_property!(Height, Length, "height");
css_property!(Width, Length, "width");
css_property!(Padding, Length, "padding");
css_property!(Spacing, Length, "gap");
css_property!(
	ShapeProp, Shape, "border-radius"
);
css_property!(
	ElevationProp, Elevation, "box-shadow"
);
css_property!(OutlineWidth, Length, "border-width");
css_property!(OutlineOffset, Length, "outline-offset");
css_property!(FontSize, Length, "font-size");
css_property!(FontWeightProp, FontWeight, "font-weight");
css_property!(LineHeight, Length, "line-height");
css_property!(Tracking, Length, "letter-spacing");

css_property!(FlexGrowProp, u32, TokenInheritance::NotInherited, "flex-grow");
css_property!(FlexOrderProp, i32, TokenInheritance::NotInherited, "order");
css_property!(AlignSelfProp, AlignSelf, TokenInheritance::NotInherited, "align-self");
css_property!(DisplayProp, Display, TokenInheritance::NotInherited, "display");
css_property!(MarginProp, Length, TokenInheritance::NotInherited, "margin");
css_property!(BorderColorProp, Color, "border-color");
