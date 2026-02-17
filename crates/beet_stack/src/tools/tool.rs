use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::exports::async_channel::Sender;
use beet_core::exports::async_channel::TryRecvError;
use beet_core::prelude::*;

/// Create a tool from a handler implementing [`IntoToolHandler`],
/// adding an associated [`ToolMeta`] which is required for tool calling.
///
/// This is the base tool constructor that adds [`ToolMeta`] and
/// the handler bundle. For tools that need to be called via
/// [`Request`]/[`Response`] serialization, use [`route_tool`] instead.
pub fn tool<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
{
	(ToolMeta::of::<H, H::In, H::Out>(), handler.into_handler())
}



/// Metadata for a tool, containing the handler name and input/output types.
/// Every tool must have a [`ToolMeta`], calling a tool with
/// types that dont match will result in an error.
#[derive(Clone, Debug, Component)]
pub struct ToolMeta {
	/// The full type name of the handler, ie `my_crate::my_tool_handler`.
	name: &'static str,
	/// Type metadata for the tool input.
	input: TypeMeta,
	/// Type metadata for the tool output.
	output: TypeMeta,
}

impl ToolMeta {
	/// Returns true if this tool natively handles [`Request`]/[`Response`].
	pub fn is_exchange(&self) -> bool {
		self.input.type_id() == std::any::TypeId::of::<Request>()
			&& self.output.type_id() == std::any::TypeId::of::<Response>()
	}
}
/// Lightweight type metadata using [`TypeId`](std::any::TypeId) for comparison
/// and [`type_name`](std::any::type_name) for display.
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

impl ToolMeta {
	/// Create a [`ToolMeta`] from handler, input and output type parameters.
	///
	/// `H` is the handler type whose [`type_name`](std::any::type_name)
	/// is stored for display purposes (ie button labels).
	pub fn of<H: 'static, In: 'static, Out: 'static>() -> Self {
		Self {
			name: std::any::type_name::<H>(),
			input: TypeMeta::of::<In>(),
			output: TypeMeta::of::<Out>(),
		}
	}

	/// The full type name of the handler function or type.
	pub fn name(&self) -> &'static str { self.name }
	/// Get the input type metadata for this tool.
	pub fn input(&self) -> TypeMeta { self.input }
	/// Get the output type metadata for this tool.
	pub fn output(&self) -> TypeMeta { self.output }

	/// Assert that the provided types match this tool's input/output types.
	///
	/// ## Errors
	///
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

/// An event emitted on the tool when it is called, containing the tool, payload and a
/// method to call [`on_out`].
#[derive(EntityEvent)]
pub struct ToolIn<In = (), Out = ()> {
	/// The entity containing an observer to react
	/// to the tool call. This is the [`EntityEvent::event_target`].
	#[event_target]
	tool: Entity,
	/// The payload of the tool input, which may only be consumed once
	input: Option<In>,
	/// Called by the tool on done, which may only be consumed once.
	out_handler: Option<ToolOutHandler<Out>>,
}

impl<In, Out> ToolIn<In, Out> {
	/// Create a new [`ToolIn`] event with the given caller, tool, and payload.
	pub fn new(tool: Entity, payload: In, on_out: ToolOutHandler<Out>) -> Self {
		Self {
			tool,
			input: Some(payload),
			out_handler: Some(on_out),
		}
	}
	/// Get the tool entity for this event, aka [`Self::event_target`]
	pub fn tool(&self) -> Entity { self.tool }


	/// Take the payload from this event. This must only be done once.
	///
	/// # Errors
	///
	/// Errors if the payload has already been taken.
	pub fn take_input(&mut self) -> Result<In> {
		self.input
			.take()
			.ok_or_else(|| bevyhow!("Tool call payload already taken. Are there multiple handlers on the same entity?"))
	}

	/// Call the on_out callback with the given output. This must only be done once.
	///
	/// # Errors
	///
	/// Errors if the [`on_out`] method has already been called.
	pub fn take_out_handler(&mut self) -> Result<ToolOutHandler<Out>> {
		self
			.out_handler.take()
			.ok_or_else(|| bevyhow!("Tool call on_out already called. This may be a bug in the IntoToolHandler implementation."))
	}
}

/// An event emitted on the tool when it is called, containing the tool, payload and a
/// method to call [`on_out`].
#[derive(EntityEvent)]
pub struct ToolOut<Out = ()> {
	/// The entity containing the tool handler that sent this [`ToolOut`] event.
	tool: Entity,
	/// The entity that originally sent the tool call,
	/// This is the [`EntityEvent::event_target`].
	#[event_target]
	caller: Entity,
	/// The payload of the tool output, which may only be consumed once
	output: Option<Out>,
}

impl<Out> ToolOut<Out> {
	/// Create a new [`ToolIn`] event with the given caller, tool, and payload.
	pub fn new(tool: Entity, caller: Entity, output: Out) -> Self {
		Self {
			tool,
			caller,
			output: Some(output),
		}
	}

