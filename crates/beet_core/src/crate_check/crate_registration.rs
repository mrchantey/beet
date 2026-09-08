use crate::prelude::*;

/// The compiled-in surface of a crate: its version and enabled cargo features.
///
/// Spawned once per participating crate, usually via [`crate_registration!`]
/// in the crate's primary plugin, so a `feature:`/`version:` cfg atom can ask
/// what this binary was actually built with.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CrateRegistration {
	/// The crate name as compiled, ie `CARGO_PKG_NAME`.
	crate_name: SmolStr,
	/// The primary registration: an unprefixed requirement (`feature:infra`
	/// rather than `feature:beet_esp/alvik`) resolves here. Only the binary
	/// crate (`beet-cli`) sets this.
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

	/// Whether a `feature:` requirement item holds across `registrations`, ie
	/// `infra` (the primary crate) or `beet_esp/alvik` (a named one).
	///
	/// The ONE feature-requirement grammar. A `bx:cfg` atom and a
	/// `<RequireCfg>` assertion both come through here, so there is no second
	/// spelling to keep in step.
	pub fn has_feature_item(registrations: &[Self], item: &str) -> bool {
		let (crate_name, feature) = match item.split_once('/') {
			Some((crate_name, feature)) => (Some(crate_name), feature),
			None => (None, item),
		};
		Self::resolve(registrations, crate_name)
			.is_some_and(|registration| registration.has_feature(feature))
	}

	/// Whether a `version:` requirement item holds across `registrations`, ie
	/// `0.0.9` (the primary crate) or `beet_esp@0.5.9` (a named one). The
	/// comparison is a minimum, so a newer compiled version satisfies it.
	pub fn meets_version_item(registrations: &[Self], item: &str) -> bool {
		let (crate_name, version) = match item.split_once('@') {
			Some((crate_name, version)) => (Some(crate_name), version),
			None => (None, item),
		};
		Self::resolve(registrations, crate_name).is_some_and(|registration| {
			parse_version(registration.version()) >= parse_version(version)
		})
	}

	/// The registration for `crate_name`, or the primary one when unprefixed.
	fn resolve<'a>(
		registrations: &'a [Self],
		crate_name: Option<&str>,
	) -> Option<&'a Self> {
		registrations.iter().find(|registration| match crate_name {
			Some(name) => registration.crate_name() == name,
			None => registration.skip_prefix(),
		})
	}
}

/// A lenient semver triple for ordering, ignoring pre-release/build suffixes.
fn parse_version(version: &str) -> (u32, u32, u32) {
	let mut parts = version
		.split('.')
		.map(|part| {
			part.split(|char: char| !char.is_ascii_digit())
				.next()
				.and_then(|digits| digits.parse().ok())
				.unwrap_or(0)
		})
		.chain(core::iter::repeat(0));
	(
		parts.next().unwrap_or(0),
		parts.next().unwrap_or(0),
		parts.next().unwrap_or(0),
	)
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
