//! Documents as scenes: instantiating one document in another.
//!
//! An [`Instance`](beet_core::prelude::Instance) names another document of its
//! package. Activating it loads the target through the parser its format
//! selects, holds it as data ([`InstancePrefab`](prelude::InstancePrefab)) and builds that under the
//! use site, each use with its own entities. See [`Activation`](prelude::Activation) for
//! the whole of what an activation does.
//!
//! A reference resolves against the [`PackageStore`](beet_net::prelude::PackageStore) nearest above it: the
//! store rooted at the directory the package's entry is in.

beet_core::test_main!();

mod instance;
mod prefab_cache;
mod scene_plugin;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::instance::*;
	pub use crate::prefab_cache::*;
	pub use crate::scene_plugin::*;
}
