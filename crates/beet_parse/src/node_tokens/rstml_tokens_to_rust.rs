use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use sweet::prelude::PipelineTarget;
use sweet::prelude::WorkspacePathBuf;

// pub fn get_app()-

/// A complete pipeline trip converting an [`rstml`] [`TokenStream`] into a
/// [`Bundle`] [`TokenStream`]. This creates and destroys a [`Bevy`] app
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
				NodeTokensToRust::default().exclude_errors(),
			))
			.insert_non_send(RstmlTokens::new(tokens))
			.id();
		app.update();
		app.world_mut()
			.entity_mut(entity)
			.remove_non_send::<TokenStream>()?
			.ok_or_else(|| {
				anyhow::anyhow!("Internal Error: Expected token stream")
			})?
			.xok()
	// })
}

// for tests see ../node_tokens_to_rust.rs
