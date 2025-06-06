use crate::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use sweet::prelude::WorkspacePathBuf;

// pub fn get_app()-

/// A complete pipeline trip converting an [`rstml`] [`TokenStream`] into a
/// [`BundleTokens`].
/// This method uses the static [`TokensApp`] so its important that *all*
/// created entities are despawned after the conversion,
/// otherwise we get a TokenStream 'use after free' error.
///
pub fn tokenize_rstml_tokens(
	tokens: TokenStream,
	source_file: WorkspacePathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((SourceFile::new(source_file), RstmlTokens::new(tokens)))
			.id();
		app.update();
		let tokens = app
			.world_mut()
			.run_system_once_with(tokenize_bundle_children, entity)?;
		app.world_mut().entity_mut(entity).despawn();
		tokens
	})
}



pub fn rstml_to_token_tree(
	tokens: TokenStream,
	source_file: WorkspacePathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((SourceFile::new(source_file), RstmlTokens::new(tokens)))
			.id();
		app.update();
		let result = tokenize_node_tree(app.world(), entity);
		app.world_mut().entity_mut(entity).despawn();
		result
	})
}

// for tests see ../tokenize_bundle.rs
