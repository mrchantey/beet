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


impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<WatchEvent>()
			.configure_sets(
				Update,
				ExportArtifactsSet
					.after(ParseRsxTokensSet)
					.before(TemplateSet),
			)
			.add_systems(
				Update,
				export_codegen_files.in_set(ExportArtifactsSet),
			);
	}
}
