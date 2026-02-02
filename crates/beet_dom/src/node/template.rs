//! Template traits and types for component-like patterns.
//!
//! This module provides traits for defining template props and builders,
//! enabling a component-like pattern similar to web frameworks.

use beet_core::prelude::*;


/// Trait for template props that can be constructed via a builder.
pub trait Props {
	/// The builder type used to construct these props.
	type Builder: PropsBuilder<Props = Self>;
	/// Marker type for required props.
	type Required;
}

/// Trait for building template props.
pub trait PropsBuilder: Default {
	/// The props type this builder produces.
	type Props: Props<Builder = Self>;
	/// Builds the props from the builder state.
	fn build(self) -> Self::Props;
}



/// A root level entity built from a template contructor, with this as a
/// a non-children relationship to distinguish it from inline/slot children.
#[derive(Component, Deref)]
#[relationship(relationship_target = TemplateRoot)]
pub struct TemplateOf(Entity);

impl TemplateOf {
	/// Creates a new template relationship pointing to the given entity.
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// Assigned to the 'container' entity of a template, pointing to the bundle
/// spawned by the template. This should always only be a single entity,
/// which may have [`Children`]. We use this relation so that the parent-child
/// relation can easily be traversed in `apply_template_children`.
/// This relationship will be replaced with a parent-child relationship
/// in apply_slots.
// TODO 1:1 relationship
#[derive(Component)]
#[relationship_target(relationship = TemplateOf,linked_spawn)]
pub struct TemplateRoot(Vec<Entity>);


impl std::ops::Deref for TemplateRoot {
	type Target = Entity;
	fn deref(&self) -> &Self::Target { &self.0[0] }
}
