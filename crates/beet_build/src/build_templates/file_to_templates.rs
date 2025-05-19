use crate::prelude::*;
use bevy::prelude::*;



pub fn file_to_templates(
	mut commands: Commands,
	query: Query<(&TemplateFile, Option<&Templates>), Changed<TemplateFile>>,
) {
	for (file, templates) in query.iter() {
		if let Some(templates) = templates {
			for template in templates.iter() {
				commands.entity(template).despawn();
			}
		}

		match file.extension() {
			Some(_) => todo!(),
			None => todo!(),
		}
	}
}
