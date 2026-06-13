#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::FontStyle;
use crate::style::FontWeight;
use crate::style::Transform;
use crate::style::AlignSelf;
use crate::style::AlignItems;
use crate::style::JustifyContent;
use crate::style::Display;
use crate::style::Direction;
use crate::style::AlignContent;
use crate::style::FlexWrap;
use crate::style::Visibility;
use crate::style::Overflow;
use crate::style::*;
use beet_core::prelude::*;

pub fn token_map()->CssTokenMap{
	CssTokenMap::default()
		.insert(ForegroundColor)
		.insert(BackgroundColor)
		.insert(ColorRoleProps)
		.insert(Font)
		.insert(Height)
		.insert(MinHeight)
		.insert(MaxHeight)
		.insert(Width)
		.insert(MinWidth)
		.insert(MaxWidth)
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
		.insert(GridTemplateColumnsProp)
		.insert(GridAutoRowsProp)
		.insert(ListStyleProp)
		.insert(OverflowXProp)
		.insert(OverflowYProp)
		.insert(PositionProp)
		.insert(InsetTop)
		.insert(InsetRight)
		.insert(InsetBottom)
		.insert(InsetLeft)
		.insert(ZIndexProp)
		.insert(ScrollbarWidthProp)
		.insert(ScrollbarColorProp)
		.insert(BorderColorProp)
		.insert(BorderTopWidth)
		.insert(BorderRightWidth)
		.insert(BorderBottomWidth)
		.insert(BorderLeftWidth)
		.insert(BreakAfterProp)
		.insert(CursorProp)
		.insert(TransitionDurationProp)
		.insert(TransitionEaseProp)
		.insert(AnimationDurationProp)
		.insert(TransformProp)
		.insert(OpacityProp)
}


// Inheritance mirrors CSS: layout/box props (padding, margin, width/height,
// border-*, border-radius, box-shadow, gap, outline, display, flex-*, transform)
// are NOT inherited; text props (color, font-*, line-height, letter-spacing,
// text-align, white-space, list-style, visibility) are. The exception is the
// text-decoration trio: CSS doesn't inherit it but paints it through in-flow
// descendants, which this renderer models as inheritance so an underline reaches
// nested spans.
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

css_property!(Height, Length, TokenInheritance::NotInherited, "height");
css_property!(MinHeight, Length, TokenInheritance::NotInherited, "min-height");
css_property!(MaxHeight, Length, TokenInheritance::NotInherited, "max-height");
css_property!(Width, Length, TokenInheritance::NotInherited, "width");
css_property!(MinWidth, Length, TokenInheritance::NotInherited, "min-width");
css_property!(MaxWidth, Length, TokenInheritance::NotInherited, "max-width");
css_property!(Padding, Spacing, TokenInheritance::NotInherited, "padding");
css_property!(GapProp, Length, TokenInheritance::NotInherited, "gap");
css_property!(ShapeProp, Shape, TokenInheritance::NotInherited, "border-radius");
canonical_property!(ElevationProp, Elevation, TokenInheritance::NotInherited, "box-shadow");
css_property!(OutlineWidth, Length, TokenInheritance::NotInherited, "border-width");
css_property!(OutlineOffset, Length, TokenInheritance::NotInherited, "outline-offset");
css_property!(FontSize, Length, "font-size");
canonical_property!(FontWeightProp, FontWeight, "font-weight");
css_property!(LineHeight, Length, "line-height");
css_property!(Tracking, Length, "letter-spacing");

