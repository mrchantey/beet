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
	// the markup scene templates (`<Lighting3d/>`, `<Ground3d/>`, `<Sprite2d/>`, ...),
	// so a `.bsx` names them and `beet_example_plugin` registers them by short type path.
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::AgentOf;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::AppWindow;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::Camera3dLookAt;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::Flock;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::Foxie;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::Ground3d;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::IkArm;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::Lighting3d;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::Scene2d;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::Scene3d;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::SeekAgent2d;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::SpaceScene;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::Sprite2d;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::SteerTo;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::UiTerminal;
	#[cfg(feature = "bevy_default")]
	pub use crate::scenes::WorldScene;
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