	/// Get the tool entity for this event
	pub fn tool(&self) -> Entity { self.tool }
	/// Get the tool caller entity for this event, aka [`Self::event_target`]
	pub fn caller(&self) -> Entity { self.caller }

	/// Take the payload from this event. This must only be done once.
	///
	/// # Errors
	///
	/// Errors if the payload has already been taken.
	pub fn take_output(&mut self) -> Result<Out> {
		self.output
			.take()
			.ok_or_else(|| bevyhow!("ToolOut payload already taken. Are there multiple handlers on the same entity?"))
	}
}

/// Handles the output of a tool call.
///
/// Determines how the tool's output is delivered back to the caller.
pub enum ToolOutHandler<Out> {
	/// The tool was called by another entity which
	/// does not need to track individual calls,
	/// and instead will listen for a [`ToolOut`]
	/// event triggered on this caller.
	Observer {
		/// The entity that will receive the [`ToolOut`] event.
		caller: Entity,
	},
	/// The tool caller is listening on a channel.
	Channel {
		/// The sender side of the channel for delivering results.
		sender: Sender<Out>,
	},
	/// The tool caller provided a callback to call on completion.
	Function {
		/// The boxed function to call with the output.
		handler: Box<dyn 'static + Send + Sync + FnOnce(Out) -> Result>,
	},
}

impl<Out: 'static + Send + Sync> ToolOutHandler<Out> {
	/// Create a handler that triggers a [`ToolOut`] event on the caller.
	pub fn observer(caller: Entity) -> Self { Self::Observer { caller } }
	/// Create a handler that sends output through a channel.
	pub fn channel(sender: Sender<Out>) -> Self { Self::Channel { sender } }
	/// Create a handler from a function.
	pub fn function(
		handler: impl 'static + Send + Sync + FnOnce(Out) -> Result,
	) -> Self {
		Self::Function {
			handler: Box::new(handler),
		}
	}

	/// Call this handler with the tool output.
	///
	/// ## Errors
	///
	/// Returns an error if the handler fails.
	pub fn call(
		self,
		mut commands: Commands,
		tool: Entity,
		output: Out,
	) -> Result {
		match self {
			ToolOutHandler::Observer { caller } => {
				commands.trigger(ToolOut::new(tool, caller, output));
				Ok(())
			}
			ToolOutHandler::Channel { sender } => {
				sender.try_send(output).map_err(|err| {
					bevyhow!(
						"Failed to send tool output through channel: {err:?}"
					)
				})
			}
			ToolOutHandler::Function { handler } => handler(output),
		}
	}
	/// Call this handler asynchronously with the tool output.
	///
	/// ## Errors
	///
	/// Returns an error if the handler fails.
	pub fn call_async(
		self,
		world: &mut AsyncWorld,
		tool: Entity,
		output: Out,
	) -> Result {
		match self {
			ToolOutHandler::Observer { caller } => {
				world.trigger(ToolOut::new(tool, caller, output));
				Ok(())
			}
			ToolOutHandler::Channel { sender } => {
				sender.try_send(output).map_err(|err| {
					bevyhow!(
						"Failed to send tool output through channel: {err:?}"
					)
				})
			}
			ToolOutHandler::Function { handler } => handler(output),
		}
	}
}

#[extend::ext(name=AsyncEntityToolExt)]
pub impl AsyncEntity {
	/// Make a tool call asynchronously.
	/// ## Errors
	///
	/// Errors if the entity has no [`ToolMeta`] or the [`ToolMeta`] does
	/// not match the provided types.
	///
	fn call<In: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&self,
		input: In,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let (send, recv) = async_channel::bounded::<Out>(1);
			let handler = ToolOutHandler::channel(send);

			self.with_then(move |mut entity| {
				entity.call_with_handler(input, handler)
			})
			.await?;

