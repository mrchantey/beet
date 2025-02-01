use proc_macro::TokenStream;

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
pub fn rsx(tokens: TokenStream) -> TokenStream { RsxMacro::parse(tokens) }

use beet_rsx_parser::prelude::*;

struct ParseStrategy;

impl RsxRustTokens for ParseStrategy {
	fn ident() -> proc_macro2::TokenStream {
		#[cfg(feature = "signals")]
		return quote::quote! {beet::rsx::signals_rsx::SignalsRsx};
		#[cfg(not(feature = "signals"))]
		return quote::quote! {beet::rsx::string_rsx::StringRsx};
	}
}


struct RsxMacro;




impl RsxMacro {
	pub fn parse(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
		let mut tokens: proc_macro2::TokenStream = tokens.into();
		let _output =
			RsxParser::<ParseStrategy>::default().parse_rsx(&mut tokens);
		// ignore output because errors are included in the token stream

		tokens.into()
	}
}
