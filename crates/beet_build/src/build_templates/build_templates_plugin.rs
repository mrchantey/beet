use super::*;
use beet_common::prelude::*;
use beet_parse::prelude::*;
use bevy::prelude::*;

/// Plugin containing all systems for building templates from files.
/// This plugin is usually added in combination with:
/// - [`NodeTokensPlugin`](beet_parse::prelude::NodeTokensPlugin)
#[derive(Debug, Default)]
pub struct BuildTemplatesPlugin;

impl Plugin for BuildTemplatesPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<HtmlConstants>()
			// types
			.add_plugins((
				node_types_plugin,
				directive_types_plugin,
				template_types_plugin,
			))
			.configure_sets(
				Update,
				(
					ImportTemplateStep.before(ImportNodesStep),
					ProcessTemplateStep
						.after(ExportNodesStep)
						.after(ImportTemplateStep),
					ExportTemplateStep.after(ProcessNodesStep),
				),
			)
			.add_systems(
				Update,
				(
					(
						load_template_files,
						(
							templates_to_nodes_rs,
							templates_to_nodes_md,
							templates_to_nodes_rsx,
						),
					)
						.chain()
						.in_set(ImportTemplateStep),
					(extract_lang_partials, apply_style_ids, parse_lightning)
						.chain()
						.in_set(ProcessTemplateStep),
					export_template_scene.in_set(ExportTemplateStep),
				),
			);
	}
}

/// Import template files into parsable formats like [`RstmlTokens`], or [`CombinatorToNodeTokens`].
/// - Before [`ImportNodesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ImportTemplateStep;

/// Perform extra processing after nodes have been imported and processed.
/// - After [`ExportNodesStep`]
/// - After [`ImportTemplatesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ProcessTemplateStep;

/// Export parsed nodes to a template scene file.
/// - After [`ProcessTemplatesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportTemplateStep;




#[allow(unused)]
fn clear_existing_templates(
	mut commands: Commands,
	query: Populated<&TemplateFileTemplates, Changed<TemplateFileTemplates>>,
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
			.add_plugins((BeetConfig::default(), BuildTemplatesPlugin))
			.update_then()
			.world_mut()
			.xpect()
			.num_components::<TemplateFile>()
			.to_be_greater_than(10)
			.to_be_less_than(20);
	}
}
