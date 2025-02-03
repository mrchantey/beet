use beet_rsx_parser::prelude::*;
use proc_macro::TokenStream;

/// this is quite unsophisticated at the moment, we can work on a nicer
/// way to expose it to library authors
struct ParseStrategy;

impl RsxRustTokens for ParseStrategy {
	fn ident() -> proc_macro2::TokenStream {
		#[cfg(feature = "signals")]
		return quote::quote! {beet::rsx::signals_rsx::SignalsRsx};
		#[cfg(not(feature = "signals"))]
		return quote::quote! {beet::rsx::string_rsx::StringRsx};
	}
}

/// This macro expands to an [RsxNode](beet_rsx::prelude::RsxNode).
///
/// The type of node is determied by the feature flags, current options are:
/// - [`StringRsx`](beet_rsx::rsx::StringRsx)
/// ```
/// # use beet::prelude::*;
/// let tree = rsx! {<div> the value is {3}</div>};
/// assert_eq!(tree.nodes.len(), 1);
///
/// ```
///
#[proc_macro]
pub fn rsx(tokens: TokenStream) -> TokenStream {
	let mut tokens: proc_macro2::TokenStream = tokens.into();
	let _output = RsxParser::<ParseStrategy>::default().parse_rsx(&mut tokens);
	// output is used by other parsers but for the macro
	// the errors are included in the token stream
	tokens.into()
}


/// This macro expands to a Vec<ReverseRsxNode>, it is used for
/// things like hot reloading.
#[proc_macro]
pub fn reverse_rsx(tokens: TokenStream) -> TokenStream {
	RstmlToReverseRsx::default()
		.map_tokens(tokens.into())
		.into()
}
