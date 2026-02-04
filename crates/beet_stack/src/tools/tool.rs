use beet_core::exports::async_channel;
use beet_core::exports::async_channel::Sender;
use beet_core::exports::async_channel::TryRecvError;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;


/// A tool handler, like a function, has an input and output.
/// This trait creates the handler, which must listen for a
/// [`ToolIn`] event, and at some point trigger a [`ToolOut`]
///
pub trait IntoToolHandler<M>: 'static + Send + Sync + Clone {
	/// The type of the input payload for the tool call.
	type In: Typed + 'static + Send + Sync;
	/// The type of the output payload for the tool call.
	type Out: Typed;
	/// Create the tool handler, this must be an Observer.
	fn into_handler(self) -> impl Bundle;
}

/// Marker component for function tool handlers.
pub struct FunctionIntoToolHandlerMarker;


impl<Func, In, Out> IntoToolHandler<(FunctionIntoToolHandlerMarker, In, Out)>
	for Func
where
	Func: 'static + Send + Sync + Clone + Fn(In) -> Out,
	In: Typed + 'static + Send + Sync,
	Out: Typed,
{
	type In = In;
	type Out = Out;

	fn into_handler(self) -> impl Bundle {
		OnSpawn::observe(
			move |mut ev: On<ToolCall<Self::In, Self::Out>>| -> Result {
				let ev = ev.event_mut();
				let payload = ev.take_payload()?;
				let output = self.clone()(payload);
				ev.call_on_out(output)?;
				Ok(())
			},
		)
	}
}

/// Create a tool from a handler implementing [`IntoToolHandler`],
/// adding an associated [`ToolMeta`] which is required for tool calling.
pub fn tool<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
{
	(
		ToolMeta {
			input: H::In::type_info(),
			output: H::Out::type_info(),
		},
		handler.into_handler(),
	)
}

/// Metadata for a tool, containing the input and output types.
/// Every tool must have a [`ToolMeta`], calling a tool with
/// types that dont match will result in an error.
#[derive(Component)]
pub struct ToolMeta {
	/// The reflected type information for the tool input.
	input: &'static TypeInfo,
	/// The reflected type information for the tool output.
	output: &'static TypeInfo,
}

impl ToolMeta {
	pub fn input(&self) -> &'static TypeInfo { self.input }
	pub fn output(&self) -> &'static TypeInfo { self.output }

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
pub struct ToolCall<In = (), Out = ()> {
	/// The entity containing an observer to react
	/// to the tool call. This is the [`EntityEvent::event_target`].
	#[event_target]
	tool: Entity,
	/// The payload of the tool input, which may only be consumed once
	payload: Option<In>,
	/// Called by the tool on done. This must only be called once.
	on_out: Option<Box<dyn 'static + Send + Sync + FnOnce(Out) -> Result>>,
}

impl<In, Out> ToolCall<In, Out> {
	/// Create a new [`ToolIn`] event with the given caller, tool, and payload.
	pub fn new(
		tool: Entity,
		payload: In,
		on_out: impl 'static + Send + Sync + FnOnce(Out) -> Result,
	) -> Self {
		Self {
			tool,
			payload: Some(payload),
			on_out: Some(Box::new(on_out)),
		}
	}
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
	pub fn call_on_out(&mut self, output: Out) -> Result {
		let on_out = self
			.on_out.take()
			.ok_or_else(|| bevyhow!("ToolCall on_out already called. This may be a bug in the IntoToolHandler implementation."))?;
		on_out(output)
	}
}

pub enum ToolOutHandler<Out> {
	/// The tool was called by another entity which
	/// does not need to track individual calls,
	/// and instead will listen for a [`ToolOut`]
	/// event.
	Observer(Entity),
	/// The tool caller is listening on a channel
	Channel(Sender<Out>),
}


/// Extension trait for calling tools on entities.
#[extend::ext(name=EntityWorldMutToolExt)]
pub impl EntityWorldMut<'_> {
	fn send_blocking<
		In: 'static + Send + Sync + Typed,
		Out: 'static + Send + Sync + Typed,
	>(
		&mut self,
		input: In,
	) -> Result<Out> {
		async_ext::block_on(self.send(input))
	}
	/// Triggers an entity target event for this entity.
	fn send<
		In: 'static + Send + Sync + Typed,
		Out: 'static + Send + Sync + Typed,
	>(
		&mut self,
		input: In,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let meta = self.get::<ToolMeta>().ok_or_else(|| {
				bevyhow!(
					"Entity does not have ToolMeta, cannot send tool call."
				)
			})?;
			meta.assert_match::<In, Out>()?;

			let entity = self.id();
			// SAFETY: While it is possible to change the entity location,
			// we no longer use the EntityWorldMut.
			let world = unsafe { self.world_mut() };
			let (send, recv) = async_channel::bounded(1);
			world.trigger(ToolCall::new(entity, input, move |output| {
				send.try_send(output)?.xok()
			}));

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



#[cfg(test)]
mod test {
	use super::*;

	fn add_tool((a, b): (i32, i32)) -> i32 { a + b }

	#[test]
	fn works() {
		World::new()
			.spawn(tool(add_tool))
			.send_blocking::<(i32, i32), i32>((2, 2))
			.unwrap()
			.xpect_eq(4);
	}
	#[test]
	#[should_panic = "Input mismatch"]
	fn input_mismatch() {
		World::new()
			.spawn(tool(add_tool))
			.send_blocking::<bool, i32>(true)
			.unwrap();
	}
	#[test]
	#[should_panic = "Output mismatch"]
	fn output_mismatch() {
		World::new()
			.spawn(tool(add_tool))
			.send_blocking::<(i32, i32), bool>((2, 2))
			.unwrap();
	}
}
