use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use sweet::prelude::PipelineTarget;
use sweet::prelude::WorkspacePathBuf;

// pub fn get_app()-

/// A complete pipeline trip converting an [`rstml`] [`TokenStream`] into a
/// [`BundleTokens`].
/// This method uses the static [`TokensApp`] so its important that *all*
/// created entities are despawned after the conversion,
/// otherwise we get a TokenStream 'use after free' error.
///
pub fn rstml_to_bundle(
	tokens: TokenStream,
	source_file: WorkspacePathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				SourceFile::new(source_file),
				NodeTokensToBundle::default().exclude_errors(),
				RstmlTokens::new(tokens),
			))
			.id();
		app.update();
		let result = app
			.world_mut()
			.entity_mut(entity)
			.take::<BundleTokens>()
			.map(|tokens| tokens.take())
			.ok_or_else(|| {
				anyhow::anyhow!("Internal Error: Expected token stream")
			})?
			.xok();
		app.world_mut().entity_mut(entity).despawn();
		result
	})
}

// for tests see ../node_tokens_to_bundle.rs
