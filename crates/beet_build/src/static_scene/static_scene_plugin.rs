use super::*;
use crate::prelude::*;
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


impl Plugin for StaticScenePlugin {
	fn build(&self, app: &mut App) {
		#[cfg(not(test))]
		app.add_systems(Startup, load_all_template_files);
		app
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
						extract_lang_partials,
						apply_style_ids,
						#[cfg(feature = "css")]
						parse_lightning,
					)
						.chain()
						.in_set(AfterParseTokens),
					#[cfg(not(test))]
					export_template_scene.in_set(ExportArtifactsSet),
				),
			);
	}
}
