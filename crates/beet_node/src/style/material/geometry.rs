#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

// ── Elevation tokens ──────────────────────────────────────────────────────────

css_variable!(Elevation0, Elevation);
css_variable!(Elevation1, Elevation);
css_variable!(Elevation2, Elevation);
css_variable!(Elevation3, Elevation);
css_variable!(Elevation4, Elevation);
css_variable!(Elevation5, Elevation);

// ── Shape corner radius ref tokens ────────────────────────────────────────────

css_variable!(ShapeCornerNone,       Length);
css_variable!(ShapeCornerExtraSmall, Length);
css_variable!(ShapeCornerSmall,      Length);
css_variable!(ShapeCornerMedium,     Length);
css_variable!(ShapeCornerLarge,      Length);
css_variable!(ShapeCornerExtraLarge, Length);
css_variable!(ShapeCornerFull,       Length);

// ── Shape sys tokens ──────────────────────────────────────────────────────────

css_variable!(ShapeNone,       Shape);
css_variable!(ShapeExtraSmall, Shape);
css_variable!(ShapeSmall,      Shape);
css_variable!(ShapeMedium,     Shape);
css_variable!(ShapeLarge,      Shape);
css_variable!(ShapeExtraLarge, Shape);
css_variable!(ShapeFull,       Shape);

// ── Outline width tokens ──────────────────────────────────────────────────────

css_variable!(OutlineWidthNone,   Length);
css_variable!(OutlineWidthThin,   Length);
css_variable!(OutlineWidthMedium, Length);
css_variable!(OutlineWidthThick,  Length);

pub fn token_map() -> CssTokenMap {
	CssTokenMap::default()
		.insert(Elevation0)
		.insert(Elevation1)
		.insert(Elevation2)
		.insert(Elevation3)
		.insert(Elevation4)
		.insert(Elevation5)

		.insert(ShapeCornerNone)
		.insert(ShapeCornerExtraSmall)
		.insert(ShapeCornerSmall)
		.insert(ShapeCornerMedium)
		.insert(ShapeCornerLarge)
		.insert(ShapeCornerExtraLarge)
		.insert(ShapeCornerFull)

		.insert(ShapeNone)
		.insert(ShapeExtraSmall)
		.insert(ShapeSmall)
		.insert(ShapeMedium)
		.insert(ShapeLarge)
		.insert(ShapeExtraLarge)
		.insert(ShapeFull)

		.insert(OutlineWidthNone)
		.insert(OutlineWidthThin)
		.insert(OutlineWidthMedium)
		.insert(OutlineWidthThick)
}


/// Returns a [`Selector`] with all MD3 elevation default values.
pub fn default_elevations() -> Selector {
	let elevation_color = Color::srgba(0., 0., 0., 0.2);

	Selector::root()
		.with_value::<Elevation0>(Elevation::default()).unwrap()
		.with_value::<Elevation1>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(1.0),
			blur_radius:   Length::Px(3.0),
			spread_radius: Length::Px(1.0),
			color:         elevation_color.clone(),
		}).unwrap()
		.with_value::<Elevation2>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(2.0),
			blur_radius:   Length::Px(6.0),
			spread_radius: Length::Px(2.0),
			color:         elevation_color.clone(),
		}).unwrap()
		.with_value::<Elevation3>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(4.0),
			blur_radius:   Length::Px(8.0),
			spread_radius: Length::Px(3.0),
			color:         elevation_color.clone(),
		}).unwrap()
		.with_value::<Elevation4>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(6.0),
			blur_radius:   Length::Px(10.0),
			spread_radius: Length::Px(4.0),
			color:         elevation_color.clone(),
		}).unwrap()
		.with_value::<Elevation5>(Elevation {
			offset_x:      Length::Px(0.0),
			offset_y:      Length::Px(8.0),
			blur_radius:   Length::Px(12.0),
			spread_radius: Length::Px(6.0),
			color:         elevation_color.clone(),
		}).unwrap()
}

/// Returns a [`Selector`] with all MD3 shape corner, composite shape, and
/// outline width default values.
pub fn default_shapes() -> Selector {
	Selector::root()
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
