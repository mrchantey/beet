//! Immediate return action for a constant value.
use crate::prelude::*;
use beet_core::prelude::*;

/// Immediately returns a constant value when called.
///
/// Conceptually similar to a `const`, though the value may be modified by
/// external systems before the call. Defaults to returning [`Outcome::PASS`].
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn(EndWith(Outcome::PASS));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[require(EndWithAction<T>)]
#[reflect(Component)]
pub struct EndWith<T = Outcome>(pub T)
where
	T: 'static + Send + Sync + Clone;

/// Returns the value stored in the [`EndWith`] component.
///
/// ## Errors
/// Errors if the caller has no [`EndWith`] component.
#[action(default)]
#[derive(Component)]
pub async fn EndWithAction<T>(cx: ActionContext) -> Result<T>
where
	T: 'static + Send + Sync + Clone,
{
	cx.caller.get_cloned::<EndWith<T>>().await.map(|end| end.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn returns_value() {
		AsyncPlugin::world()
			.spawn(EndWith(Outcome::PASS))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn works_as_sequence_child() {
		AsyncPlugin::world()
			.spawn((Sequence::new(), children![
				EndWith(Outcome::PASS),
				EndWith(Outcome::PASS),
			]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
