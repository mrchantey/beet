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
pub struct Tool<In: 'static, Out: 'static> {
	handler: Box<dyn ToolHandler<In = In, Out = Out>>,
}

impl<In, Out> Tool<In, Out>
where
	In: 'static,
	Out: 'static,
{
	/// Create a new tool from any [`ToolHandler`].
	pub fn new<H>(handler: H) -> Self
	where
		H: 'static + ToolHandler<In = In, Out = Out>,
	{
		Self {
			handler: Box::new(handler),
		}
	}

	/// Call this tool with exclusive world access.
	///
	/// # Errors
	/// Errors if the handler or [`OutHandler`] fails.
	pub fn call(
		&mut self,
		world: &mut World,
		call: ToolCall<In, Out>,
	) -> Result {
		self.handler.call(world, call)
	}

	/// Downcast the inner handler to a concrete type.
	///
	/// # Errors
	/// Errors if the handler is not of the requested type.
	pub fn handler_as_mut<T>(&mut self) -> Result<&mut T>
	where
		T: ToolHandler<In = In, Out = Out> + 'static,
	{
		self.handler.as_any_mut().downcast_mut().ok_or_else(|| {
			bevyhow!("Failed to downcast tool handler to the requested type")
		})
	}
}

/// Trait for tool handler implementations that receive exclusive world access.
///
/// Unlike the observer-based `IntoToolHandler`, implementors receive `&mut World`
/// directly, making it straightforward to chain handlers or access ECS state.
pub trait ToolHandler: 'static + Send + Sync + AsAny {
	/// Input type for this handler.
	type In;
	/// Output type for this handler.
	type Out;

	/// Execute this handler with exclusive world access.
	///
	/// # Errors
	/// Errors if the handler logic or the [`OutHandler`] callback fails.
	fn call(
		&mut self,
		world: &mut World,
		call: ToolCall<Self::In, Self::Out>,
	) -> Result;
}

/// Blanket impl so any [`ToolHandler`] is also an [`IntoToolHandler2`].
impl<T> IntoToolHandler2<T> for T
where
	T: ToolHandler + 'static,
{
	type In = T::In;
	type Out = T::Out;
	fn into_tool_handler(
		self,
	) -> impl ToolHandler<In = Self::In, Out = Self::Out> {
		self
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
	fn into_tool_handler(
		self,
	) -> impl ToolHandler<In = Self::In, Out = Self::Out>;
}

/// Payload for a single tool invocation, containing the tool entity,
/// input value, and a callback for delivering the output.
pub struct ToolCall<In, Out> {
	/// The entity that owns the [`Tool`] component being called.
	pub tool: Entity,
	/// The input payload for this invocation.
	pub input: In,
	/// Callback invoked with the output when the tool completes.
	pub out_handler: OutHandler<Out>,
}

impl<In, Out> ToolCall<In, Out> {
	/// Create a new tool call.
	pub fn new(
		tool: Entity,
		input: In,
		out_handler: impl Into<OutHandler<Out>>,
	) -> Self {
		Self {
			tool,
			input,
			out_handler: out_handler.into(),
		}
	}
}

/// Delivers a tool's output back to the caller.
///
/// Wraps a `FnOnce(Out) -> Result` callback so that different delivery
/// mechanisms (channels, closures, etc.) share a uniform interface.
pub struct OutHandler<Out> {
	func: Box<dyn 'static + Send + Sync + FnOnce(Out) -> Result>,
}

impl<Out> OutHandler<Out> {
	/// Create an [`OutHandler`] from any compatible closure.
	pub fn new<F>(func: F) -> Self
	where
		F: 'static + Send + Sync + FnOnce(Out) -> Result,
	{
		Self {
			func: Box::new(func),
		}
	}

	/// Deliver the output, consuming this handler.
	///
	/// # Errors
	/// Returns whatever error the inner callback produces.
	pub fn call(self, output: Out) -> Result { (self.func)(output) }
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
	fn call2<In: 'static, Out: 'static + Send + Sync>(
		mut self,
		input: In,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let entity = self.id();
			let (send, recv) = async_channel::bounded(1);
			let out_handler = OutHandler::new(move |output: Out| {
				send.try_send(output).map_err(|err| {
					bevyhow!(
						"Failed to send tool output through channel: {err:?}"
					)
				})
			});

			let mut tool = self.take::<Tool<In, Out>>().ok_or_else(|| {
				bevyhow!(
					"No Tool<{}, {}> component on entity",
					std::any::type_name::<In>(),
					std::any::type_name::<Out>()
				)
			})?;

			let call = ToolCall::new(entity, input, out_handler);
			let world = self.into_world_mut();
			tool.call(world, call)?;

			// Reinsert the tool component
			world.entity_mut(entity).insert(tool);

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

	#[test]
	fn func_tool_via_entity() {
		let mut world = World::new();
		let tool = FuncTool::new(|cx: ToolContext<(i32, i32)>| -> i32 {
			cx.input.0 + cx.input.1
		});
		world
			.spawn(Tool::new(tool))
			.call2_blocking::<(i32, i32), i32>((3, 4))
			.unwrap()
			.xpect_eq(7);
	}

	#[test]
	#[should_panic = "No Tool"]
	fn missing_tool_component() {
		World::new()
			.spawn_empty()
			.call2_blocking::<(), ()>(())
			.unwrap();
	}
}
