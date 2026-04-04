use crate::prelude::*;
use bevy::tasks::ConditionalSendFuture;

/// Component for storing async initialization functions for an entity,
/// to be executed manually at the developers discression, usually immediately
/// after load.
#[derive(Default, Component)]
pub struct AsyncInit {
	items: Vec<
		Box<
			dyn Send
				+ Sync
				+ FnOnce(AsyncEntity) -> BoxedFuture<'static, Result>,
		>,
	>,
}


impl AsyncInit {
	/// Register an async init function when the entity is initialized.
	pub fn register_on_add<Func, Fut>(
		mut world: DeferredWorld,
		cx: HookContext,
		func: Func,
	) where
		Func: 'static + Send + Sync + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + ConditionalSendFuture<Output = Result>,
	{
		world.commands().entity(cx.entity).queue(
			|mut entity: EntityWorldMut| {
				if let Some(mut init) = entity.get_mut::<Self>() {
					init.add(func);
				} else {
					let mut init = Self::new();
					init.add(func);
					entity.insert(init);
				}
			},
		);
	}

	/// Create a new empty AsyncInit component.
	pub fn new() -> Self { Self { items: Vec::new() } }
	/// Add an async init function to this entity, which will be run when the entity is initialized.
	pub fn add<Func, Fut>(&mut self, func: Func)
	where
		Func: 'static + Send + Sync + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + ConditionalSendFuture<Output = Result>,
	{
		self.items
			.push(Box::new(move |entity| Box::pin(func(entity))));
	}

	/// Run the async init functions for this entity.
	pub async fn run(self, entity: AsyncEntity) -> Result {
		futures::future::try_join_all(
			self.items.into_iter().map(|item| item(entity.clone())),
		)
		.await?;
		Ok(())
	}

	/// Run the async init functions for this entity and its children,
	/// allowing each to add more async init functions to the entity.
	pub async fn run_recursive(entity: AsyncEntity) -> Result {
		let inits = entity
			.with_then(|mut entity| entity.take_recursive::<AsyncInit>())
			.await;

		futures::future::try_join_all(
			inits.into_iter().map(|init| init.run(entity.clone())),
		)
		.await?;


		Ok(())
	}
}
