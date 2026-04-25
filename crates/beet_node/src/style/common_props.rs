#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::{prelude::*, style::*};
use beet_core::prelude::Color;

token2!(ForegroundColor, Color, DocumentPath::Ancestor);
token2!(BackgroundColor, Color, DocumentPath::This);
token2!(Height, Length, DocumentPath::This);
token2!(Width, Length, DocumentPath::This);
token2!(Size, Length, DocumentPath::This);
token2!(Padding, Length, DocumentPath::This);
token2!(Spacing, Length, DocumentPath::This);
token2!(
    /// Shape property token, named to avoid conflict with the [`Shape`] type.
    ShapeProp, Shape, DocumentPath::This
);
token2!(
    /// Elevation property token, named to avoid conflict with the [`Elevation`] type.
    ElevationProp, Elevation, DocumentPath::This
);
token2!(OutlineWidth, Length, DocumentPath::This);
token2!(OutlineOffset, Length, DocumentPath::This);
token2!(Font, Typeface, DocumentPath::Ancestor);
token2!(FontSize, Length, DocumentPath::Ancestor);
token2!(
    /// Font weight property token, named to avoid conflict with the [`FontWeight`] type.
    FontWeightProp, FontWeight, DocumentPath::Ancestor
);
token2!(LineHeight, Length, DocumentPath::Ancestor);
token2!(Tracking, Length, DocumentPath::Ancestor);

pub fn css_key_map() -> CssKeyMap {
	CssKeyMap::default()
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
