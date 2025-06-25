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
