#![doc = include_str!("../README.md")]

beet_core::test_main!();

#[cfg(feature = "bevy_default")]
pub mod components;
// the cloudflare/aws deploy example wiring, so an `examples/infra/hello_*.bsx`
// runs through the one `beet` binary (headless, no render).
#[cfg(feature = "infra")]
pub mod infra;
// the perceive-act types the agent and every head/body client share (the wire
// types + the socket-client primitives), split out so the wasm head reuses them
// without the `thread` runtime. Present whenever the agent or the web head is.
#[cfg(any(feature = "thread", feature = "perceive_act_web"))]
pub mod perceive_act_core;
// the embodied perceive-act agent tools, so an `examples/perceive_act/*.bsx`
// scene runs through the one binary (headless, no render needed). Needs the
// thread runtime, blob store and child-process exec the `thread` feature pulls.
#[cfg(feature = "thread")]
pub mod perceive_act;
// the wasm browser head for the perceive-act demo (v3): a socket client served to a
// browser tab, capturing the webcam, speaking via the Web Speech API and rendering an
// `<img>` face. Wasm-safe, so gated on its own `perceive_act_web` feature (the `web`
// base, no `thread`), not on the native `thread` set.
#[cfg(feature = "perceive_act_web")]
pub mod perceive_act_web;
// always compiled: `BeetExtraPlugin` selects its members by feature flag, so
// the thread-only (`thread`, no `bevy_default`) build still gets the plugin.
pub mod plugins;
#[cfg(feature = "bevy_default")]
pub mod scenes;

pub mod prelude {
	#[cfg(feature = "bevy_default")]
	pub use crate::components::*;
	#[cfg(feature = "infra")]
	pub use crate::infra::*;
	#[cfg(any(feature = "thread", feature = "perceive_act_web"))]
	pub use crate::perceive_act_core::*;
	#[cfg(feature = "thread")]
	pub use crate::perceive_act::*;
	#[cfg(feature = "perceive_act_web")]
	pub use crate::perceive_act_web::*;
	pub use crate::plugins::*;
	// the markup scene templates (`<Lighting3d/>`, `<Ground3d/>`, `<Sprite2d/>`, ...),
	// so a `.bsx` names them and `beet_extra_bevy_default_plugin` registers them by
	// short type path.
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
	pub use crate::scenes::IkTarget;
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
		// only the render example set (`beet_extra_bevy_default_plugin`) uses the
		// spatial prelude.
		#[cfg(feature = "bevy_default")]
		pub use beet_spatial::prelude::*;
		// the render scenes' `RunOnLoad` load verb lives in `beet_net` with the rest
		// of the family; pull just it in (not the whole net prelude) so the render set
		// resolves it without depending on the `thread` example wiring.
		#[cfg(feature = "bevy_default")]
		pub use beet_net::prelude::RunOnLoad;
		#[cfg(feature = "thread")]
		pub use beet_thread::prelude::*;
	}
}
