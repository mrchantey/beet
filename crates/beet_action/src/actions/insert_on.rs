//! Insert a bundle when called.
use crate::prelude::*;
use beet_core::prelude::*;

/// Inserts a cloned bundle on a [`TargetEntity`] when called, then passes.
///
/// See also [`RemoveOn`] for the inverse operation.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn(InsertOn::new(Name::new("bill")));
/// ```
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn InsertOn<B>(
	/// The bundle to be cloned and inserted.
	#[field]
	bundle: B,
	/// Which entity to insert the bundle on.
	#[field]
	target_entity: TargetEntity,
	cx: ActionContext,
) -> Result<Outcome>
where
	B: 'static
		+ Send
		+ Sync
		+ Bundle
		+ Clone
		+ Default
		+ Reflect
		+ FromReflect
		+ TypePath,
{
	let action = cx.caller.id();
	let world = cx.world();
	let target = target_entity.get_async(&world, action).await;
	world.entity(target).insert(bundle).await?;
	Outcome::PASS.xok()
}

impl<B> InsertOn<B>
where
	B: 'static
		+ Send
		+ Sync
		+ Bundle
		+ Clone
		+ Default
		+ Reflect
		+ FromReflect
		+ TypePath,
{
	/// Insert `bundle` on the action entity itself.
	pub fn new(bundle: B) -> Self {
		Self {
			bundle,
			target_entity: TargetEntity::Action,
			_marker: PhantomData,
		}
	}
	/// Insert `bundle` on the given [`TargetEntity`].
	pub fn new_with_target(bundle: B, target_entity: TargetEntity) -> Self {
		Self {
			bundle,
			target_entity,
			_marker: PhantomData,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn inserts_on_action() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(InsertOn::new(Name::new("bill"))).id();
		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		world
			.entity(entity)
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("bill");
	}

	#[beet_core::test]
	async fn inserts_on_agent() {
		let mut world = AsyncPlugin::world();
		let agent = world.spawn_empty().id();
		let action = world
			.spawn((
				ActionOf(agent),
				InsertOn::new_with_target(
					Name::new("on-agent"),
					TargetEntity::Agent,
				),
			))
			.id();
		world
			.entity_mut(action)
			.call::<(), Outcome>(())
			.await
			.unwrap();
		world
			.entity(agent)
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("on-agent");
	}
}
