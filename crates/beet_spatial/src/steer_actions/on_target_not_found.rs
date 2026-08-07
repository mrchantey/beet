use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Instructions for how to behave when an action's [`SteerTarget`] cannot be
/// resolved, ie a [`SteerTarget::Entity`] that has since been despawned.
///
/// Required by every steering action that reads a target ([`Seek`], [`Flee`],
/// [`Pursue`], [`Evade`]), so the policy is authored on the action entity:
/// `<Seek {OnTargetNotFound::Clear}/>`.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub enum OnTargetNotFound {
	/// Warn
	#[default]
	Warn,
	/// Remove the [`SteerTarget`]
	Clear,
	/// Do nothing
	Ignore,
	/// End the run with [`Outcome::FAIL`]
	Fail,
	/// End the run with [`Outcome::PASS`]
	Succeed,
}

impl OnTargetNotFound {
	/// Apply this policy for an `action` whose `agent` has an unresolvable
	/// [`SteerTarget`], `err` describing why it did not resolve.
	pub(crate) fn apply(
		&self,
		commands: &mut Commands,
		action: Entity,
		agent: Entity,
		err: BevyError,
	) {
		match self {
			Self::Warn => log::warn!("{err}"),
			Self::Clear => {
				commands.entity(agent).remove::<SteerTarget>();
			}
			Self::Ignore => {}
			Self::Fail => {
				commands.entity(action).queue(EndRun(Outcome::FAIL));
			}
			Self::Succeed => {
				commands.entity(action).queue(EndRun(Outcome::PASS));
			}
		}
	}
}
