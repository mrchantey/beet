use bevy::prelude::*;
use beet_utils::prelude::*;
use crate::prelude::*;

/// Adde to an entity used to represent an entire file specified by [`BuildTemplatesConfig`],
/// which may contain multiple templates.
#[derive(Debug, Clone, PartialEq, Eq, Component, Deref)]
// #[component(immutable)]
#[require(FileExprHash)]
pub struct TemplateFile {
	path: WsPathBuf,
}

impl TemplateFile {
	pub fn new(path: WsPathBuf) -> Self { Self { path } }
	pub fn path(&self) -> &WsPathBuf { &self.path }
}


/// The entity containing the [`TemplateFile`] that this template belongs to.
#[derive(Component)]
#[relationship(relationship_target = TemplateRoots)]
pub struct TemplateFileSource(pub Entity);

/// The templates created from this entities [`TemplateFile`]. Each of these
/// should have an associated [`TemplateRoot`]
#[derive(Component, Deref)]
#[relationship_target(relationship = TemplateFileSource,linked_spawn)]
pub struct TemplateRoots(Vec<Entity>);
