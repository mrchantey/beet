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
#[derive(Component)]
#[require(RemoveOnAction<B>)]
pub struct RemoveOn<B: 'static + Send + Sync + Bundle> {
	/// Which entity to remove the bundle from.
	pub target_entity: TargetEntity,
	phantom: PhantomData<B>,
}

impl<B: 'static + Send + Sync + Bundle> Clone for RemoveOn<B> {
	fn clone(&self) -> Self {
		Self {
			target_entity: self.target_entity.clone(),
			phantom: PhantomData,
		}
	}
}

impl<B: 'static + Send + Sync + Bundle> Default for RemoveOn<B> {
	fn default() -> Self {
		Self {
			target_entity: TargetEntity::Action,
			phantom: PhantomData,
		}
	}
}

impl<B: 'static + Send + Sync + Bundle> RemoveOn<B> {
	/// Remove `B` from the given [`TargetEntity`].
	pub fn new_with_target(target_entity: TargetEntity) -> Self {
		Self {
			target_entity,
			phantom: PhantomData,
		}
	}
}

/// Resolves the target, removes the bundle, then passes.
///
/// ## Errors
/// Errors if the caller has no [`RemoveOn`] component.
#[action(default)]
#[derive(Component)]
pub async fn RemoveOnAction<B>(cx: ActionContext) -> Result<Outcome>
where
	B: 'static + Send + Sync + Bundle,
{
	let action = cx.caller.id();
	let world = cx.world();
	let target = cx
		.caller
		.get_cloned::<RemoveOn<B>>()
		.await?
		.target_entity
		.get_async(&world, action)
		.await;
	world
		.entity(target)
		.with_then(|mut entity| {
			entity.remove::<B>();
		})
		.await;
	Outcome::PASS.xok()
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
