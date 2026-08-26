use crate::prelude::*;

/// Cargo features a subtree's behavior needs, enforced at *dispatch* time: a
/// route at or under this declaration fails a call with the missing feature
/// list instead of running whichever of its steps happened to link.
///
/// The dispatch-time complement of [`CrateCheck`]: a check fails the whole
/// load because the entry cannot function at all, while this leaves the
/// document loading whole (structure is universal) and puts the loudness on
/// the one boundary that must not degrade silently. The vocabulary is the same
/// `feature` / `crate/feature` items, verified against the spawned
/// [`CrateRegistration`] set:
///
/// ```html
/// <Route path="deploy" {(ExchangeSequence, RequireFeatures(["infra","extra"]))}>
/// ```
///
/// Also read by the bsx resolver: an unregistered tag under an *unmet*
/// requirement builds inert at `debug!` rather than `warn!`, since its
/// inertness is declared. The enforcing middleware is registered by
/// beet_router's `RouterPlugin`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deref, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RequireFeatures(pub Vec<SmolStr>);

impl RequireFeatures {
	/// Require every item, each `feature` or `crate/feature`.
	pub fn new(features: impl IntoIterator<Item = impl Into<SmolStr>>) -> Self {
		Self(features.into_iter().map(Into::into).collect())
	}

	/// Every missing requirement as a sorted list, empty when all are compiled
	/// in. Delegates to [`CrateCheck::feature_failures`], the one requirement
	/// grammar.
	pub fn failures<'a>(
		&self,
		registrations: impl IntoIterator<Item = &'a CrateRegistration> + Clone,
	) -> Vec<String> {
		CrateCheck::feature_failures(&self.0, registrations)
	}
}
