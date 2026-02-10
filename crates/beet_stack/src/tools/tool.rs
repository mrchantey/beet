use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::exports::async_channel::Sender;
use beet_core::exports::async_channel::TryRecvError;
use beet_core::prelude::*;

/// Marker component indicating this tool supports [`Request`]/[`Response`]
/// calls via automatic serialization. Added by [`exchange_tool`] or
/// automatically by [`tool`] when the `interface` feature is enabled.
#[derive(Debug, Component)]
pub struct ExchangeToolMarker;


/// Create a tool from a handler implementing [`IntoToolHandler`],
/// adding an associated [`ToolMeta`] which is required for tool calling.
///
/// When the `interface` feature is enabled, this automatically adds
/// exchange support for [`Request`]/[`Response`] calls via serialization.
/// Use [`direct_tool`] if you need a tool without exchange support.
#[cfg(feature = "interface")]
pub fn tool<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
	H::In: serde::de::DeserializeOwned,
	H::Out: serde::Serialize,
{
	exchange_tool(handler)
}

/// Create a tool from a handler implementing [`IntoToolHandler`],
/// adding an associated [`ToolMeta`] which is required for tool calling.
///
/// Without the `interface` feature, this creates a direct tool
/// without exchange support.
#[cfg(not(feature = "interface"))]
pub fn tool<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
{
	direct_tool(handler)
}

/// Create a tool without exchange support, regardless of feature flags.
///
/// This is the base tool constructor that only adds [`ToolMeta`] and
/// the handler. Use this when the handler's input/output types do not
/// implement [`Serialize`](serde::Serialize)/[`Deserialize`](serde::de::DeserializeOwned),
/// or when exchange support is not needed.
pub fn direct_tool<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
{
	(ToolMeta::of::<H::In, H::Out>(), handler.into_handler())
}



/// Metadata for a tool, containing the input and output types.
/// Every tool must have a [`ToolMeta`], calling a tool with
/// types that dont match will result in an error.
#[derive(Clone, Debug, Component)]
pub struct ToolMeta {
	/// Type metadata for the tool input.
	input: TypeMeta,
	/// Type metadata for the tool output.
	output: TypeMeta,
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
	/// Create a [`ToolMeta`] from input and output type parameters.
	pub fn of<In: 'static, Out: 'static>() -> Self {
		Self {
			input: TypeMeta::of::<In>(),
			output: TypeMeta::of::<Out>(),
		}
	}

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
	payload: Option<In>,
	/// Called by the tool on done, which may only be consumed once.
	out_handler: Option<ToolOutHandler<Out>>,
}

impl<In, Out> ToolIn<In, Out> {
	/// Create a new [`ToolIn`] event with the given caller, tool, and payload.
	pub fn new(tool: Entity, payload: In, on_out: ToolOutHandler<Out>) -> Self {
		Self {
			tool,
			payload: Some(payload),
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
	pub fn take_payload(&mut self) -> Result<In> {
		self.payload
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
	payload: Option<Out>,
}

impl<Out> ToolOut<Out> {
	/// Create a new [`ToolIn`] event with the given caller, tool, and payload.
	pub fn new(tool: Entity, caller: Entity, payload: Out) -> Self {
		Self {
			tool,
			caller,
			payload: Some(payload),
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
	pub fn take_payload(&mut self) -> Result<Out> {
		self.payload
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
		payload: Out,
	) -> Result {
		match self {
			ToolOutHandler::Observer { caller } => {
				commands.trigger(ToolOut::new(tool, caller, payload));
				Ok(())
			}
			ToolOutHandler::Channel { sender } => {
				sender.try_send(payload).map_err(|err| {
					bevyhow!(
						"Failed to send tool output through channel: {err:?}"
					)
				})
			}
			ToolOutHandler::Function { handler } => handler(payload),
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
		payload: Out,
	) -> Result {
		match self {
			ToolOutHandler::Observer { caller } => {
				world.trigger(ToolOut::new(tool, caller, payload));
				Ok(())
			}
			ToolOutHandler::Channel { sender } => {
				sender.try_send(payload).map_err(|err| {
					bevyhow!(
						"Failed to send tool output through channel: {err:?}"
					)
				})
			}
			ToolOutHandler::Function { handler } => handler(payload),
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
	/// Returns an error if the tool call fails or types don't match.
	fn call_blocking<In: 'static + Send + Sync, Out: 'static + Send + Sync>(
		self,
		input: In,
	) -> Result<Out> {
		async_ext::block_on(self.call(input))
	}

	/// Make a tool call asynchronously.
	///
	/// ## Errors
	///
	/// Returns an error if the tool call fails or types don't match.
	fn call<In: 'static + Send + Sync, Out: 'static + Send + Sync>(
		mut self,
		input: In,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let (send, recv) = async_channel::bounded(1);
			let handler = ToolOutHandler::channel(send);
			trigger_checked(&mut self, input, handler)?;

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
			trigger_checked(&mut entity, input, out_handler)
		});
	}
}

/// Perform a tool call on the given entity,
/// checking that the input and output types match the entity's [`ToolMeta`].
///
/// If the entity has an [`ExchangeToolMarker`], calls with
/// `Request`/`Response` types are also accepted and routed
/// through the exchange handler.
fn trigger_checked<In: 'static + Send + Sync, Out: 'static + Send + Sync>(
	entity: &mut EntityWorldMut,
	input: In,
	out_handler: ToolOutHandler<Out>,
) -> Result {
	let meta = entity.get::<ToolMeta>().ok_or_else(|| {
		bevyhow!("No ToolMeta on entity, cannot send tool call.")
	})?;

	let is_exchange_call = TypeMeta::of::<In>() == TypeMeta::of::<Request>()
		&& TypeMeta::of::<Out>() == TypeMeta::of::<Response>();

	if is_exchange_call {
		// allow Request/Response calls only if the entity has an exchange handler
		#[cfg(feature = "interface")]
		{
			if !entity.contains::<ExchangeToolMarker>() {
				bevybail!(
					"Tool does not support Request/Response calls. \
					 Use exchange_tool() to enable serialized exchange."
				);
			}
		}
		#[cfg(not(feature = "interface"))]
		{
			bevybail!(
				"Request/Response exchange calls require the 'interface' feature."
			);
		}
	} else {
		meta.assert_match::<In, Out>()?;
	}

	entity.trigger(|entity| ToolIn::new(entity, input, out_handler));
	Ok(())
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
