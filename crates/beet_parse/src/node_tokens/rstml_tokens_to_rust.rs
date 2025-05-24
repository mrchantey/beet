use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use sweet::prelude::PipelineTarget;
use sweet::prelude::WorkspacePathBuf;

// pub fn get_app()-

/// A complete pipeline trip converting an [`rstml`] [`TokenStream`] into a
/// [`BundleTokens`]. This creates and destroys a [`Bevy`] app
/// each time so should not be used in hot paths.
pub fn rstml_tokens_to_rust(
	tokens: TokenStream,
	source_file: WorkspacePathBuf,
) -> Result<TokenStream> {
	let mut app = App::new();
	app.add_plugins(NodeTokensPlugin);

	// TokensApp::with(|app| {
	let entity = app
		.world_mut()
		.spawn((
			SourceFile::new(source_file),
			NodeTokensToBundle::default().exclude_errors(),
			RstmlTokens::new(tokens),
		))
		.id();
	app.update();
	app.world_mut()
		.entity_mut(entity)
		.take::<BundleTokens>()
		.map(|tokens| tokens.take())
		.ok_or_else(|| {
			anyhow::anyhow!("Internal Error: Expected token stream")
		})?
		.xok()
	// })
}

// for tests see ../node_tokens_to_rust.rs
