#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::FontWeight;
use beet_core::prelude::*;

token!(ForegroundColor, Color, DocumentPath::Ancestor);
impl CssToken for ForegroundColor {
	fn properties() -> Vec<SmolStr> {
		vec!["color".into()]
	}
	fn declarations(
		value: &Value,
		builder: &CssBuilder,
	    ) -> Result<Vec<(String, String)>> {
    let color = value.into_reflect::<Color>()?.to_css_value(builder)?;
    Ok(vec![("color".into(), color)])
  }
}


token!(BackgroundColor, Color, DocumentPath::This);

token!(Font, Typography, DocumentPath::This)
;

token!(Height, Length, DocumentPath::This);
token!(Width, Length, DocumentPath::This);
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
// token!(Font, Typeface, DocumentPath::Ancestor);
token!(FontSize, Length, DocumentPath::Ancestor);
token!(
    /// Font weight property token, named to avoid conflict with the [`FontWeight`] type.
    FontWeightProp, FontWeight, DocumentPath::Ancestor
);
token!(LineHeight, Length, DocumentPath::Ancestor);
token!(Tracking, Length, DocumentPath::Ancestor);

pub fn css_ident_map() -> CssProperties {
	CssProperties::default()
		.with_property::<ForegroundColor>("color")
		.with_property::<BackgroundColor>("background-color")
		.with_property::<Height>("height")
		.with_property::<Width>("width")
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
