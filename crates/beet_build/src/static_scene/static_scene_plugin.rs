use super::*;
use crate::prelude::*;
use beet_common::prelude::*;
use beet_fs::process::WatchEvent;
use beet_template::prelude::*;
use bevy::prelude::*;

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

		#[cfg(not(test))]
		app.add_systems(Startup, load_all_template_files);

		app.add_event::<WatchEvent>()
			// .init_resource::<WorkspaceConfig>()
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			// types
			.add_plugins(NodeTypesPlugin)
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
						.in_set(BeforeParseTokens),
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
						.in_set(AfterParseTokens),
					#[cfg(not(test))]
					export_template_scene.in_set(ExportArtifactsSet),
				)
			);
	}
}
