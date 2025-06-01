use bevy::prelude::*;


/// Trait for template props.
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



/// An entity that was created as a non-slot child of a template,
/// which must be held as as seperate relationship to distinguish between
/// the entities created by a template function and inline/slot children.
#[derive(Component, Deref)]
#[relationship(relationship_target = TemplateRoot)]
pub struct TemplateOf(Entity);

impl TemplateOf {
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// The entity that was spawned by this template. This should
/// always only be a single entity, which may have [`Children`].
#[derive(Component)]
#[relationship_target(relationship = TemplateOf,linked_spawn)]
pub struct TemplateRoot(Vec<Entity>);

impl std::ops::Deref for TemplateRoot {
	type Target = Entity;
	fn deref(&self) -> &Self::Target { &self.0[0] }
}
