#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::{prelude::*, style::*};
use beet_core::prelude::Color;

token!(ForegroundColor, Color, DocumentPath::Ancestor);
token!(BackgroundColor, Color, DocumentPath::This);
token!(Height, Length, DocumentPath::This);
token!(Width, Length, DocumentPath::This);
token!(Size, Length, DocumentPath::This);
token!(Padding, Length, DocumentPath::This);
token!(Spacing, Length, DocumentPath::This);
token!(
    /// Shape property token, named to avoid conflict with the [`Shape`] type.
    ShapeProp, Shape, DocumentPath::This
);
token!(
    /// Elevation property token, named to avoid conflict with the [`Elevation`] type.
    ElevationProp, Elevation, DocumentPath::This
);
token!(OutlineWidth, Length, DocumentPath::This);
token!(OutlineOffset, Length, DocumentPath::This);
token!(Font, Typeface, DocumentPath::Ancestor);
token!(FontSize, Length, DocumentPath::Ancestor);
token!(
    /// Font weight property token, named to avoid conflict with the [`FontWeight`] type.
    FontWeightProp, FontWeight, DocumentPath::Ancestor
);
token!(LineHeight, Length, DocumentPath::Ancestor);
token!(Tracking, Length, DocumentPath::Ancestor);

pub fn css_key_map() -> CssIdentMap {
	CssIdentMap::default()
		.with_property::<ForegroundColor>("color")
		.with_property::<BackgroundColor>("background-color")
		.with_property::<Height>("height")
		.with_property::<Width>("width")
		.with_variable::<Size>("size")
		.with_property::<Padding>("padding")
		.with_property::<Spacing>("gap")
		.with_property::<ShapeProp>("border-radius")
		.with_property::<ElevationProp>("box-shadow")
		.with_property::<OutlineWidth>("border-width")
		.with_property::<OutlineOffset>("outline-offset")
		.with_property::<Font>("font-family")
		.with_property::<FontSize>("font-size")
		.with_property::<FontWeightProp>("font-weight")
		.with_property::<LineHeight>("line-height")
		.with_property::<Tracking>("letter-spacing")
}
