#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::FontStyle;
use crate::style::FontWeight;
use crate::style::AlignSelf;
use crate::style::AlignItems;
use crate::style::JustifyContent;
use crate::style::Display;
use crate::style::Direction;
use crate::style::AlignContent;
use crate::style::FlexWrap;
use crate::style::Visibility;
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
		.insert(GapProp)
		.insert(ShapeProp)
		.insert(ElevationProp)
		.insert(OutlineWidth)
		.insert(OutlineOffset)
		.insert(FontSize)
		.insert(FontWeightProp)
		.insert(LineHeight)
		.insert(Tracking)
		.insert(TextAlignProp)
		.insert(DisplayProp)
		.insert(BorderColorProp)
		.insert(BreakAfterProp)
		.insert(TransitionDurationProp)
		.insert(AnimationDurationProp)
}


css_property!(ForegroundColor, Color, "color");
css_property!(BackgroundColor, Color, TokenInheritance::NotInherited, "background-color");
css_property!(DecorationColor, Color, "text-decoration-color");
css_property!(TextAlignProp, TextAlign, "text-align");
css_property!(FontStyleProp, FontStyle, "font-style");
css_property!(BlinkStyleProp, BlinkStyle, "blink");
css_property!(VisibilityProp, Visibility, "visibility");
css_property!(DecorationLineProp, DecorationLine, "text-decoration-line");
css_property!(DecorationStyleProp, DecorationStyle, "text-decoration-style");

css_property!(Font, Typography, "font-family");

css_property!(Height, Length, "height");
css_property!(Width, Length, "width");
css_property!(Padding, Spacing, "padding");
css_property!(GapProp, Length, "gap");
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
css_property!(BreakAfterProp, BreakAfter, TokenInheritance::NotInherited, "break-after");
css_property!(TransitionDurationProp, Duration, TokenInheritance::NotInherited, "transition-duration");
css_property!(AnimationDurationProp, Duration, TokenInheritance::NotInherited, "animation-duration");
css_property!(WhiteSpaceProp, WhiteSpace, "white-space");
css_property!(MarginProp, Spacing, TokenInheritance::NotInherited, "margin");
css_property!(BorderColorProp, Color, "border-color");

css_property!(JustifyContentProp, JustifyContent, TokenInheritance::NotInherited, "justify-content");
css_property!(AlignItemsProp, AlignItems, TokenInheritance::NotInherited, "align-items");
css_property!(AlignContentProp, AlignContent, TokenInheritance::NotInherited, "align-content");
css_property!(FlexDirectionProp, Direction, TokenInheritance::NotInherited, "flex-direction");
css_property!(FlexWrapProp, FlexWrap, TokenInheritance::NotInherited, "flex-wrap");
css_property!(RowGapProp, u32, TokenInheritance::NotInherited, "row-gap");
css_property!(ColumnGapProp, u32, TokenInheritance::NotInherited, "column-gap");
