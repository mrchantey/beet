use crate::prelude::*;

/// The compiled-in surface of a crate: its version and enabled cargo features.
///
/// Spawned once per participating crate, usually via [`crate_registration!`]
/// in the crate's primary plugin, so a [`CrateCheck`](super::CrateCheck) can
/// verify the running binary was built with the features an entry requires.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CrateRegistration {
	/// The crate name as compiled, ie `CARGO_PKG_NAME`.
	crate_name: SmolStr,
	/// The primary registration: unprefixed [`CrateCheck`](super::CrateCheck)
	/// requirements resolve here. Only the binary crate (`beet-cli`) sets this.
	skip_prefix: bool,
	/// The compiled crate version, ie `CARGO_PKG_VERSION`.
	version: SmolStr,
	/// The cargo features the crate was compiled with.
	features: HashSet<SmolStr>,
}

impl CrateRegistration {
	/// A registration with no features, usually via [`crate_registration!`].
	pub fn new(
		crate_name: impl Into<SmolStr>,
		version: impl Into<SmolStr>,
	) -> Self {
		Self {
			crate_name: crate_name.into(),
			skip_prefix: false,
			version: version.into(),
			features: HashSet::default(),
		}
	}

	/// Record `feature` as compiled in, usually via [`crate_registration!`].
	pub fn with_feature(mut self, feature: impl Into<SmolStr>) -> Self {
		self.features.insert(feature.into());
		self
	}

	/// Mark this as the primary registration, resolving unprefixed requirements.
	/// Only the binary crate (`beet-cli`) may set this.
	#[doc(hidden)]
	pub fn with_skip_prefix(mut self) -> Self {
		self.skip_prefix = true;
		self
	}

	/// The crate name as compiled.
	pub fn crate_name(&self) -> &str { &self.crate_name }

	/// Whether this is the primary registration.
	pub fn skip_prefix(&self) -> bool { self.skip_prefix }

	/// The compiled crate version.
	pub fn version(&self) -> &str { &self.version }

	/// Whether `feature` was compiled in.
	pub fn has_feature(&self, feature: &str) -> bool {
		self.features.contains(feature)
	}
}

/// Builds a [`CrateRegistration`] for the calling crate from compile-time
/// cargo env vars, recording which of the listed features are enabled.
///
/// The feature list names every feature the crate *could* have (cargo offers
/// no way to enumerate them); each is recorded only if enabled in this build.
///
/// ```
/// # use beet_core::prelude::*;
/// let mut world = World::new();
/// world.spawn(crate_registration!({ features: ["std", "some-feature"] }));
/// ```
#[macro_export]
macro_rules! crate_registration {
	() => {
		$crate::prelude::CrateRegistration::new(
			env!("CARGO_PKG_NAME"),
			env!("CARGO_PKG_VERSION"),
		)
	};
	({ features: [$($feature:literal),* $(,)?] $(,)? }) => {{
		#[allow(unused_mut)]
		let mut registration = $crate::crate_registration!();
		$(
			if cfg!(feature = $feature) {
				registration = registration.with_feature($feature);
			}
		)*
		registration
	}};
}

// a knowingly-unknown feature below: the lint it trips is the desirable typo
// guard for real callers, silenced only here
#[cfg(test)]
#[allow(unexpected_cfgs)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn records_enabled_features_only() {
		// `std` is enabled for beet_core tests, `not-a-feature` never is
		let registration =
			crate_registration!({ features: ["std", "not-a-feature"] });
		registration.crate_name().xpect_eq("beet_core");
		registration.version().xpect_eq(env!("CARGO_PKG_VERSION"));
		registration.has_feature("std").xpect_true();
		registration.has_feature("not-a-feature").xpect_false();
	}
}
