use bevy::prelude::*;
use sweet::prelude::*;


/// A file specified by [`BuildTemplatesConfig`], to be parsed for its templates.
#[derive(Debug, Clone, PartialEq, Eq, Component, Deref)]
// #[component(immutable)]
pub struct TemplateFile {
	path: WorkspacePathBuf,
}

impl TemplateFile {
	pub fn new(path: WorkspacePathBuf) -> Self { Self { path } }
	pub fn path(&self) -> &WorkspacePathBuf { &self.path }
}


/// The entity containing the [`TemplateFile`] that this template belongs to.
#[derive(Component)]
#[relationship(relationship_target = TemplateRoots)]
pub struct TemplateFileSource(pub Entity);

/// The templates created from this entities [`TemplateFile`].
#[derive(Component, Deref)]
#[relationship_target(relationship = TemplateFileSource,linked_spawn)]
pub struct TemplateRoots(Vec<Entity>);
