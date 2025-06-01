use bevy::prelude::Component;
use sweet::prelude::*;

/// A scene contaning all templates used in the project.
pub struct TemplateScene;


/// Used to reconcile [`BundleTokens`] with their corresponding template
/// entity in the [`TemplateScene`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component)]
pub struct TemplateKey {
	/// The source file containing the template.
	pub file: WorkspacePathBuf,
	/// The index of the template in the file.
	/// - For md and rsx files this is always 0 as they only have one template.
	/// - For rust files this is the top-down appearance of the `rsx!` macro.
	pub index: usize,
}
impl TemplateKey {
	/// Create a new [`TemplateKey`] from a file and index.
	pub fn new(file: WorkspacePathBuf, index: usize) -> Self {
		Self { file, index }
	}
}
