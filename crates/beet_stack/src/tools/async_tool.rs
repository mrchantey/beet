//! [`IntoToolHandler`] implementation for async closures.
//!
//! Any `Fn(Arg) -> impl Future<Output = Out>` where `Arg` implements
//! [`FromAsyncToolContext`] automatically becomes a tool handler. Unlike
//! [`func_tool`](super::func_tool), async tools can perform non-blocking
//! work such as network requests or streaming without stalling the ECS.
//!
//! ## Extractors
//!
//! The closure's argument is created via [`FromAsyncToolContext`], which
//! allows extracting either the raw input payload, a [`ToolContext`], or
//! the full [`AsyncToolContext`] (payload + [`AsyncEntity`]).
//!
//! ## Examples
//!
//! ```rust,no_run
//! # use beet_stack::prelude::*;
//! # use beet_core::prelude::*;
//! // Async tool that returns after an async operation
//! let handler = tool(async |val: u32| -> u32 { val * 2 });
//!
//! // Access the async entity handle
//! let handler = tool(async |cx: AsyncToolContext<String>| -> String {
//!     format!("received: {}", cx.input)
//! });
//! ```
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;

/// Marker for the async function [`IntoToolHandler`] impl.
pub struct AsyncToolMarker;

impl<Func, In, Fut, Arg, Out, IntoOut, IntoOutM, ArgM>
	IntoToolHandler<(AsyncToolMarker, In, Arg, Out, IntoOut, IntoOutM, ArgM)>
	for Func
where
	Func: 'static + Send + Sync + Clone + Fn(Arg) -> Fut,
	Arg: 'static + Send + Sync + FromAsyncToolContext<In, ArgM>,
	In: 'static + Send + Sync,
	Fut: 'static + Send + Future<Output = IntoOut>,
	IntoOut: 'static + IntoToolOutput<Out, IntoOutM>,
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
				let async_entity = commands.world().entity(tool);
				let arg = Arg::from_async_tool_context(AsyncToolContext {
					tool: async_entity,
					input,
				});
				let func = self.clone();
				commands.run(async move |world: AsyncWorld| -> Result {
					let output = func(arg).await.into_tool_output()?;

					// Obtain fresh AsyncCommands via SystemState so
					// the out_handler (and any downstream pipe
					// handlers) can queue further work.
					world
						.with_then(move |world: &mut World| -> Result {
							let result = {
								let mut state =
									SystemState::<AsyncCommands>::new(world);
								let async_commands = state.get_mut(world);
								let result =
									out_handler.call(async_commands, output);
								state.apply(world);
								result
							};
							world.flush();
							result
						})
						.await
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
	fn async_pure() {
		AsyncPlugin::world()
			.spawn(tool(async |(a, b): (i32, i32)| -> i32 { a + b }))
			.call_blocking::<(i32, i32), i32>((3, 4))
			.unwrap()
			.xpect_eq(7);
	}

	#[test]
	fn async_negate() {
		AsyncPlugin::world()
			.spawn(tool(async |val: i32| -> i32 { -val }))
			.call_blocking::<i32, i32>(42)
			.unwrap()
			.xpect_eq(-42);
	}

	#[test]
	fn async_tool_context() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(tool(async |cx: AsyncToolContext<()>| -> Entity {
				cx.tool.id()
			}))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), Entity>(())
			.unwrap()
			.xpect_eq(entity);
	}

	#[test]
	fn async_result_output() {
		AsyncPlugin::world()
			.spawn(tool(async |_: u32| -> Result { Ok(()) }))
			.call_blocking::<u32, ()>(99)
			.unwrap();
	}

	#[test]
	fn async_string_processing() {
		AsyncPlugin::world()
			.spawn(tool(async |val: String| -> String {
				format!("hello {val}")
			}))
			.call_blocking::<String, String>("world".to_string())
			.unwrap()
			.xpect_eq("hello world".to_string());
	}
}
