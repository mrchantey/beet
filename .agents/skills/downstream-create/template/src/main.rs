//! This repo's beet binary: the stock runner plus [`@Name@Plugin`].
//!
//! The stock `beet` binary serves any entry whose tags it can resolve, which is
//! every type beet itself registers. A tag defined in THIS crate is one no beet
//! build can know, so the crate that defines it builds the binary that runs it.
//!
//! Run it through the justfile, which threads the feature flag:
//!
//! ```sh
//! just cli --help
//! ```
use beet::prelude::*;
use beet_@name@::prelude::*;

fn main() -> AppExit {
	let mut app = App::new();
	app.add_plugins((BeetPlugins, @Name@Plugin, LaunchPlugin));
	// this binary's compiled surface, spawned before the entry loads so its
	// `<RequireCfg/>` verifies against it
	app.world_mut()
		.spawn(crate_registration!({ features: ["cli"] }).with_skip_prefix());
	app.run()
}
