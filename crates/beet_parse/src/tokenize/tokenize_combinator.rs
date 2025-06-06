use crate::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use sweet::prelude::*;

pub fn tokenize_combinator_str(
	tokens: &str,
	source_file: WorkspacePathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				SourceFile::new(source_file),
				CombinatorTokens(tokens.to_string()),
			))
			.id();
		app.update();
		let tokens = app
			.world_mut()
			.run_system_once_with(tokenize_bundle_children, entity)?;
		app.world_mut().entity_mut(entity).despawn();
		tokens
	})
}
pub fn tokenize_combinator_tree(
	tokens: &str,
	source_file: WorkspacePathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				SourceFile::new(source_file),
				CombinatorTokens::new(tokens),
			))
			.id();
		app.update();
		let result = tokenize_node_tree(app.world(), entity);
		app.world_mut().entity_mut(entity).despawn();
		result
	})
}