css_property!(FlexGrowProp, u32, TokenInheritance::NotInherited, "flex-grow");
css_property!(FlexOrderProp, i32, TokenInheritance::NotInherited, "order");
canonical_property!(AlignSelfProp, AlignSelf, TokenInheritance::NotInherited, "align-self");
canonical_property!(DisplayProp, Display, TokenInheritance::NotInherited, "display");
canonical_property!(BreakAfterProp, BreakAfter, TokenInheritance::NotInherited, "break-after");
canonical_property!(CursorProp, Cursor, TokenInheritance::NotInherited, "cursor");
canonical_property!(TransformProp, Transform, TokenInheritance::NotInherited, "transform");
css_property!(TransitionDurationProp, Duration, TokenInheritance::NotInherited, "transition-duration");
// Easing for style transitions; pairs with `TransitionDurationProp` to drive
// the charcell `VisualTransition` (see `style::animate`).
css_property!(TransitionEaseProp, EaseFunction, TokenInheritance::NotInherited, "transition-timing-function");
// Whole-element opacity, driving the interactive hover/active dim. A unitless
// `f32` in `[0,1]`; the charcell target approximates it by blending colours
// toward the element surface (see `VisualStyle::apply_opacity`).
css_property!(OpacityProp, f32, TokenInheritance::NotInherited, "opacity");
css_property!(AnimationDurationProp, Duration, TokenInheritance::NotInherited, "animation-duration");
canonical_property!(WhiteSpaceProp, WhiteSpace, "white-space");
canonical_property!(ListStyleProp, ListStyle, "list-style-type");
// overflow-x/-y share the `Overflow` value type, so neither can be the single
// canonical token for it; author rules with `with_value(OverflowXProp, ..)`.
css_property!(OverflowXProp, Overflow, TokenInheritance::NotInherited, "overflow-x");
css_property!(OverflowYProp, Overflow, TokenInheritance::NotInherited, "overflow-y");
canonical_property!(PositionProp, Position, TokenInheritance::NotInherited, "position");
// inset properties; all four feed the same `Length` value type, so plain props.
css_property!(InsetTop, Length, TokenInheritance::NotInherited, "top");
css_property!(InsetRight, Length, TokenInheritance::NotInherited, "right");
css_property!(InsetBottom, Length, TokenInheritance::NotInherited, "bottom");
css_property!(InsetLeft, Length, TokenInheritance::NotInherited, "left");
css_property!(ZIndexProp, i32, TokenInheritance::NotInherited, "z-index");
canonical_property!(ScrollbarWidthProp, ScrollbarWidth, TokenInheritance::NotInherited, "scrollbar-width");
canonical_property!(ScrollbarColorProp, ScrollbarColor, TokenInheritance::NotInherited, "scrollbar-color");
css_property!(MarginProp, Spacing, TokenInheritance::NotInherited, "margin");
css_property!(BorderColorProp, Color, TokenInheritance::NotInherited, "border-color");
css_property!(BorderTopWidth, Length, TokenInheritance::NotInherited, "border-top-width");
css_property!(BorderRightWidth, Length, TokenInheritance::NotInherited, "border-right-width");
css_property!(BorderBottomWidth, Length, TokenInheritance::NotInherited, "border-bottom-width");
css_property!(BorderLeftWidth, Length, TokenInheritance::NotInherited, "border-left-width");

canonical_property!(GridTemplateColumnsProp, GridColumns, TokenInheritance::NotInherited, "grid-template-columns");
canonical_property!(GridAutoRowsProp, GridRows, TokenInheritance::NotInherited, "grid-auto-rows");
canonical_property!(JustifyContentProp, JustifyContent, TokenInheritance::NotInherited, "justify-content");
canonical_property!(AlignItemsProp, AlignItems, TokenInheritance::NotInherited, "align-items");
canonical_property!(AlignContentProp, AlignContent, TokenInheritance::NotInherited, "align-content");
canonical_property!(FlexDirectionProp, Direction, TokenInheritance::NotInherited, "flex-direction");
canonical_property!(FlexWrapProp, FlexWrap, TokenInheritance::NotInherited, "flex-wrap");
css_property!(RowGapProp, Length, TokenInheritance::NotInherited, "row-gap");
css_property!(ColumnGapProp, Length, TokenInheritance::NotInherited, "column-gap");
