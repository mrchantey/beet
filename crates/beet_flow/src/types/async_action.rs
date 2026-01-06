use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::world::CommandQueue;


/// A convenience wrapper around an [`AsyncWorld`] and [`ActionContext`],
/// upon drop all commands in the [`ActionContext`] are applied to the [`AsyncWorld`]
pub struct AsyncAction {
	/// AsyncEntity used to schedule work back onto the main world
	entity: AsyncEntity,
	/// The ActionContext local to this async task
	cx: ActionContext,
}

impl AsyncAction {
	pub fn new(entity: AsyncEntity, cx: ActionContext) -> Self {
		Self { entity, cx }
	}
	/// Access the AsyncEntity representing this action
	pub fn entity(&self) -> &AsyncEntity { &self.entity }
	/// Access the AsyncWorld
	pub fn world(&self) -> &AsyncWorld { self.entity.world() }
	/// Access the ActionContext
	pub fn context(&self) -> &ActionContext { &self.cx }
	/// Access the ActionContext mutably
	pub fn context_mut(&mut self) -> &mut ActionContext { &mut self.cx }
}

impl std::ops::Deref for AsyncAction {
	type Target = ActionContext;
	fn deref(&self) -> &Self::Target { &self.cx }
}

impl std::ops::DerefMut for AsyncAction {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.cx }
}

impl Drop for AsyncAction {
	fn drop(&mut self) {
		// Move the queued commands out, then apply them to the world
		let mut queue = CommandQueue::default();
		std::mem::swap(&mut self.cx.queue, &mut queue);
		self.world().with(move |world: &mut World| {
			queue.apply(world);
		});
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[action(foo)]
	#[derive(Component)]
	struct Foo;

	fn foo(mut ev: On<GetOutcome>) {
		ev.run_async(async move |mut action| {
			time_ext::sleep(Duration::from_millis(20)).await;
			action.world().spawn_then(Name::new("foo")).await;
			action.trigger_target(Outcome::Pass);
		});
	}

	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins((ControlFlowPlugin, AsyncPlugin));
		app.world_mut()
			.spawn((Sequence, ExitOnEnd, children![
				Foo,
				EndWith(Outcome::Fail)
			]))
			.trigger_target(GetOutcome);

		app.run_async().await.xpect_eq(AppExit::error());
	}
}
