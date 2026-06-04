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
		.insert(FontStyleProp)
		.insert(DecorationLineProp)
		.insert(WhiteSpaceProp)
		.insert(MarginProp)
		.insert(FlexGrowProp)
		.insert(AlignItemsProp)
		.insert(AlignContentProp)
		.insert(JustifyContentProp)
		.insert(FlexDirectionProp)
		.insert(FlexWrapProp)
		.insert(ColumnGapProp)
		.insert(RowGapProp)
		.insert(DisplayProp)
		.insert(BorderColorProp)
		.insert(BorderTopWidth)
		.insert(BorderRightWidth)
		.insert(BorderBottomWidth)
		.insert(BorderLeftWidth)
		.insert(BreakAfterProp)
		.insert(TransitionDurationProp)
		.insert(AnimationDurationProp)
}


css_property!(ForegroundColor, Color, "color");
css_property!(BackgroundColor, Color, TokenInheritance::NotInherited, "background-color");
css_property!(DecorationColor, Color, "text-decoration-color");
canonical_property!(TextAlignProp, TextAlign, "text-align");
canonical_property!(FontStyleProp, FontStyle, "font-style");
canonical_property!(BlinkStyleProp, BlinkStyle, "blink");
canonical_property!(VisibilityProp, Visibility, "visibility");
canonical_property!(DecorationLineProp, DecorationLine, "text-decoration-line");
canonical_property!(DecorationStyleProp, DecorationStyle, "text-decoration-style");

css_property!(Font, Typography, "font-family");

css_property!(Height, Length, "height");
css_property!(Width, Length, "width");
css_property!(Padding, Spacing, "padding");
css_property!(GapProp, Length, "gap");
css_property!(
	ShapeProp, Shape, "border-radius"
);
canonical_property!(
	ElevationProp, Elevation, "box-shadow"
);
css_property!(OutlineWidth, Length, "border-width");
css_property!(OutlineOffset, Length, "outline-offset");
css_property!(FontSize, Length, "font-size");
canonical_property!(FontWeightProp, FontWeight, "font-weight");
css_property!(LineHeight, Length, "line-height");
css_property!(Tracking, Length, "letter-spacing");

css_property!(FlexGrowProp, u32, TokenInheritance::NotInherited, "flex-grow");
css_property!(FlexOrderProp, i32, TokenInheritance::NotInherited, "order");
canonical_property!(AlignSelfProp, AlignSelf, TokenInheritance::NotInherited, "align-self");
canonical_property!(DisplayProp, Display, TokenInheritance::NotInherited, "display");
canonical_property!(BreakAfterProp, BreakAfter, TokenInheritance::NotInherited, "break-after");
css_property!(TransitionDurationProp, Duration, TokenInheritance::NotInherited, "transition-duration");
css_property!(AnimationDurationProp, Duration, TokenInheritance::NotInherited, "animation-duration");
canonical_property!(WhiteSpaceProp, WhiteSpace, "white-space");
css_property!(MarginProp, Spacing, TokenInheritance::NotInherited, "margin");
css_property!(BorderColorProp, Color, "border-color");
css_property!(BorderTopWidth, Length, TokenInheritance::NotInherited, "border-top-width");
css_property!(BorderRightWidth, Length, TokenInheritance::NotInherited, "border-right-width");
css_property!(BorderBottomWidth, Length, TokenInheritance::NotInherited, "border-bottom-width");
css_property!(BorderLeftWidth, Length, TokenInheritance::NotInherited, "border-left-width");

canonical_property!(JustifyContentProp, JustifyContent, TokenInheritance::NotInherited, "justify-content");
canonical_property!(AlignItemsProp, AlignItems, TokenInheritance::NotInherited, "align-items");
canonical_property!(AlignContentProp, AlignContent, TokenInheritance::NotInherited, "align-content");
canonical_property!(FlexDirectionProp, Direction, TokenInheritance::NotInherited, "flex-direction");
canonical_property!(FlexWrapProp, FlexWrap, TokenInheritance::NotInherited, "flex-wrap");
css_property!(RowGapProp, u32, TokenInheritance::NotInherited, "row-gap");
css_property!(ColumnGapProp, u32, TokenInheritance::NotInherited, "column-gap");
