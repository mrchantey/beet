#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

// ── Elevation tokens ──────────────────────────────────────────────────────────

token!(Elevation0, Elevation);
token!(Elevation1, Elevation);
token!(Elevation2, Elevation);
token!(Elevation3, Elevation);
token!(Elevation4, Elevation);
token!(Elevation5, Elevation);

// ── Shape corner radius ref tokens ────────────────────────────────────────────

token!(ShapeCornerNone,       Length);
token!(ShapeCornerExtraSmall, Length);
token!(ShapeCornerSmall,      Length);
token!(ShapeCornerMedium,     Length);
token!(ShapeCornerLarge,      Length);
token!(ShapeCornerExtraLarge, Length);
token!(ShapeCornerFull,       Length);

// ── Shape sys tokens ──────────────────────────────────────────────────────────

token!(ShapeNone,       Shape);
token!(ShapeExtraSmall, Shape);
token!(ShapeSmall,      Shape);
token!(ShapeMedium,     Shape);
token!(ShapeLarge,      Shape);
token!(ShapeExtraLarge, Shape);
token!(ShapeFull,       Shape);

// ── Outline width tokens ──────────────────────────────────────────────────────

token!(OutlineWidthNone,   Length);
token!(OutlineWidthThin,   Length);
token!(OutlineWidthMedium, Length);
token!(OutlineWidthThick,  Length);

/// Returns a [`Selector`] with all MD3 elevation default values.
pub fn default_elevations() -> Selector {
	Selector::new()
		.with_value::<Elevation0>(Elevation::default()).unwrap()
		.with_value::<Elevation1>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(1.0),
			blur_radius:   Length::Px(3.0),
			spread_radius: Length::Px(1.0),
			color:         palettes::basic::BLACK.into(),
		}).unwrap()
		.with_value::<Elevation2>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(2.0),
			blur_radius:   Length::Px(6.0),
			spread_radius: Length::Px(2.0),
			color:         palettes::basic::BLACK.into(),
		}).unwrap()
		.with_value::<Elevation3>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(4.0),
			blur_radius:   Length::Px(8.0),
			spread_radius: Length::Px(3.0),
			color:         palettes::basic::BLACK.into(),
		}).unwrap()
		.with_value::<Elevation4>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(6.0),
			blur_radius:   Length::Px(10.0),
			spread_radius: Length::Px(4.0),
			color:         palettes::basic::BLACK.into(),
		}).unwrap()
		.with_value::<Elevation5>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(8.0),
			blur_radius:   Length::Px(12.0),
			spread_radius: Length::Px(6.0),
			color:         palettes::basic::BLACK.into(),
		}).unwrap()
}

/// Returns a [`Selector`] with all MD3 shape corner, composite shape, and
/// outline width default values.
pub fn default_shapes() -> Selector {
	Selector::new()
		// corner length ref tokens
		.with_value::<ShapeCornerNone>(Length::Px(0.0)).unwrap()
		.with_value::<ShapeCornerExtraSmall>(Length::Px(4.0)).unwrap()
		.with_value::<ShapeCornerSmall>(Length::Px(8.0)).unwrap()
		.with_value::<ShapeCornerMedium>(Length::Px(12.0)).unwrap()
		.with_value::<ShapeCornerLarge>(Length::Px(16.0)).unwrap()
		.with_value::<ShapeCornerExtraLarge>(Length::Px(28.0)).unwrap()
		.with_value::<ShapeCornerFull>(Length::Percent(100.0)).unwrap()
		// composite shape sys tokens
		.with_value::<ShapeNone>(Shape       { corner: Length::Px(0.0),          edge: ShapeEdge::None }).unwrap()
		.with_value::<ShapeExtraSmall>(Shape  { corner: Length::Px(4.0),          edge: ShapeEdge::None }).unwrap()
		.with_value::<ShapeSmall>(Shape       { corner: Length::Px(8.0),          edge: ShapeEdge::None }).unwrap()
		.with_value::<ShapeMedium>(Shape      { corner: Length::Px(12.0),         edge: ShapeEdge::None }).unwrap()
		.with_value::<ShapeLarge>(Shape       { corner: Length::Px(16.0),         edge: ShapeEdge::None }).unwrap()
		.with_value::<ShapeExtraLarge>(Shape  { corner: Length::Px(28.0),         edge: ShapeEdge::None }).unwrap()
		.with_value::<ShapeFull>(Shape        { corner: Length::Percent(100.0),   edge: ShapeEdge::None }).unwrap()
		// outline width tokens
		.with_value::<OutlineWidthNone>(Length::Px(0.0)).unwrap()
		.with_value::<OutlineWidthThin>(Length::Px(1.0)).unwrap()
		.with_value::<OutlineWidthMedium>(Length::Px(2.0)).unwrap()
		.with_value::<OutlineWidthThick>(Length::Px(3.0)).unwrap()
}
