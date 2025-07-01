use super::*;
use crate::prelude::*;
use beet_common::node::HtmlConstants;
use beet_common::prelude::*;
use beet_fs::process::WatchEvent;
use beet_parse::prelude::ParseRsxTokensSet;
use beet_template::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


/// Config file usually located at `beet.toml`
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildConfig {
	#[serde(flatten)]
	pub template_config: TemplateConfig,
	pub route_codegen: RouteCodegenConfig,
	pub client_island_codegen: ClientIslandCodegenConfig,
}


/// Base plugin for beet_build
#[derive(Debug, Default)]
pub struct BuildPlugin;


#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct BeforeParseTokens;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct AfterParseTokens;
/// System set for exporting codegen files, Static Trees, Lang Partials, etc.
/// This set should be configured to run after all importing and processing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportArtifactsSet;



impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();


		app.add_event::<WatchEvent>()
			// .init_resource::<WorkspaceConfig>()
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			// types
			.add_plugins(NodeTypesPlugin)
			.configure_sets(
				Update,
				(
					BeforeParseTokens.before(ParseRsxTokensSet),
					AfterParseTokens
						.after(BeforeParseTokens)
						.after(ParseRsxTokensSet),
					ExportArtifactsSet
						.after(AfterParseTokens)
						.before(TemplateSet),
				),
			)
			.add_systems(
				Update,
				(
					touch_changed_source_files.before(BeforeParseTokens),
					update_file_expr_hash
						.after(ParseRsxTokensSet)
						.before(AfterParseTokens),
					export_codegen_files.in_set(ExportArtifactsSet),
				),
			);
	}
}
