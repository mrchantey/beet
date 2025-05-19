use crate::prelude::*;
use bevy::prelude::*;




/// Plugin containing all systems for building templates from files.
#[derive(Debug, Default)]
pub struct BuildTemplatesPlugin;

impl Plugin for BuildTemplatesPlugin {
	fn build(&self, app: &mut App) {
		app
			// should already be initialized by [`BeetConfig`]
			.init_resource::<BuildTemplatesConfig>()
			.add_systems(Startup, load_all_templates)
			.add_systems(Update, file_to_templates);
	}
}




fn load_all_templates(
	mut commands: Commands,
	config: Res<BuildTemplatesConfig>,
) -> Result<()> {
	config.get_files()?.into_iter().for_each(|path| {
		commands.spawn(TemplateFile::new(path));
	});

	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn load_all_templates() {
		App::new()
			.add_plugins(BuildTemplatesPlugin)
			.update_then()
			.world_mut()
			.xpect()
			.num_components::<TemplateFile>()
			.to_be_greater_than(10)
			.to_be_less_than(20);
	}
}
