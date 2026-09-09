//! Remove a bundle when called.
use crate::prelude::*;
use beet_core::prelude::*;

/// Removes a bundle from a [`TargetEntity`] when called, then passes.
///
/// See also [`InsertOn`] for the inverse operation.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn((Name::new("bill"), RemoveOn::<Name>::default()));
/// ```
#[action]
#[derive(Component)]
pub async fn RemoveOn<B>(
	/// Which entity to remove the bundle from.
	#[field]
	target_entity: TargetEntity,
	cx: ActionContext,
) -> Result<Outcome>
where
	B: 'static + Send + Sync + Bundle,
{
	let action = cx.caller.id();
	let world = cx.world();
	let target = target_entity.get_async(&world, action).await;
	world
		.entity(target)
		.with(|mut entity| {
			entity.remove::<B>();
		})
		.await?;
	Outcome::PASS.xok()
}

impl<B: 'static + Send + Sync + Bundle> RemoveOn<B> {
	/// Remove `B` from the given [`TargetEntity`].
	pub fn new_with_target(target_entity: TargetEntity) -> Self {
		Self {
			target_entity,
			_marker: PhantomData,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn removes_from_action() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn((Name::new("bill"), RemoveOn::<Name>::default()))
			.id();
		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		world.entity(entity).get::<Name>().xpect_none();
	}
}
