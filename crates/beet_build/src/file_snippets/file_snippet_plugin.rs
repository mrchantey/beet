use super::*;
use crate::prelude::*;
use bevy::prelude::*;

/// Plugin containing all systems for exporting a scene including:
/// - [`LangSnippet`]
/// - [`StaticNodeRoot`]
///  from files.
/// This plugin is usually added in combination with:
/// - [`NodeTokensPlugin`](beet_parse::prelude::NodeTokensPlugin)
#[derive(Debug, Default)]
pub struct FileSnippetPlugin;


impl Plugin for FileSnippetPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(
				(
					extract_lang_snippets,
					#[cfg(feature = "css")]
					parse_lightning,
				)
					.chain()
					.in_set(AfterParseTokens),
				(export_rsx_snippets, export_lang_snippets)
					.in_set(ExportArtifactsSet),
			),
		);
	}
}
