use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::IsFunctionSystem;
use bevy::ecs::system::SystemParamFunction;
use bevy::ecs::system::SystemState;

/// Create a [`ToolHandler`] from a Bevy system that returns [`Result<Out>`].
///
/// Unlike [`func_tool`](crate::func_tool), system tools have access to
/// ECS queries, resources, and other system parameters.
///
/// The system's first argument must be `In<Input>` (the tool's input
/// payload), followed by any number of regular system parameters.
///
/// ## Examples
///
/// ```rust
/// # use beet_tool::prelude::*;
/// # use beet_core::prelude::*;
/// let handler = system_tool(|In(()): In<()>, time: Res<Time>| -> Result<f32> {
///     Ok(time.elapsed_secs())
/// });
/// ```
pub fn system_tool<Func, Input, Out, FnMarker>(
	func: Func,
) -> ToolHandler<Input, Out>
where
	Func: 'static + Send + Sync + Clone,
	FnMarker: 'static,
	Func: SystemParamFunction<FnMarker, Out = Result<Out>>,
	Func: IntoSystem<In<Input>, Result<Out>, (IsFunctionSystem, FnMarker)>,
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	ToolHandler::new(
		TypeMeta::of::<Func>(),
		move |ToolCall {
		          mut commands,
		          tool: _,
		          input,
		          out_handler,
		      }| {
			let func = func.clone();
			commands.commands.queue(move |world: &mut World| -> Result {
				let output: Result<Out> =
					world.run_system_cached_with(func, input)?;
				let output = output?;

				// Obtain fresh AsyncCommands via SystemState so
				// the out_handler (and any downstream pipe
				// handlers) can queue further work.
				let result = {
					let mut state = SystemState::<AsyncCommands>::new(world);
					let async_commands = state.get_mut(world);
					let result = out_handler.call(async_commands, output);
					state.apply(world);
					result
				};
				world.flush();
				result
			});
			Ok(())
		},
	)
}

/// Marker for the system tool [`IntoToolHandler`] impl.
pub struct SystemToolMarker;

impl<Func, Input, Out, FnMarker>
	IntoToolHandler<(SystemToolMarker, Input, Out, FnMarker)> for Func
where
	Func: 'static + Send + Sync + Clone,
	FnMarker: 'static,
	Func: SystemParamFunction<FnMarker, Out = Result<Out>>,
	Func: IntoSystem<In<Input>, Result<Out>, (IsFunctionSystem, FnMarker)>,
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out> {
		system_tool(self)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn with_resource() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(system_tool(
				|In(()): In<()>, time: Res<Time>| -> Result<f32> {
					Ok(time.elapsed_secs())
				},
			))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), f32>(())
			.unwrap()
			.xpect_eq(0.0);
	}

	#[test]
	fn with_input() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(system_tool(
				|In(val): In<i32>, _time: Res<Time>| -> Result<i32> {
					Ok(val * 2)
				},
			))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<i32, i32>(21)
			.unwrap()
			.xpect_eq(42);
	}

	#[test]
	fn unit_in_unit_out() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(system_tool(|In(()): In<()>| -> Result { Ok(()) }))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), ()>(())
			.unwrap();
	}

	#[test]
	fn pipe_with_system() {
		#[tool]
		fn negate(val: i32) -> i32 { -val }

		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(
				system_tool(
					|In(val): In<i32>, _time: Res<Time>| -> Result<i32> {
						Ok(val * 2)
					},
				)
				.pipe(negate),
			)
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<i32, i32>(5)
			.unwrap()
			.xpect_eq(-10);
	}
}
