use crate::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;


fn wrap_in_fragment(tokens: &str) -> String {
	if tokens.trim().starts_with("<>") {
		tokens.to_string()
	} else {
		format!("<>{}</>", tokens)
	}
}


/// Parse combinator string into a *finalized* [`Bundle`], see [`tokenize_bundle`].
pub fn tokenize_combinator(
	tokens: &str,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	let tokens = wrap_in_fragment(tokens);
	println!("Tokenizing combinator: {}", tokens);
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((SourceFile::new(source_file), CombinatorTokens(tokens)))
			.id();
		app.update();
		let tokens = tokenize_bundle(app.world_mut(), entity);
		app.world_mut().entity_mut(entity).despawn();
		tokens
	})
}

/// Parse combinator string into a *tokenized* [`Bundle`], see [`tokenize_bundle_tokens`].
pub fn tokenize_combinator_tokens(
	tokens: &str,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	let tokens = wrap_in_fragment(tokens);
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				SourceFile::new(source_file),
				CombinatorTokens::new(tokens),
			))
			.id();
		app.update();
		let result = tokenize_bundle_tokens(app.world(), entity);
		app.world_mut().entity_mut(entity).despawn();
		result
	})
}
