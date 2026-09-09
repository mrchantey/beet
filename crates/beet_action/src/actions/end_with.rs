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
/// world.spawn(EndWith::new(Outcome::PASS));
/// ```
#[action(plain_meta)]
#[derive(Debug, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn EndWith<T = Outcome>(
	/// The value returned on every call.
	#[field]
	value: T,
) -> Result<T>
where
	T: 'static + Send + Sync + Clone + Default,
{
	value.xok()
}

impl<T> EndWith<T>
where
	T: 'static + Send + Sync + Clone + Default,
{
	/// Always return `value`.
	pub fn new(value: T) -> Self {
		Self {
			value,
			_marker: PhantomData,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn returns_value() {
		AsyncPlugin::world()
			.spawn(EndWith::new(Outcome::PASS))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn works_as_sequence_child() {
		AsyncPlugin::world()
			.spawn((Sequence::new(), children![
				EndWith::new(Outcome::PASS),
				EndWith::new(Outcome::PASS),
			]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
