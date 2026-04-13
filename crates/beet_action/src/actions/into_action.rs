use crate::prelude::*;

/// Conversion trait for creating an [`Action`] from a value.
///
/// Implementations exist for plain closures
/// ([`Action::new_pure`]), Bevy systems
/// ([`Action::new_system`]), and async closures
/// ([`Action::new_async`]).
pub trait IntoAction<M>: Sized {
	/// Input type for the resulting handler.
	type In;
	/// Output type for the resulting handler.
	type Out;
	/// Convert into a concrete [`Action`].
	fn into_action(self) -> Action<Self::In, Self::Out>;
}

impl<In, Out> IntoAction<()> for Action<In, Out>
where
	In: 'static,
	Out: 'static,
{
	type In = In;
	type Out = Out;

	fn into_action(self) -> Action<Self::In, Self::Out> { self }
}

pub trait DefaultAction<Input, Output> {
	fn default_action() -> Action<Input, Output>;
}
