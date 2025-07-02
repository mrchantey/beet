use super::*;
use crate::prelude::*;
use beet_template::prelude::*;
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
					// style roundtrip breaks without resolving templates,
					// im not sure if this should be here, doesnt it indicate
					// we're relying on exprs in templates?
					spawn_templates,
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
			),
		);
	}
}
