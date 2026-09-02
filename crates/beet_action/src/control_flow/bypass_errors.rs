//! Opt-outs from the failures control flow raises, one component per mechanism.
use crate::prelude::*;
use beet_core::prelude::*;
use bitflags::bitflags;

/// Which child errors to bypass, defaults to none.
#[derive(Debug, Default, Clone, Copy, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct BypassErrors(pub ChildError);

impl BypassErrors {
	/// Whether `child` serves `Action<Input, Out>`, under this policy.
	///
	/// `Ok(false)` is a child the policy skipped, `Err` one it did not. The
	/// three call sites used to spell this out with three different error
	/// constructions around the same two checks, which is how they drifted;
	/// `node` names the caller, ie `"sequence"`.
	///
	/// # Errors
	/// Errors naming the child, and what it does serve, when the policy does
	/// not bypass its failure.
	pub fn serves<Input, Out>(
		&self,
		node: &str,
		child: Entity,
		meta: Option<&ActionMeta>,
	) -> Result<bool>
	where
		Input: 'static,
		Out: 'static,
	{
		let Some(meta) = meta else {
			if self.contains(ChildError::NO_ACTION) {
				return Ok(false);
			}
			bevybail!("{node} child has no action: {child}");
		};
		if !meta.matches::<Input, Out>() {
			if self.contains(ChildError::ACTION_MISMATCH) {
				return Ok(false);
			}
			bevybail!(
				"{node} child has the wrong action signature: {child}, it serves: {}",
				meta.signatures()
			);
		}
		Ok(true)
	}
}

bitflags! {
	/// Why a child of a control-flow node could not serve its call.
	/// Used with [`BypassErrors`] to selectively skip certain child issues.
	///
	/// Every variant is about a child that *was* a step and could not serve.
	/// Whether something is a step at all is not this policy's question:
	/// [`BehaviourChildren`] answers that before the policy is consulted, so
	/// markup punctuation never appears here and there is no flag to forget.
	#[repr(transparent)]
	#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
	#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
	#[reflect(opaque)]
	#[reflect(Hash, Clone, PartialEq, Debug, Default)]
	#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
	pub struct ChildError: u8 {
		/// Child entity has no [`ActionMeta`](crate::prelude::ActionMeta) component.
		const NO_ACTION = 0b01;
		/// Child entity has an action with an incompatible signature.
		const ACTION_MISMATCH = 0b10;
		/// Every child was skipped, so a parent that has children would run none
		/// of them. Bypassed only by a parent for which doing nothing is a valid
		/// outcome.
		const NONE_VALID = 0b100;
	}
}

/// Which [`RunningSet`](crate::prelude::RunningSet) errors to bypass, defaults to none.
///
/// An entity whose facets may all decline yet which should still park (a boot
/// whose selection named none of them) declares the opt-out here, rather than
/// the set second-guessing what a caller meant by an empty start.
///
/// Deliberately not merged with [`BypassErrors`], which reads like it: that one
/// classifies a child *before* anything runs, structurally and all at once, and
/// this one classifies a facet's outcome *while* it runs, one at a time. One
/// flag set would leave each caller carrying flags that cannot arise for it.
#[derive(Debug, Default, Clone, Copy, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct BypassRunningErrors(pub RunningError);

bitflags! {
	/// Failures a [`RunningSet`](crate::prelude::RunningSet) resolves its parked call with.
	/// Used with [`BypassRunningErrors`] to park instead.
	#[repr(transparent)]
	#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
	#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
	#[reflect(opaque)]
	#[reflect(Hash, Clone, PartialEq, Debug, Default)]
	#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
	pub struct RunningError: u8 {
		/// Every declared facet declined the start, so nothing holds the run open.
		const NONE_STARTED = 0b01;
		/// A driven facet errored. Bypassed, the error is logged loudly, that facet
		/// is dropped and the survivors keep being driven; the call still fails once
		/// no facet is left alive, so a fully dead run is never silent.
		const FACET_FAILED = 0b10;
	}
}
