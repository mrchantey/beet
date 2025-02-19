use beet_rsx_parser::prelude::*;
use proc_macro::TokenStream;
mod derive_deref;

/// Demonstrates how to select a different reactive runtime
#[allow(unused_mut)]
fn feature_flag_idents() -> RsxIdents {
	let mut idents = RsxIdents::default();
	#[cfg(feature = "sigfault")]
	{
		idents.runtime = RsxRuntime::sigfault();
	}
	#[cfg(feature = "bevy")]
	{
		idents.runtime = RsxRuntime::bevy();
	}
	idents
}



/// This macro expands to an [RsxNode](beet_rsx::prelude::RsxNode).
///
/// The type of node is determied by the feature flags, current options are:
/// - [`StringRsx`](beet_rsx::rsx::StringRsx)
/// ```ignore
/// let tree = rsx! {<div> the value is {3}</div>};
/// ```
///
#[proc_macro]
pub fn rsx(tokens: TokenStream) -> TokenStream {
	RstmlToRsx {
		// perhaps we can feature gate this if it proves expensive
		idents: feature_flag_idents(),
		..Default::default()
	}
	.map_tokens(tokens.into())
	.into()
}

/// Mostly used for testing,
/// this macro expands to an RsxTemplateNode, it is used for
/// things like hot reloading.
#[proc_macro]
pub fn rsx_template(tokens: TokenStream) -> TokenStream {
	RstmlToRsxTemplate::default()
		.from_macro(tokens.into())
		.into()
}




#[proc_macro_derive(Deref)]
pub fn derive_deref(input: TokenStream) -> TokenStream {
	derive_deref::derive_deref(input)
}

#[proc_macro_derive(DerefMut)]
pub fn derive_deref_mut(input: TokenStream) -> TokenStream {
	derive_deref::derive_deref_mut(input)
}
