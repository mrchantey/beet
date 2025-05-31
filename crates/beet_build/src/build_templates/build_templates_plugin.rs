use crate::prelude::*;
use beet_parse::prelude::*;
use bevy::prelude::*;

/// Plugin containing all systems for building templates from files.
/// This plugin is usually added in combination with:
/// - [`NodeTokensPlugin`](beet_parse::prelude::NodeTokensPlugin)
#[derive(Debug, Default)]
pub struct BuildTemplatesPlugin;

impl Plugin for BuildTemplatesPlugin {
	fn build(&self, app: &mut App) {
		app
			// should already be initialized by [`BeetConfig`]
			.init_resource::<BuildTemplatesConfig>()
			.configure_sets(
				Update,
				(
					ImportTemplatesStep.before(ImportNodesStep),
					ExportTemplatesStep
						.after(ExportNodesStep)
						.after(ImportTemplatesStep),
				),
			)
			.add_systems(
				Startup,
				load_all_templates.in_set(ImportTemplatesStep),
			)
			.add_systems(
				Update,
				(
					templates_to_nodes_rs,
					templates_to_nodes_md,
					templates_to_nodes_rsx,
				)
					.in_set(ImportTemplatesStep),
			);
	}
}

/// Import template files into parsable formats like [`RstmlTokens`], or [`CombinatorToNodeTokens`].
/// - Before [`ImportNodesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ImportTemplatesStep;

/// Export parsed nodes to a template scene file.
/// - After [`ExportNodesStep`]
/// - After [`ImportTemplatesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportTemplatesStep;



/// Create a [`TemplateFile`] for each file specified in the [`BuildTemplatesConfig`].
fn load_all_templates(
	mut commands: Commands,
	config: Res<BuildTemplatesConfig>,
) -> Result<()> {
	config.get_files()?.into_iter().for_each(|path| {
		commands.spawn(TemplateFile::new(path));
	});

	Ok(())
}

#[allow(unused)]
fn clear_existing_templates(
	mut commands: Commands,
	query: Populated<&Templates, Changed<Templates>>,
) -> Result {
	for templates in query.iter() {
		for template in templates.iter() {
			commands.entity(template).despawn();
		}
	}
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
