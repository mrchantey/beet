use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;


token2!(Elevation0, Elevation);


pub fn elevations() -> Selector {
	Selector::new()
		.with_value::<Elevation0>(Elevation {
			offset_x: Length::Px(0.0),
			offset_y: Length::Px(1.0),
			blur_radius: Length::Px(3.0),
			spread_radius: Length::Px(1.0),
			color: palettes::basic::BLACK.into(),
		})
		.unwrap()
}
