// use crate::prelude::*;
use beet_core::prelude::*;
// use bevy::ecs::system::BoxedSystem;
// use bevy::tasks::BoxedFuture;


pub struct Tool<In, Out> {
	handler: Box<dyn ToolHandler<In = In, Out = Out>>,
}

impl<In, Out> Tool<In, Out> {
	pub fn new<H>(handler: H) -> Self
	where
		H: ToolHandler<In = In, Out = Out> + 'static,
	{
		Self {
			handler: Box::new(handler),
		}
	}
	pub fn call(
		&mut self,
		commands: Commands,
		async_commands: AsyncCommands,
		call: ToolCall<In, Out>,
	) -> Result {
		self.handler.call(commands, async_commands, call)
	}
}



pub trait ToolHandler {
	type In;
	type Out;
	/// # Errors
	/// Errors if the [`OutHandler`] failed to handle the output
	fn call(
		&mut self,
		commands: Commands,
		async_commands: AsyncCommands,
		call: ToolCall<Self::In, Self::Out>,
	) -> Result;
}

impl<T> IntoToolHandler2<T> for T
where
	T: ToolHandler + 'static,
{
	type In = T::In;
	type Out = T::Out;
	fn into_handler(self) -> impl ToolHandler<In = Self::In, Out = Self::Out> {
		self
	}
}

pub trait IntoToolHandler2<M>: Sized {
	type In;
	type Out;
	fn into_handler(self) -> impl ToolHandler<In = Self::In, Out = Self::Out>;
}




pub struct ToolCall<In, Out> {
	/// The entity containing an observer to react
	/// to the tool call. This is the [`EntityEvent::event_target`].
	pub tool: Entity,
	/// The payload of the tool input.
	pub input: In,
	/// Called by the tool on done.
	pub out_handler: OutHandler<Out>,
}

impl<In, Out> ToolCall<In, Out> {
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

pub struct OutHandler<Out> {
	/// The callback to deliver the tool output to the caller.
	/// The result here should be from a failure to handle the output.
	func: Box<dyn 'static + Send + Sync + FnOnce(Out) -> Result>,
}
impl<Out> OutHandler<Out> {
	pub fn new<F>(f: F) -> Self
	where
		F: 'static + Send + Sync + FnOnce(Out) -> Result,
	{
		Self { func: Box::new(f) }
	}
	pub fn call(self, output: Out) -> Result { (self.func)(output) }
}
