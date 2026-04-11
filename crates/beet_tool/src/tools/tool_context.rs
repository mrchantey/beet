use beet_core::prelude::*;

/// Unified context for all tool handlers, providing access to the
/// caller entity and the input payload. The `caller` field is always an
/// [`AsyncEntity`], giving tools consistent access to the entity that
/// initiated the call regardless of whether the tool is sync or async.
#[derive(Debug, Clone, Deref, DerefMut, Get)]
pub struct ToolContext<In = ()> {
	/// The entity that initiated this tool call.
	pub caller: AsyncEntity,
	/// The input payload for this tool call.
	#[deref]
	pub input: In,
}

impl<In> ToolContext<In> {
	/// Returns the [`Entity`] id of the caller.
	pub fn id(&self) -> Entity { self.caller.id() }

	/// Consume the context and return the inner input payload.
	pub fn take(self) -> In { self.input }

	pub fn world(&self) -> AsyncWorld { self.caller.world().clone() }

	/// Map the input to a different type, keeping the same caller.
	pub fn map_input<NewIn>(self, input: NewIn) -> ToolContext<NewIn> {
		ToolContext {
			caller: self.caller,
			input,
		}
	}
}
