//! The one app body a beet binary runs: the trusted defaults, whatever the
//! binary links on top, and the entry loader.
//!
//! beet is unopinionated like a game engine: a binary links a library of
//! capabilities (registered reflect types) and ships zero behaviour, and the
//! entry document decides what runs. A workspace that names only beet types runs
//! through the stock `beet` binary; a workspace that EXTENDS beet with reflect
//! types of its own builds a binary of its own and reaches for [`app`], which is
//! the same resolution, load and lifecycle with those types linked in.
//!
//! ```rust,ignore
//! use beet::prelude::*;
//!
//! fn main() -> AppExit {
//! 	env_ext::load_dotenv().ok();
//! 	let mut app = launch::app(MyCratePlugin);
//! 	app.world_mut().spawn(
//! 		crate_registration!({ features: ["my-feature"] }).with_skip_prefix(),
//! 	);
//! 	app.run()
//! }
//! ```
//!
//! The two halves are separable on purpose. [`LaunchPlugin`] and the
//! [`entry_build`] core live in `beet_router` and know nothing about which
//! capabilities exist; `BeetPlugins` is the facade's own trusted default set.
//! This module is only their composition.
use crate::exports::bevy::app::Plugins;
use crate::prelude::*;

/// An [`App`] with the trusted defaults ([`BeetPlugins`]: the runner, beet's
/// logging, the async runtime, and the router/scene/server capabilities selected
/// by feature flag), `plugins` on top, and the [`LaunchPlugin`] entry loader.
///
/// The process exits when the loaded tree writes `AppExit` for the one-shot it
/// resolves; a long-running server parks its boot call, so its unresolved
/// `Running<Response>` persists the process with no refcount.
///
/// `plugins` is where a binary links what beet cannot know: its own registered
/// types, so an entry naming them resolves rather than degrading into an
/// `UnregisteredTag`. Pass `()` for none.
///
/// The caller still spawns its own [`CrateRegistration`] (see [`LaunchPlugin`]),
/// since only the binary knows which cargo features it was compiled with.
pub fn app<M>(plugins: impl Plugins<M>) -> App {
	let mut app = App::new();
	app.add_plugins(BeetPlugins);
	app.add_plugins(plugins);
	app.add_plugins(LaunchPlugin);
	app
}
