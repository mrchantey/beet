use beet_core::prelude::*;
use std::sync::Arc;

#[derive(Component)]
#[component(on_add=on_add::<In, Out>)]
pub struct ToolHandler<In: 'static, Out: 'static> {
	/// The full type name of the handler, for display and debugging.
	handler_meta: TypeMeta,
	handler: Arc<dyn 'static + Send + Sync + Fn(ToolCall<In, Out>) -> Result>,
}

impl<In: 'static, Out: 'static> Clone for ToolHandler<In, Out> {
	fn clone(&self) -> Self {
		Self {
			handler_meta: self.handler_meta,
			handler: Arc::clone(&self.handler),
		}
	}
}

fn on_add<In: 'static, Out: 'static>(
	mut world: DeferredWorld,
	cx: HookContext,
) {
	let handler = world
		.entity(cx.entity)
		.get::<ToolHandler<In, Out>>()
		.unwrap()
		.handler_meta
		.clone();
	let meta = ToolMeta {
		handler,
		input: TypeMeta::of::<In>(),
		output: TypeMeta::of::<Out>(),
	};

	world.commands().entity(cx.entity).insert(meta);
}

impl<In, Out> ToolHandler<In, Out>
where
	In: 'static,
	Out: 'static,
{
	pub fn new(
		handler_meta: TypeMeta,
		handler: impl 'static + Send + Sync + Fn(ToolCall<In, Out>) -> Result,
	) -> Self {
		Self {
			handler_meta,
			handler: Arc::new(handler),
		}
	}

	pub fn handler_meta(&self) -> TypeMeta { self.handler_meta }


	/// Invoke this tool handler with the given [`ToolCall`].
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call(&self, call: ToolCall<In, Out>) -> Result {
		(self.handler)(call)
	}
}

#[derive(Copy, Clone, Debug, Component)]
pub struct ToolMeta {
	/// Type metadata for the tool handler.
	handler: TypeMeta,
	/// Type metadata for the tool input.
	input: TypeMeta,
	/// Type metadata for the tool output.
	output: TypeMeta,
}

impl ToolMeta {
	/// Create a [`ToolMeta`] from handler, input and output type parameters.
	pub fn of<H: 'static, In: 'static, Out: 'static>() -> Self {
		Self {
			handler: TypeMeta::of::<H>(),
			input: TypeMeta::of::<In>(),
			output: TypeMeta::of::<Out>(),
		}
	}

	/// Returns true if this tool natively handles [`Request`]/[`Response`].
	#[cfg(feature = "exchange")]
	pub fn is_exchange(&self) -> bool {
		self.input.type_id() == std::any::TypeId::of::<Request>()
			&& self.output.type_id() == std::any::TypeId::of::<Response>()
	}
	/// Get the handler type metadata for this tool.
	pub fn handler(&self) -> TypeMeta { self.handler }
	/// The full type name of the handler function or type.
	pub fn name(&self) -> &'static str { self.handler.type_name }
	/// Get the input type metadata for this tool.
	pub fn input(&self) -> TypeMeta { self.input }
	/// Get the output type metadata for this tool.
	pub fn output(&self) -> TypeMeta { self.output }

	/// Assert that the provided types match this tool's input/output types.
	///
	/// # Errors
	/// Returns an error if types don't match.
	pub fn assert_match<In: 'static, Out: 'static>(&self) -> Result {
		let expected_input = self.input();
		let expected_output = self.output();
		let received_input = TypeMeta::of::<In>();
		let received_output = TypeMeta::of::<Out>();
		if expected_input != received_input {
			bevybail!(
				"Tool Call Input mismatch.\nExpected: {}\nReceived: {}.",
				expected_input,
				received_input,
			);
		} else if expected_output != received_output {
			bevybail!(
				"Tool Call Output mismatch.\nExpected: {}\nReceived: {}.",
				expected_output,
				received_output,
			);
		} else {
			Ok(())
		}
	}
}

/// Lightweight type metadata using [`TypeId`](std::any::TypeId) for
/// comparison and [`type_name`](std::any::type_name) for display.
#[derive(Debug, Copy, Clone)]
pub struct TypeMeta {
	type_name: &'static str,
	type_id: std::any::TypeId,
}

impl TypeMeta {
	/// Create a [`TypeMeta`] for the given type.
	pub fn of<T: 'static>() -> Self {
		Self {
			type_name: std::any::type_name::<T>(),
			type_id: std::any::TypeId::of::<T>(),
		}
	}
	pub fn of_val<T: 'static>(_: &T) -> Self { Self::of::<T>() }

	/// The full type name, ie `core::option::Option<i32>`.
	pub fn type_name(&self) -> &'static str { self.type_name }
	/// The [`TypeId`](std::any::TypeId) for this type.
	pub fn type_id(&self) -> std::any::TypeId { self.type_id }
}

impl std::fmt::Display for TypeMeta {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.type_name)
	}
}

impl PartialEq for TypeMeta {
	fn eq(&self, other: &Self) -> bool { self.type_id == other.type_id }
}

/// Payload for a single tool invocation, containing the tool entity,
/// input value, [`AsyncCommands`] for queuing work, and a callback
/// for delivering the output.
pub struct ToolCall<'w, 's, In, Out> {
	/// Commands for queuing ECS work or spawning async tasks.
	pub commands: AsyncCommands<'w, 's>,
	/// The entity that owns the [`ToolHandler`] component being called.
	pub tool: Entity,
	/// The input payload for this invocation.
	pub input: In,
	/// Callback invoked with the output when the tool completes.
	pub out_handler: OutHandler<Out>,
}

impl<'w, 's, In, Out> ToolCall<'w, 's, In, Out> {}

/// Delivers a tool's output back to the caller.
///
/// Wraps a closure so that different delivery mechanisms (channels,
/// pipe chains, etc.) share a uniform interface.
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


/// Conversion trait for creating a [`ToolHandler`] from a value.
///
/// Implementations exist for plain closures
/// ([`func_tool`](super::func_tool)), Bevy systems
/// ([`system_tool`](super::system_tool)), and async closures
/// ([`async_tool`](super::async_tool)).
pub trait IntoToolHandler<M>: Sized {
	/// Input type for the resulting handler.
	type In;
	/// Output type for the resulting handler.
	type Out;
	/// Convert into a concrete [`ToolHandler`].
	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out>;
}

impl<In, Out> IntoToolHandler<()> for ToolHandler<In, Out>
where
	In: 'static,
	Out: 'static,
{
	type In = In;
	type Out = Out;

	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out> { self }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	#[should_panic = "No Tool"]
	fn missing_tool_component() {
		AsyncPlugin::world()
			.spawn_empty()
			.call_blocking::<(), ()>(())
			.unwrap();
	}
}
