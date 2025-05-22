/// Trait for scene templates.
pub trait Props {
	/// The builder used by.
	type Builder: PropsBuilder<Props = Self>;
	type Required;
}

// TODO From<Self::Component>
pub trait PropsBuilder: Default {
	type Props: Props<Builder = Self>;
	fn build(self) -> Self::Props;
}
