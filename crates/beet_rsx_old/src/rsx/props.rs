use crate::prelude::*;

/// Trait for using a [`Component`] as a node in the `rsx!` macro.
pub trait Props: IntoWebNode<()> {
	/// The builder used by.
	type Builder: PropsBuilder<Component = Self>;
	/// A helper struct of bools used by the `rsx!`
	/// macro to determine that all required fields are present.
	type Required;
}

// TODO From<Self::Component>
pub trait PropsBuilder: Default {
	type Component: IntoWebNode<()>;
	fn build(self) -> Self::Component;
}
