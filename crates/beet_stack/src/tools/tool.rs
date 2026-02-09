use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::exports::async_channel::Sender;
use beet_core::exports::async_channel::TryRecvError;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;


/// Create a tool from a handler implementing [`IntoToolHandler`],
/// adding an associated [`ToolMeta`] which is required for tool calling.
pub fn tool<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
{
	let meta = ToolMeta {
		input: H::In::type_info(),
		output: H::Out::type_info(),
	};

	(meta, handler.into_handler())
}

/// Observer that listens for new tools and inserts their path and params patterns.
/// Any [`PathPartial`] or [`ParamsPartial`] will be collected so long as they are
/// spawned at the same time as the tool, even if they come after it in the tuple.
/// This is because, unlike OnAdd component hooks, observers run after the entire
/// tree is spawned.
pub fn insert_tool_path_and_params(
	ev: On<Insert, ToolMeta>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	params: Query<&ParamsPartial>,
	mut commands: Commands,
) -> Result {
	let path = PathPattern::collect(ev.entity, &ancestors, &paths)?;
	let params = ParamsPattern::collect(ev.entity, &ancestors, &params)?;
	commands.entity(ev.entity).insert((path, params));
	Ok(())
}


/// Metadata for a tool, containing the input and output types.
/// Every tool must have a [`ToolMeta`], calling a tool with
/// types that dont match will result in an error.
#[derive(Clone, Debug, Component)]
pub struct ToolMeta {
	/// The reflected type information for the tool input.
	input: &'static TypeInfo,
	/// The reflected type information for the tool output.
	output: &'static TypeInfo,
}

impl ToolMeta {
	/// Create a [`ToolMeta`] from input and output type parameters.
	pub fn of<In: Typed, Out: Typed>() -> Self {
		Self {
			input: In::type_info(),
			output: Out::type_info(),
		}
	}

	/// Get the input type information for this tool.
	pub fn input(&self) -> &'static TypeInfo { self.input }
	/// Get the output type information for this tool.
	pub fn output(&self) -> &'static TypeInfo { self.output }

	/// Assert that the provided types match this tool's input/output types.
	///
	/// ## Errors
	///
	/// Returns an error if types don't match.
	pub fn assert_match<In: Typed, Out: Typed>(&self) -> Result {
		let expected_input = self.input().type_path();
		let expected_output = self.output().type_path();
		let received_input = In::type_info().type_path();
		let received_output = Out::type_info().type_path();
		if expected_input != received_input {
			bevybail!(
				"Tool input type mismatch.\nExpected: {}\nReceived: {}.",
				expected_input,
				received_input,
			);
		} else if expected_output != received_output {
			bevybail!(
				"Tool output type mismatch.\nExpected: {}\nReceived: {}.",
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
			.ok_or_else(|| bevyhow!("ToolCall payload already taken. Are there multiple handlers on the same entity?"))
	}

	/// Call the on_out callback with the given output. This must only be done once.
	///
	/// # Errors
	///
	/// Errors if the [`on_out`] method has already been called.
	pub fn take_out_handler(&mut self) -> Result<ToolOutHandler<Out>> {
		self
			.out_handler.take()
			.ok_or_else(|| bevyhow!("ToolCall on_out already called. This may be a bug in the IntoToolHandler implementation."))
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
}

/// Extension trait for calling tools on entities.
#[extend::ext(name=EntityWorldMutToolExt)]
pub impl EntityWorldMut<'_> {
	/// Make a tool call and block until the result is ready.
	///
	/// ## Errors
	///
	/// Returns an error if the tool call fails or types don't match.
	fn call_blocking<
		In: 'static + Send + Sync + Typed,
		Out: 'static + Send + Sync + Typed,
	>(
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
	fn call<
		In: 'static + Send + Sync + Typed,
		Out: 'static + Send + Sync + Typed,
	>(
		mut self,
		input: In,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let (send, recv) = async_channel::bounded(1);
			let handler = ToolOutHandler::channel(send);
			trigger_for_entity(&mut self, input, handler)?;

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
					bevybail!("ToolCall response channel closed unexpectedly.")
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
	fn call<
		In: 'static + Send + Sync + Typed,
		Out: 'static + Send + Sync + Typed,
	>(
		&mut self,
		input: In,
		out_handler: ToolOutHandler<Out>,
	) {
		self.queue(|mut entity: EntityWorldMut| -> Result {
			trigger_for_entity(&mut entity, input, out_handler)
		});
	}
}

fn trigger_for_entity<In: Typed, Out: Typed>(
	entity: &mut EntityWorldMut,
	input: In,
	out_handler: ToolOutHandler<Out>,
) -> Result {
	let meta = entity.get::<ToolMeta>().ok_or_else(|| {
		bevyhow!("Entity does not have ToolMeta, cannot send tool call.")
	})?;
	meta.assert_match::<In, Out>()?;
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
		let mut world = ToolPlugin::world();
		world.add_observer(insert_tool_path_and_params);
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

		let mut world = ToolPlugin::world();
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
