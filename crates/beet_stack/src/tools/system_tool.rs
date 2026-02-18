//! [`IntoToolHandler`] implementation for Bevy systems.
//!
//! Any function that implements [`IntoSystem`] automatically becomes a
//! tool handler via this module. Unlike [`func_tool`](super::func_tool),
//! system tools have access to ECS queries, resources, and other system
//! parameters.
//!
//! ## Extractors
//!
//! The system's [`SystemInput`] is created via [`FromToolContext`],
//! allowing extraction of the raw input payload or the full
//! [`ToolContext`].
//!
//! ## Examples
//!
//! ```rust
//! # use beet_stack::prelude::*;
//! # use beet_core::prelude::*;
//! // A system tool that reads a resource
//! let handler = tool(|_: In<()>, time: Res<Time>| -> f32 {
//!     time.elapsed_secs()
//! });
//! ```
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::IsFunctionSystem;
use bevy::ecs::system::SystemParamFunction;
use bevy::ecs::system::SystemState;

/// Marker for the system tool [`IntoToolHandler`] impl.
///
/// This impl uses [`SystemParamFunction::Out`] to get the **actual**
/// return type of the function, bypassing Bevy's `IntoSystem`
/// `IntoResult` ambiguity where `Result<T, BevyError>` could resolve
/// as either `Result<T>` or `T`.
pub struct SystemToolMarker;

impl<Func, In, Arg, ArgM, Out, IntoOut, IntoOutM, FnMarker>
	IntoToolHandler<(
		SystemToolMarker,
		In,
		Arg,
		ArgM,
		Out,
		IntoOut,
		IntoOutM,
		FnMarker,
	)> for Func
where
	Func: 'static + Send + Sync + Clone,
	FnMarker: 'static,
	Func: SystemParamFunction<FnMarker, Out = IntoOut>,
	Func: IntoSystem<Arg, IntoOut, (IsFunctionSystem, FnMarker)>,
	Arg: 'static + SystemInput,
	for<'a> Arg::Inner<'a>: FromToolContext<In, ArgM>,
	In: 'static + Send + Sync,
	IntoOut: 'static + Send + Sync + IntoToolOutput<Out, IntoOutM>,
	Out: 'static + Send + Sync,
{
	type In = In;
	type Out = Out;

	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out> {
		ToolHandler::new(
			move |ToolCall {
			          mut commands,
			          tool,
			          input,
			          out_handler,
			      }| {
				let func = self.clone();
				commands.commands.queue(move |world: &mut World| -> Result {
					let arg = <Arg::Inner<'_> as FromToolContext<
							In,
							ArgM,
						>>::from_tool_context(
							ToolContext { tool, input }
						);
					let raw_output: IntoOut =
						world.run_system_cached_with(func, arg)?;
					let output = raw_output.into_tool_output()?;

					// Obtain fresh AsyncCommands via SystemState so
					// the out_handler (and any downstream pipe
					// handlers) can queue further work.
					let result = {
						let mut state =
							SystemState::<AsyncCommands>::new(world);
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
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn system_with_resource() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(tool(|_: In<()>, time: Res<Time>| -> f32 {
				time.elapsed_secs()
			}))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), f32>(())
			.unwrap()
			.xpect_eq(0.0);
	}

	#[test]
	fn system_with_input() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(tool(
				|In(cx): In<ToolContext<i32>>, _time: Res<Time>| -> i32 {
					*cx * 2
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
	fn system_no_input() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(tool(|_time: Res<Time>| -> String { "hello".to_string() }))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello".to_string());
	}

	#[test]
	fn system_result_output() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(tool(|_: Res<Time>| -> Result<()> { Ok(()) }))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), ()>(())
			.unwrap();
	}

	#[test]
	fn system_unit_in_unit_out() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(tool(|| {})).id();
		world
			.entity_mut(entity)
			.call_blocking::<(), ()>(())
			.unwrap();
	}
}
