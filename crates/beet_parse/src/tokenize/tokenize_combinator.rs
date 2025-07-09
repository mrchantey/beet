use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;




/// Parse combinator string into a *finalized* [`InstanceRoot`], see [`tokenize_bundle`].
pub fn tokenize_combinator(
	tokens: &str,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				InstanceRoot,
				MacroIdx::new(source_file, LineCol::default()),
				CombinatorTokens::new(tokens),
			))
			.id();
		app.update();
		let tokens = tokenize_bundle(app.world_mut(), entity);
		app.world_mut().entity_mut(entity).despawn();
		tokens
	})
}

/// Parse combinator string into a *tokenized* [`InstanceRoot`], see [`tokenize_bundle_tokens`].
pub fn tokenize_combinator_tokens(
	tokens: &str,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				InstanceRoot,
				MacroIdx::new(source_file, LineCol::default()),
				CombinatorTokens::new(tokens),
			))
			.id();
		app.update();
		let result = tokenize_bundle_tokens(app.world(), entity);
		app.world_mut().entity_mut(entity).despawn();
		result
	})
}
