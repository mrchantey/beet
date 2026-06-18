#![doc = include_str!("../README.md")]

beet_core::test_main!();

#[cfg(feature = "bevy_default")]
pub mod components;
#[cfg(feature = "bevy_default")]
pub mod plugins;
#[cfg(feature = "bevy_default")]
pub mod scenes;

#[cfg(feature = "bevy_default")]
pub mod prelude {
	pub use crate::components::*;
	pub use crate::plugins::*;
}

// because of cyclic deps we cant use beet directly
// so instead we make a pretend beet module
#[cfg(feature = "bevy_default")]
pub(crate) mod beet {
	// pub use beet_action as action;
	// pub use beet_ml as ml;
	// pub use beet_spatial as spatial;

	pub mod prelude {
		pub use beet_action::prelude::*;
		#[cfg(feature = "ml")]
		pub use beet_ml::prelude::*;
		pub use beet_spatial::prelude::*;
	}
}
