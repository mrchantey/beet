#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::FontWeight;
use crate::style::*;
use beet_core::prelude::*;

pub fn token_map()->CssTokenMap{
	CssTokenMap::default()
		.insert::<ForegroundColor>()
		.insert::<BackgroundColor>()
		.insert::<Font>()
		.insert::<Height>()
		.insert::<Width>()
		.insert::<Padding>()
		.insert::<Spacing>()
		.insert::<ShapeProp>()
		.insert::<ElevationProp>()
		.insert::<OutlineWidth>()
		.insert::<OutlineOffset>()
		.insert::<FontSize>()
		.insert::<FontWeightProp>()
		.insert::<LineHeight>()
		.insert::<Tracking>()
}


css_property!(ForegroundColor, Color, DocumentPath::Ancestor, "color");
css_property!(BackgroundColor, Color, DocumentPath::This, "background-color");

css_property!(Font, Typography, DocumentPath::This, "font-family");

css_property!(Height, Length, DocumentPath::This, "height");
css_property!(Width, Length, DocumentPath::This, "width");
css_property!(Padding, Length, DocumentPath::This, "padding");
css_property!(Spacing, Length, DocumentPath::This, "gap");
css_property!(
	/// Shape property token, named to avoid conflict with the [`Shape`] type.
	ShapeProp, Shape, DocumentPath::This, "border-radius"
);
css_property!(
	/// Elevation property token, named to avoid conflict with the [`Elevation`] type.
	ElevationProp, Elevation, DocumentPath::This, "box-shadow"
);
css_property!(OutlineWidth, Length, DocumentPath::This, "border-width");
css_property!(OutlineOffset, Length, DocumentPath::This, "outline-offset");
css_property!(FontSize, Length, DocumentPath::Ancestor, "font-size");
css_property!(
	/// Font weight property token, named to avoid conflict with the [`FontWeight`] type.
	FontWeightProp, FontWeight, DocumentPath::Ancestor, "font-weight"
);
css_property!(LineHeight, Length, DocumentPath::Ancestor, "line-height");
css_property!(Tracking, Length, DocumentPath::Ancestor, "letter-spacing");
