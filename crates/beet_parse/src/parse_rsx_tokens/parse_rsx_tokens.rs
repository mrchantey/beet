use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use proc_macro2::TokenStream;


/// A schedule for parsing raw rstml token streams and combinator strings into
/// rsx trees, then extracting directives.
/// This schedule is shared by macros and `beet_build` file parsing, enabling
/// consistency amongst the original source code and live reloading.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct ParseRsxTokens;

fn rstml(tokens: TokenStream, source_file: WsPathBuf) -> impl Bundle {
	(
		SnippetRoot::new_from_tokens(source_file, &tokens),
		InstanceRoot,
		RstmlTokens::new(tokens),
	)
}
fn combinator(tokens: &str, source_file: WsPathBuf) -> impl Bundle {
	(
		SnippetRoot::new(source_file, LineCol::default()),
		InstanceRoot,
		CombinatorTokens::new(tokens),
	)
}

impl ParseRsxTokens {
	/// Parse combinator string into a *finalized* [`InstanceRoot`], see [`tokenize_rsx`].
	pub fn combinator_to_rsx(
		tokens: &str,
		source_file: WsPathBuf,
	) -> Result<TokenStream> {
		Self::parse_bundle(
			combinator(tokens, source_file),
			tokenize_rsx_resolve_snippet,
		)
	}

	/// Parse combinator string into a *tokenized* [`InstanceRoot`], see [`tokenize_rsx_tokens`].
	pub fn combinator_to_rsx_tokens(
		tokens: &str,
		source_file: WsPathBuf,
	) -> Result<TokenStream> {
		Self::parse_bundle(combinator(tokens, source_file), tokenize_rsx_tokens)
	}

	/// Parse rstml tokens into a bsx representation, see [`tokenize_bsx_root`].
	pub fn rstml_to_bsx(
		tokens: TokenStream,
		source_file: WsPathBuf,
	) -> Result<TokenStream> {
		Self::parse_bundle(rstml(tokens, source_file), tokenize_bsx_root)
	}

	/// Parse rstml tokens into a *finalized* [`InstanceRoot`], see [`tokenize_rsx`].
	pub fn rstml_to_rsx(
		tokens: TokenStream,
		source_file: WsPathBuf,
	) -> Result<TokenStream> {
		Self::parse_bundle(
			rstml(tokens, source_file),
			tokenize_rsx_resolve_snippet,
		)
	}

	/// Parse rstml tokens into a *tokenized* [`InstanceRoot`], see [`tokenize_rsx_tokens`].
	pub fn rstml_to_rsx_tokens(
		tokens: TokenStream,
		source_file: WsPathBuf,
	) -> Result<TokenStream> {
		Self::parse_bundle(rstml(tokens, source_file), tokenize_rsx_tokens)
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

// for tests see ../tokenize_rsx.rs
