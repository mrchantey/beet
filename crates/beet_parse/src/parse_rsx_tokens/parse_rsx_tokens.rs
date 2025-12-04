use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use proc_macro2::TokenStream;


/// A sequence for parsing raw rstml token streams and combinator strings into
/// rsx trees, then extracting directives.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct ParseRsxTokens;

impl ParseRsxTokens {
	/// Parse combinator string into a *finalized* [`InstanceRoot`], see [`tokenize_bundle`].
	pub fn parse_combinator(
		tokens: &str,
		source_file: WsPathBuf,
	) -> Result<TokenStream> {
		ParseRsxTokens::parse_bundle(
			(
				SnippetRoot::new(source_file, LineCol::default()),
				InstanceRoot,
				CombinatorTokens::new(tokens),
			),
			tokenize_bundle_resolve_snippet,
		)
	}

	/// Parse combinator string into a *tokenized* [`InstanceRoot`], see [`tokenize_bundle_tokens`].
	pub fn parse_combinator_tokens(
		tokens: &str,
		source_file: WsPathBuf,
	) -> Result<TokenStream> {
		ParseRsxTokens::parse_bundle(
			(
				SnippetRoot::new(source_file, LineCol::default()),
				InstanceRoot,
				CombinatorTokens::new(tokens),
			),
			tokenize_bundle_tokens,
		)
	}

	/// Parse rstml tokens into a *finalized* [`InstanceRoot`], see [`tokenize_bundle`].
	pub fn parse_rstml(
		tokens: TokenStream,
		source_file: WsPathBuf,
	) -> Result<TokenStream> {
		ParseRsxTokens::parse_bundle(
			(
				SnippetRoot::new_from_tokens(source_file, &tokens),
				InstanceRoot,
				RstmlTokens::new(tokens),
			),
			tokenize_bundle_resolve_snippet,
		)
	}

	/// Parse rstml tokens into a *tokenized* [`InstanceRoot`], see [`tokenize_bundle_tokens`].
	pub fn parse_rstml_tokens(
		tokens: TokenStream,
		source_file: WsPathBuf,
	) -> Result<TokenStream> {
		ParseRsxTokens::parse_bundle(
			(
				SnippetRoot::new_from_tokens(source_file, &tokens),
				InstanceRoot,
				RstmlTokens::new(tokens),
			),
			tokenize_bundle_tokens,
		)
	}

	/// Spawn the bundle, run the function with it, then return the result.
	fn parse_bundle(
		bundle: impl Bundle,
		func: impl FnOnce(&World, Entity) -> Result<TokenStream>,
	) -> Result<TokenStream> {
		// TODO cost 100us creating an app per macro, we should cache thread
		// local app, wait for BeetMain pattern
		let mut app = App::new();
		app.add_plugins(ParseRsxTokensPlugin);
		let world = app.world_mut();
		let entity = world.spawn(bundle).id();
		world.run_schedule(ParseRsxTokens);
		let tokens = func(world, entity)?;

		let imports = dom_imports();

		quote::quote! {{
			#imports
			#tokens
		}}
		.xok()
	}
}

// for tests see ../tokenize_bundle.rs
