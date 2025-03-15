use crate::prelude::*;

/// Trait for using a [`Component`] as a node in the `rsx!` macro.
pub trait Props: Component {
	/// The builder used by.
	type Builder: PropsBuilder<Props = Self>;
	/// A helper struct of bools used by the `rsx!`
	/// macro to determine that all required fields are present.
	type Required;
}


pub trait PropsBuilder: Default {
	type Props: Props;
	fn build(self) -> Self::Props;
}
