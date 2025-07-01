use super::*;
use beet_common::prelude::*;
use beet_parse::prelude::*;
use beet_template::prelude::*;
use bevy::prelude::*;

/// Import template files into parsable formats like [`RstmlTokens`], or [`CombinatorToNodeTokens`].
/// - Before [`ParseRsxTokensSet`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ImportTemplateStep;

/// Perform extra processing after nodes have been imported and processed.
/// - After [`ParseRsxTokensSet`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ProcessTemplateStep;

/// Export parsed nodes to a template scene file.
/// - After [`ProcessTemplateStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportTemplateStep;


/// Plugin containing all systems for exporting a scene including:
/// - [`LangPartial`]
/// - [`StaticNodeRoot`]
///  from files.
/// This plugin is usually added in combination with:
/// - [`NodeTokensPlugin`](beet_parse::prelude::NodeTokensPlugin)
#[derive(Debug, Default)]
pub struct StaticScenePlugin;


/// Idents used for template macros.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Resource)]
pub struct TemplateMacros {
	pub rstml: String,
}
impl Default for TemplateMacros {
	fn default() -> Self {
		Self {
			rstml: "rsx".to_string(),
		}
	}
}


impl Plugin for StaticScenePlugin {
	fn build(&self, app: &mut App) {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();

		app.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			// types
			.add_plugins(NodeTypesPlugin)
			.configure_sets(
				Update,
				(
					ImportTemplateStep.before(ParseRsxTokensSet),
					ProcessTemplateStep
						.after(ParseRsxTokensSet)
						.after(ImportTemplateStep),
					ExportTemplateStep
						.after(ProcessTemplateStep)
						// before all [`TemplatePlugin`] systems
						.before(SpawnStep),
				),
			)
			.add_systems(Startup, load_template_files)
			.add_systems(
				Update,
				(
					(
						// style roundtrip breaks without resolving templates,
						// im not sure if this should be here, doesnt it indicate
						// we're relying on exprs in templates?
						spawn_templates,
						(
							templates_to_nodes_rs,
							templates_to_nodes_md,
							templates_to_nodes_rsx,
						),
					)
						.chain()
						.in_set(ImportTemplateStep),
					(
						update_file_expr_hash,
						(
							extract_lang_partials,
							apply_style_ids,
							#[cfg(feature = "css")]
							parse_lightning,
						)
							.chain(),
					)
						.in_set(ProcessTemplateStep),
					#[cfg(not(test))]
					export_template_scene.in_set(ExportTemplateStep),
				),
			);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_template::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn load_all_templates() {
		App::new()
			.add_plugins(StaticScenePlugin)
			.insert_resource(StaticSceneConfig::test_site())
			.update_then()
			.world_mut()
			.xpect()
			.num_components::<TemplateFile>()
			.to_be_greater_than(10)
			.to_be_less_than(20);
	}
}
