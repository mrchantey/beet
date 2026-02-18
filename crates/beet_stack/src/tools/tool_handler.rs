//! Shared types for tool handler context extraction and output conversion.
//!
//! This module provides the context types ([`ToolContext`],
//! [`AsyncToolContext`]) and the conversion traits
//! ([`FromToolContext`], [`FromAsyncToolContext`], [`IntoToolOutput`])
//! used by all tool handler implementations.
use beet_core::prelude::*;


/// Context passed to tool handlers containing the tool entity and input payload.
pub struct ToolContext<In = ()> {
	/// The tool entity being called.
	pub tool: Entity,
	/// The input payload for this tool call.
	pub input: In,
}

impl<In> std::ops::Deref for ToolContext<In> {
	type Target = In;
	fn deref(&self) -> &Self::Target { &self.input }
}

impl<In> ToolContext<In> {
	/// Create a new tool context with the given tool and payload.
	pub fn new(tool: Entity, input: In) -> Self { Self { tool, input } }

	/// Consume the context and return the inner input payload.
	pub fn take(self) -> In { self.input }
}

/// Convert from a [`ToolContext`] into a tool handler parameter.
///
/// This has a blanket impl restricted to [`Reflect`] types to avoid
/// collision with concrete impls like [`ToolContext`] itself.
///
/// ## Example
///
/// ```rust
/// # use beet_stack::prelude::*;
///
/// struct MyPayload;
///
/// impl FromToolContext<MyPayload, Self> for MyPayload {
///		fn from_tool_context(ctx: ToolContext<MyPayload>) -> Self { ctx.input }
/// }
/// ```
///
// TODO this should be much easier with negative impls https://doc.rust-lang.org/beta/unstable-book/language-features/negative-impls.html
pub trait FromToolContext<In, M> {
	/// Convert the tool context into this type.
	fn from_tool_context(ctx: ToolContext<In>) -> Self;
}

/// Marker type for extracting just the payload from a [`ToolContext`].
pub struct PayloadFromToolContextMarker;

impl<In> FromToolContext<In, PayloadFromToolContextMarker> for In
where
	// as ToolContext is not Reflect we avoid multiple impls
	In: bevy::reflect::Typed,
{
	fn from_tool_context(ctx: ToolContext<In>) -> Self { ctx.input }
}

impl<In> FromToolContext<In, Self> for ToolContext<In> {
	fn from_tool_context(ctx: ToolContext<In>) -> Self { ctx }
}

impl FromToolContext<Request, Self> for Request {
	fn from_tool_context(ctx: ToolContext<Request>) -> Self { ctx.input }
}

/// Async context passed to async tool handlers, providing an
/// [`AsyncEntity`] handle for non-blocking ECS access.
pub struct AsyncToolContext<In = ()> {
	/// The async tool entity being called.
	pub tool: AsyncEntity,
	/// The input payload for this tool call.
	pub input: In,
}

impl<In> std::ops::Deref for AsyncToolContext<In> {
	type Target = In;
	fn deref(&self) -> &Self::Target { &self.input }
}

impl<In> AsyncToolContext<In> {
	/// Create a new async tool context.
	pub fn new(tool: AsyncEntity, input: In) -> Self { Self { tool, input } }
}

/// Convert from an [`AsyncToolContext`] into an async tool handler parameter.
pub trait FromAsyncToolContext<In, M> {
	/// Convert the async tool context into this type.
	fn from_async_tool_context(ctx: AsyncToolContext<In>) -> Self;
}

/// Marker type for extracting the payload from an [`AsyncToolContext`].
pub struct PayloadFromAsyncToolContextMarker;

impl<In> FromAsyncToolContext<In, Self> for AsyncToolContext<In> {
	fn from_async_tool_context(ctx: AsyncToolContext<In>) -> Self { ctx }
}

impl<T, In, M> FromAsyncToolContext<In, (In, M)> for T
where
	T: FromToolContext<In, M>,
{
	fn from_async_tool_context(cx: AsyncToolContext<In>) -> Self {
		T::from_tool_context(ToolContext {
			tool: cx.tool.id(),
			input: cx.input,
		})
	}
}


/// Trait for converting tool handler outputs into the final output type.
///
/// This handles the conversion at the output level to avoid Bevy's
/// `IntoSystem` ambiguity where `Result<T, BevyError>` could resolve
/// as either identity or unwrap.
pub trait IntoToolOutput<Out, M> {
	/// Convert this type into a tool output result.
	fn into_tool_output(self) -> Result<Out>;
}

/// Marker for converting [`Result<T>`] into tool output.
pub struct ResultIntoToolOutput;
impl<Out> IntoToolOutput<Out, ResultIntoToolOutput> for Result<Out> {
	fn into_tool_output(self) -> Result<Out> { self }
}

/// Marker for converting any [`Reflect`] value directly into tool output.
pub struct TypeIntoToolOutput;
impl<Out> IntoToolOutput<Out, TypeIntoToolOutput> for Out
where
	Out: bevy::reflect::Typed,
{
	fn into_tool_output(self) -> Result<Out> { self.xok() }
}

impl IntoToolOutput<Self, Self> for Response {
	fn into_tool_output(self) -> Result<Self> { Ok(self) }
}
