use crate::prelude::*;
use bevy::platform::hash::FixedHasher;
use bevy::prelude::*;



/// Created when a template file is loaded or changed.
#[derive(Debug, Clone, PartialEq, Eq, Component, Deref)]
pub struct TemplateFileHash(u64);

impl TemplateFileHash {
	pub fn new(hash: u64) -> Self { Self(hash) }

	pub fn hash(&self) -> u64 { self.0 }
}



pub fn apply_template_file_hash(
	mut commands: Commands,
	templates: Query<&TemplateRoots>,
	children: Query<&Children>,
	// hashable:Query<&ItemOf
	query: Populated<(Entity, &TemplateFile), Changed<TemplateRoots>>,
) {
	for (entity, template_file) in query.iter() {
		let mut hasher = FixedHasher::default();

		for template in templates.iter_descendants(entity) {
			todo!();
		}
	}
}
