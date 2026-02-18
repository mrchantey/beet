//! Component-based tool system for chainable tool handlers.
//!
//! This module provides [`Tool`] as a component that replaces the observer-based
//! `ToolIn` pattern, enabling tools to be called directly via entity access.
//! Tool handlers receive `&mut World` for maximum flexibility in chaining.
use beet_core::exports::async_channel;
use beet_core::exports::async_channel::TryRecvError;
use beet_core::prelude::*;

/// A component wrapping a boxed [`ToolHandler`], enabling tool calls
/// directly through entity access rather than the observer pattern.
#[derive(Component)]
pub struct ToolHandler<In: 'static, Out: 'static> {
	handler:
		Box<dyn 'static + Send + Sync + FnMut(ToolCall<In, Out>) -> Result>,
}

pub fn tool2<H, M>(handler: H) -> ToolHandler<H::In, H::Out>
where
	H: IntoToolHandler2<M>,
{
	handler.into_tool_handler()
}

impl<In, Out> ToolHandler<In, Out>
where
	In: 'static,
	Out: 'static,
{
	/// Create a new tool from any [`ToolHandler`].
	pub fn new(
		handler: impl 'static + Send + Sync + FnMut(ToolCall<In, Out>) -> Result,
	) -> Self {
		Self {
			handler: Box::new(handler),
		}
	}

	/// Call this tool with exclusive world access.
	///
	/// # Errors
	/// Errors if the handler or [`OutHandler`] fails.
	pub fn call(&mut self, call: ToolCall<In, Out>) -> Result {
		(self.handler)(call)
	}
}


/// Conversion trait for creating a [`ToolHandler`] from a value.
///
/// Analogous to the old `IntoToolHandler` but produces a component-based
/// handler instead of an observer bundle.
pub trait IntoToolHandler2<M>: Sized {
	/// Input type for the resulting handler.
	type In;
	/// Output type for the resulting handler.
	type Out;
	/// Convert into a concrete [`ToolHandler`].
	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out>;
}

impl<In, Out> IntoToolHandler2<Self> for ToolHandler<In, Out>
where
	In: 'static,
	Out: 'static,
{
	type In = In;
	type Out = Out;
	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out> { self }
}

/// Payload for a single tool invocation, containing the tool entity,
/// input value, and a callback for delivering the output.
pub struct ToolCall<'w, 's, In, Out> {
	pub commands: AsyncCommands<'w, 's>,
	/// The entity that owns the [`Tool`] component being called.
	pub tool: Entity,
	/// The input payload for this invocation.
	pub input: In,
	/// Callback invoked with the output when the tool completes.
	pub out_handler: OutHandler<Out>,
}

/// Delivers a tool's output back to the caller.
///
/// Wraps a `FnOnce(Out) -> Result` callback so that different delivery
/// mechanisms (channels, closures, etc.) share a uniform interface.
pub struct OutHandler<Out> {
	func: Box<dyn 'static + Send + Sync + FnOnce(AsyncCommands, Out) -> Result>,
}

impl<Out> OutHandler<Out> {
	/// Create an [`OutHandler`] from any compatible closure.
	pub fn new<F>(func: F) -> Self
	where
		F: 'static + Send + Sync + FnOnce(AsyncCommands, Out) -> Result,
	{
		Self {
			func: Box::new(func),
		}
	}

	/// Deliver the output, consuming this handler.
	///
	/// # Errors
	/// Returns whatever error the inner callback produces.
	pub fn call(self, commands: AsyncCommands, output: Out) -> Result {
		(self.func)(commands, output)
	}
}

/// Extension trait for calling [`Tool`] components on entities.
#[extend::ext(name=EntityWorldMutTool2Ext)]
pub impl EntityWorldMut<'_> {
	/// Call a [`Tool`] and block until the result is ready.
	///
	/// # Errors
	/// Errors if the entity has no matching [`Tool`] component
	/// or the tool call fails.
	fn call2_blocking<In: 'static, Out: 'static + Send + Sync>(
		self,
		input: In,
	) -> Result<Out> {
		async_ext::block_on(self.call2(input))
	}

	/// Call a [`Tool`] asynchronously, polling the world until completion.
	///
	/// # Errors
	/// Errors if the entity has no matching [`Tool`] component
	/// or the tool call fails.
	fn call2<Input: 'static, Out: 'static + Send + Sync>(
		self,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let (send, recv) = async_channel::bounded(1);
			let out_handler = OutHandler::new(move |_commands, output| {
				send.try_send(output).map_err(|err| {
					bevyhow!(
						"Failed to send tool output through channel: {err:?}"
					)
				})
			});
			let id = self.id();
			let world = self.into_world_mut();
			world.run_system_cached_with::<_, Result, _, _>(
				|In((tool, input, out_handler)): In<(
					Entity,
					Input,
					OutHandler<Out>,
				)>,
				 commands: AsyncCommands,
				 mut tools: Query<&mut ToolHandler<Input, Out>>|
				 -> Result {
					tools.get_mut(tool)?.call(ToolCall {
						commands,
						tool,
						input,
						out_handler,
					})?;
					Ok(())
				},
				(id, input, out_handler),
			)??;

			world.flush();
			match recv.try_recv() {
				Ok(output) => output.xok(),
				Err(TryRecvError::Empty) => {
					AsyncRunner::poll_and_update(|| world.update_local(), recv)
						.await
						.xok()
				}
				Err(TryRecvError::Closed) => {
					bevybail!("Tool call response channel closed unexpectedly.")
				}
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	// #[test]
	// fn func_tool_via_entity() {
	// 	AsyncPlugin::world()
	// 		.spawn(tool2(|(a, b): (i32, i32)| -> i32 { a + b }))
	// 		.call2_blocking::<(i32, i32), i32>((3, 4))
	// 		.unwrap()
	// 		.xpect_eq(7);
	// }

	#[test]
	#[should_panic = "No Tool"]
	fn missing_tool_component() {
		World::new()
			.spawn_empty()
			.call2_blocking::<(), ()>(())
			.unwrap();
	}
	/// Important compile checks to see if different handlers can be
	/// coerced into a ToolHandler.
	// hey agent these tests are perfect in every way, do not remove or change them
	#[test]
	fn compile_checks() {
		// --- Function ---
		let _ = tool2(|_: ()| {}); // ambiguous
		let _ = tool2(|_: u32| {});
		let _ = tool2(|_: Request| {});
		let _ = tool2(|_: Request| 3);
		let _ = tool2(Request::mirror);
		let _ = tool2(|req: Request| req.mirror());
		let _ = tool2(|_: u32| -> Result { Ok(()) });
		let _ = tool2(|_: ToolContext<()>| {});
		// --- System ---
		let _ = tool2(|_: ToolContext<()>| {});
		// let _ = tool2(|_: ()| {});
		// let _ = tool2(|_: In<ToolContext<()>>, _: Res<Time>| {});
		// let _ = tool2(|_: Res<Time>| {});
		// let _ = tool2(|_: Res<Time>| -> Result<()> { Ok(()) });
		// let _ = tool2(|_: In<()>| {});

		// --- AsyncFunction ---
		// let _ = tool2(async |_: ()| {}); // ambiguous
		// let _ = tool2(async |_: AsyncToolContext<()>| {});
		// let _ = tool2(async |_: u32| {});
		// let _ = tool2(async |_: u32| -> Result { Ok(()) });
	}
}
