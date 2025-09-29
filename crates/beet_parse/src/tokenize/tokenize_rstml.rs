use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use proc_macro2::TokenStream;

/// Parse rstml tokens into a *finalized* [`InstanceRoot`], see [`tokenize_bundle`].
pub fn tokenize_rstml(
	tokens: TokenStream,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	ParseRsxTokens::parse_and_run(
		(
			SnippetRoot::new_from_tokens(source_file, &tokens),
			InstanceRoot,
			RstmlTokens::new(tokens),
		),
		tokenize_bundle,
	)
}


/// Parse rstml tokens into a *tokenized* [`InstanceRoot`], see [`tokenize_bundle_tokens`].
pub fn tokenize_rstml_tokens(
	tokens: TokenStream,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	ParseRsxTokens::parse_and_run(
		(
			SnippetRoot::new_from_tokens(source_file, &tokens),
			InstanceRoot,
			RstmlTokens::new(tokens),
		),
		tokenize_bundle_tokens,
	)
}

// for tests see ../tokenize_bundle.rs
