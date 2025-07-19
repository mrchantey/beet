use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;

/// Parse rstml tokens into a *finalized* [`InstanceRoot`], see [`tokenize_bundle`].
pub fn tokenize_rstml(
	tokens: TokenStream,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				SnippetRoot::new_from_tokens(source_file, &tokens),
				InstanceRoot,
				RstmlTokens::new(tokens),
			))
			.id();
		app.update();
		let tokens = tokenize_bundle(app.world_mut(), entity);
		app.world_mut().entity_mut(entity).despawn();
		tokens
	})
}


/// Parse rstml tokens into a *tokenized* [`InstanceRoot`], see [`tokenize_bundle_tokens`].
pub fn tokenize_rstml_tokens(
	tokens: TokenStream,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				SnippetRoot::new_from_tokens(source_file, &tokens),
				InstanceRoot,
				RstmlTokens::new(tokens),
			))
			.id();
		app.update();
		let result = tokenize_bundle_tokens(app.world(), entity);
		app.world_mut().entity_mut(entity).despawn();
		result
	})
}

// for tests see ../tokenize_bundle.rs
