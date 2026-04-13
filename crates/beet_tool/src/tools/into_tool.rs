use crate::prelude::*;

/// Conversion trait for creating a [`Tool`] from a value.
///
/// Implementations exist for plain closures
/// ([`Tool::new_pure`]), Bevy systems
/// ([`Tool::new_system`]), and async closures
/// ([`Tool::new_async`]).
pub trait IntoTool<M>: Sized {
	/// Input type for the resulting handler.
	type In;
	/// Output type for the resulting handler.
	type Out;
	/// Convert into a concrete [`Tool`].
	fn into_tool(self) -> Tool<Self::In, Self::Out>;
}

impl<In, Out> IntoTool<()> for Tool<In, Out>
where
	In: 'static,
	Out: 'static,
{
	type In = In;
	type Out = Out;

	fn into_tool(self) -> Tool<Self::In, Self::Out> { self }
}

pub trait DefaultTool<Input, Output> {
	fn default_tool() -> Tool<Input, Output>;
}