			recv.recv().await?.xok()
		}
	}
}
/// Extension trait for calling tools on entities.
#[extend::ext(name=EntityWorldMutToolExt)]
pub impl EntityWorldMut<'_> {
	/// Make a tool call and block until the result is ready.
	///
	/// ## Errors
	///
	/// Errors if the tool call fails.
	/// Errors if the entity has no [`ToolMeta`] or the [`ToolMeta`] does
	/// not match the provided types.
	fn call_blocking<In: 'static + Send + Sync, Out: 'static + Send + Sync>(
		self,
		input: In,
	) -> Result<Out> {
		async_ext::block_on(self.call(input))
	}

	/// Make a tool call asynchronously, updating the world until it concludes.
	///
	/// ## Errors
	///
	/// Errors if the tool call fails.
	/// Errors if the entity has no [`ToolMeta`] or the [`ToolMeta`] does
	/// not match the provided types.
	fn call<In: 'static + Send + Sync, Out: 'static + Send + Sync>(
		mut self,
		input: In,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let (send, recv) = async_channel::bounded(1);
			let handler = ToolOutHandler::channel(send);
			self.call_with_handler(input, handler)?;

			let world = self.into_world_mut();

			world.flush();
			// check if response was sent synchronously
			let out: Out = match recv.try_recv() {
				Ok(response) => response,
				Err(TryRecvError::Empty) => {
					// poll async tasks until we get a response
					AsyncRunner::poll_and_update(|| world.update_local(), recv)
						.await
				}
				Err(TryRecvError::Closed) => {
					bevybail!("Tool call response channel closed unexpectedly.")
				}
			};
			out.xok()
		}
	}
	/// Perform a tool call on the given entity,
	/// checking that the input and output types match the entity's [`ToolMeta`].
	///
	/// If the entity has a [`RouteToolMarker`], calls with
	/// `Request`/`Response` types are accepted and routed
	/// through the exchange handler. Typed calls on route tools
	/// are rejected — use the inner tool directly instead.
	///
	/// ## Note
	///
	/// This call returns immediately and does not update the world.
	///
	/// ## Errors
	///
	/// Errors if the entity has no [`ToolMeta`] or the [`ToolMeta`] does
	/// not match the provided types.
	fn call_with_handler<
		In: 'static + Send + Sync,
		Out: 'static + Send + Sync,
	>(
		&mut self,
		input: In,
		out_handler: ToolOutHandler<Out>,
	) -> Result {
		use std::any::TypeId;
		let is_request_response = TypeId::of::<In>() == TypeId::of::<Request>()
			&& TypeId::of::<Out>() == TypeId::of::<Response>();
		let is_route_tool = self.contains::<RouteToolMarker>();

		if is_route_tool {
			if !is_request_response {
				bevybail!(
					"Route tools only accept Request/Response calls. \
					 Use the inner tool entity for typed calls."
				);
			}
			// Request/Response on a route tool, allow it
		} else {
			self.get::<ToolMeta>()
				.ok_or_else(|| {
					bevyhow!("No ToolMeta on entity, cannot send tool call.")
				})?
				.assert_match::<In, Out>()?;
		}

		self.trigger(|entity| ToolIn::new(entity, input, out_handler));
		Ok(())
	}
}


/// Extension trait for calling tools on entities.
#[extend::ext(name=EntityCommandsToolExt)]
pub impl EntityCommands<'_> {
	/// Make a tool call with the provided input value and output handler.
	///
	/// ## Errors
	///
	/// Errors if there is a type mismatch between this entity's [`ToolMeta`]
	/// and this tool call.
	fn call<In: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&mut self,
		input: In,
		out_handler: ToolOutHandler<Out>,
	) {
		self.queue(|mut entity: EntityWorldMut| -> Result {
			entity.call_with_handler(input, out_handler)
		});
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn add_tool_handler((a, b): (i32, i32)) -> i32 { a + b }
	fn add_tool() -> impl Bundle {
		(PathPartial::new("add"), tool(add_tool_handler))
	}


	#[test]
	fn works() {
		World::new()
			.spawn(add_tool())
			.call_blocking::<(i32, i32), i32>((2, 2))
			.unwrap()
			.xpect_eq(4);
	}
	#[test]
	#[should_panic = "No Tool"]
	fn no_tool() {
		World::new()
			.spawn_empty()
			.call_blocking::<(), ()>(())
			.unwrap();
	}
	#[test]
	#[should_panic = "Input mismatch"]
	fn input_mismatch() {
		World::new()
			.spawn(add_tool())
			.call_blocking::<bool, i32>(true)
			.unwrap();
	}
	#[test]
	#[should_panic = "Output mismatch"]
	fn output_mismatch() {
		World::new()
			.spawn(add_tool())
			.call_blocking::<(i32, i32), bool>((2, 2))
			.unwrap();
	}

	#[test]
	fn path_pattern() {
		let mut world = RouterPlugin::world();
		world
			.spawn(add_tool())
			.get::<PathPattern>()
			.unwrap()
			.annotated_route_path()
			.to_string()
			.xpect_eq("/add");
		let root = world.spawn(PathPartial::new(":foo")).id();
		world
			.spawn((ChildOf(root), add_tool()))
			.get::<PathPattern>()
			.unwrap()
			.annotated_route_path()
			.to_string()
			.xpect_eq("/:foo/add");
	}

	#[test]
	fn params_pattern() {
		#[derive(Reflect)]
		struct MyParams {
			foo: u32,
		}

		let mut world = RouterPlugin::world();
		world
			.spawn(add_tool())
			.get::<ParamsPattern>()
			.unwrap()
			.xpect_empty();

		// params inserted after tool
		world
			.spawn((add_tool(), ParamsPartial::new::<MyParams>()))
			.get::<ParamsPattern>()
			.unwrap()[0]
			.name()
			.xpect_eq("foo");

		// ancestor
		let root = world.spawn(ParamsPartial::new::<MyParams>()).id();
		world
			.spawn((ChildOf(root), add_tool()))
			.get::<ParamsPattern>()
			.unwrap()[0]
			.name()
			.xpect_eq("foo");
	}
}
