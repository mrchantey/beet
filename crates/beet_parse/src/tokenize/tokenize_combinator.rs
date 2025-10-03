use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use proc_macro2::TokenStream;




/// Parse combinator string into a *finalized* [`InstanceRoot`], see [`tokenize_bundle`].
pub fn tokenize_combinator(
	tokens: &str,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	ParseRsxTokens::parse_and_run(
		(
			SnippetRoot::new(source_file, LineCol::default()),
			InstanceRoot,
			CombinatorTokens::new(tokens),
		),
		tokenize_bundle,
	)
}

/// Parse combinator string into a *tokenized* [`InstanceRoot`], see [`tokenize_bundle_tokens`].
pub fn tokenize_combinator_tokens(
	tokens: &str,
	source_file: WsPathBuf,
) -> Result<TokenStream> {
	ParseRsxTokens::parse_and_run(
		(
			SnippetRoot::new(source_file, LineCol::default()),
			InstanceRoot,
			CombinatorTokens::new(tokens),
		),
		tokenize_bundle_tokens,
	)
}
