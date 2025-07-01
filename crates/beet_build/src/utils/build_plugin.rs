use super::*;
use beet_fs::process::WatchEvent;
use beet_parse::prelude::ParseRsxTokensSet;
use beet_template::prelude::*;
use bevy::prelude::*;

/// System set for exporting codegen files, Static Trees, Lang Partials, etc.
/// This set should be configured to run after all importing and processing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportArtifactsSet;

/// Base plugin for beet_build
#[derive(Debug, Default)]
pub struct BuildPlugin;


#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct BeforeParseTokens;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct AfterParseTokens;



impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<WatchEvent>()
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
					export_codegen_files.in_set(ExportArtifactsSet),
				),
			);
	}
}
