#![cfg_attr(rustfmt, rustfmt_skip)]
use bevy::color::palettes;
use crate::style::*;
use crate::token;

token!(Elevation, ELEVATION_0, "elevation-0");
token!(Elevation, ELEVATION_1, "elevation-1");
token!(Elevation, ELEVATION_2, "elevation-2");
token!(Elevation, ELEVATION_3, "elevation-3");
token!(Elevation, ELEVATION_4, "elevation-4");
token!(Elevation, ELEVATION_5, "elevation-5");


token!(Length, SHAPE_CORNER_EXTRA_SMALL, "shape-corner-extra-small");
token!(Length, SHAPE_CORNER_SMALL, "shape-corner-small");
token!(Length, SHAPE_CORNER_MEDIUM, "shape-corner-medium");
token!(Length, SHAPE_CORNER_LARGE, "shape-corner-large");
token!(Length, SHAPE_CORNER_EXTRA_LARGE, "shape-corner-extra-large");
token!(Length, SHAPE_CORNER_FULL, "shape-corner-full");


pub fn default_shape_corners()->TokenStore{
	TokenStore::new()
		.with(SHAPE_CORNER_EXTRA_SMALL, Length::Px(4.0))
		.with(SHAPE_CORNER_SMALL, Length::Px(8.0))
		.with(SHAPE_CORNER_MEDIUM, Length::Px(12.0))
		.with(SHAPE_CORNER_LARGE, Length::Px(16.0))
		.with(SHAPE_CORNER_EXTRA_LARGE, Length::Px(28.0))
		.with(SHAPE_CORNER_FULL, Length::Percent(100.0))
}


pub fn default_elevations() -> TokenStore {
TokenStore::new()
	.with(ELEVATION_0, Elevation::default())
	.with(ELEVATION_1, Elevation {
    offset_x: Length::Px(0.0),
    offset_y: Length::Px(1.0),
    blur_radius: Length::Px(3.0),
    spread_radius: Length::Px(1.0),
    color: palettes::basic::BLACK.into(),
	})
	.with(ELEVATION_2, Elevation {
    offset_x: Length::Px(0.0),
    offset_y: Length::Px(2.0),
    blur_radius: Length::Px(6.0),
    spread_radius: Length::Px(2.0),
    color: palettes::basic::BLACK.into()
	})
	.with(ELEVATION_3, Elevation {
		offset_x: Length::Px(0.0),
    offset_y: Length::Px(4.0),
    blur_radius: Length::Px(8.0),
    spread_radius: Length::Px(3.0),
    color: palettes::basic::BLACK.into()
	})
	.with(ELEVATION_4, Elevation {
		offset_x: Length::Px(0.0),
    offset_y: Length::Px(6.0),
    blur_radius: Length::Px(10.0),
    spread_radius: Length::Px(4.0),
    color: palettes::basic::BLACK.into()
	})
	.with(ELEVATION_5, Elevation {
		offset_x: Length::Px(0.0),
    offset_y: Length::Px(8.0),
    blur_radius: Length::Px(12.0),
    spread_radius: Length::Px(6.0),
    color: palettes::basic::BLACK.into()
	})
}
