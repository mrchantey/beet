/// Trait for using a [`Component`] as a node in the `rsx!` macro.
pub trait Props {
	/// The builder used by.
	type Builder: PropsBuilder<Bundle = Self>;
	type Required;
}

// TODO From<Self::Component>
pub trait PropsBuilder: Default {
	type Bundle: Props<Builder = Self>;
	fn build(self) -> Self::Bundle;
}
