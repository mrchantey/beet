use crate::prelude::*;


#[extend::ext(name=CommandsExt)]
pub impl Commands<'_, '_> {
	fn log_component_names(&mut self, entity: Entity) {
		self.queue(move |world: &mut World| {
			world.log_component_names(entity);
		});
	}

	#[cfg(feature = "bevy_scene")]
	fn load_scene(&mut self, scene: impl Into<String>) {
		let scene = scene.into();
		self.queue(move |world: &mut World| -> Result {
			world.load_scene(&scene)
		});
	}

	/// Queues an asynchronous task to be run in the world context.
	fn queue_async<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		self.queue(move |world: &mut World| {
			world.run_async(move |world| func(world));
		});
	}

	fn run_system_once_with<I, M, S>(
		&mut self,
		system: S,
		input: I::Inner<'static>,
	) where
		I: SystemInput<Inner<'static>: Send> + Send + 'static,
		M: 'static,
		S: IntoSystem<I, (), M> + Send + 'static,
	{
		self.queue(move |world: &mut World| {
			world.run_system_once_with(system, input).ok();
		});
	}
}
