use beet_core::prelude::*;


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



/// A root level entity built from a template contructor, with this as a
/// a non-children relationship to distinguish it from inline/slot children.
#[derive(Component, Deref)]
#[relationship(relationship_target = TemplateRoot)]
pub struct TemplateOf(Entity);

impl TemplateOf {
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// Assigned to the 'container' entity of a template, pointing to the bundle
/// spawned by the template.
/// This relationship will be replaced with a parent-child relationship
/// in apply_slots.
/// Points to the entity that was spawned by this template. This should
/// always only be a single entity, which may have [`Children`].
// TODO 1:1 relationship
#[derive(Component)]
#[relationship_target(relationship = TemplateOf,linked_spawn)]
pub struct TemplateRoot(Vec<Entity>);


impl std::ops::Deref for TemplateRoot {
	type Target = Entity;
	fn deref(&self) -> &Self::Target { &self.0[0] }
}
