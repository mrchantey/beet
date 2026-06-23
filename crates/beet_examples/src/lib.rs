#![doc = include_str!("../README.md")]

beet_core::test_main!();

#[cfg(feature = "bevy_default")]
pub mod components;
// always compiled: `BeetExamplePlugins` selects its members by feature flag, so
// the thread-only (`thread`, no `bevy_default`) build still gets the group.
pub mod plugins;
#[cfg(feature = "bevy_default")]
pub mod scenes;

pub mod prelude {
	#[cfg(feature = "bevy_default")]
	pub use crate::components::*;
	pub use crate::plugins::*;
}

// because of cyclic deps we cant use beet directly
// so instead we make a pretend beet module. its re-exports vary by feature (the
// render set vs the thread set use different subsets), so allow unused imports.
pub(crate) mod beet {
	#[allow(unused_imports)]
	pub mod prelude {
		pub use beet_action::prelude::*;
		#[cfg(feature = "ml")]
		pub use beet_ml::prelude::*;
		// only the render example set (`beet_example_plugin`) uses the spatial prelude.
		#[cfg(feature = "bevy_default")]
		pub use beet_spatial::prelude::*;
		#[cfg(feature = "thread")]
		pub use beet_thread::prelude::*;
	}
}
