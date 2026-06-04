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
#[derive(Debug, Clone, Component)]
#[require(InsertOnAction<B>)]
pub struct InsertOn<B: 'static + Send + Sync + Bundle + Clone> {
	/// The bundle to be cloned and inserted.
	pub bundle: B,
	/// Which entity to insert the bundle on.
	pub target_entity: TargetEntity,
}

impl<B: 'static + Send + Sync + Bundle + Clone> InsertOn<B> {
	/// Insert `bundle` on the action entity itself.
	pub fn new(bundle: B) -> Self {
		Self {
			bundle,
			target_entity: TargetEntity::Action,
		}
	}
	/// Insert `bundle` on the given [`TargetEntity`].
	pub fn new_with_target(bundle: B, target_entity: TargetEntity) -> Self {
		Self {
			bundle,
			target_entity,
		}
	}
}

/// Resolves the target, inserts the cloned bundle, then passes.
///
/// ## Errors
/// Errors if the caller has no [`InsertOn`] component.
#[action(default)]
#[derive(Component)]
pub async fn InsertOnAction<B>(cx: ActionContext) -> Result<Outcome>
where
	B: 'static + Send + Sync + Bundle + Clone,
{
	let action = cx.caller.id();
	let world = cx.world();
	let insert_on = cx.caller.get_cloned::<InsertOn<B>>().await?;
	let target = insert_on.target_entity.get_async(&world, action).await;
	world.entity(target).insert(insert_on.bundle).await?;
	Outcome::PASS.xok()
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
