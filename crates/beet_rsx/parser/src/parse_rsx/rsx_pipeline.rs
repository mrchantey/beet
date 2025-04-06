use crate::prelude::*;
use proc_macro2::TokenStream;
use sweet::prelude::*;
use syn::spanned::Spanned;

pub struct ToProcMacro2;
impl<T: Into<TokenStream>> Pipeline<T, TokenStream> for ToProcMacro2 {
	fn apply(self, tokens: T) -> TokenStream { tokens.into() }
}
pub struct FromProcMacro2;
impl<T: From<TokenStream>> Pipeline<TokenStream, T> for FromProcMacro2 {
	fn apply(self, tokens: TokenStream) -> T { tokens.into() }
}
#[derive(Default)]
pub struct RsxMacroPipeline {
	pub no_errors: bool,
}
impl RsxMacroPipeline {
	pub fn no_errors() -> Self { Self { no_errors: true } }
}

impl<T: Into<TokenStream>> Pipeline<T, TokenStream> for RsxMacroPipeline {
	fn apply(self, tokens: T) -> TokenStream {
		let tokens = tokens.into();
		let span = tokens.span();
		let (rstml, rstml_errors) = tokens.xpipe(TokensToRstml::default());
		let (html, html_errors) = rstml.xpipe(RstmlToHtmlTokens::new());
		let tokens = html
			.xpipe(ApplyDefaultTemplateDirectives::default())
			.xpipe(HtmlTokensToRust::new_spanned(RsxIdents::default(), &span));

		if self.no_errors {
			tokens
		} else {
			quote::quote! {{
				#(#rstml_errors;)*
				#(#html_errors;)*
				#tokens
			}}
		}
	}
}
// /// Demonstrates how to select a different reactive runtime
// #[allow(unused_mut)]
// fn feature_flag_idents() -> RsxIdents {
// 	let mut idents = RsxIdents::default();
// 	#[cfg(feature = "sigfault")]
// 	{
// 		idents.runtime = RsxRuntime::sigfault();
// 	}
// 	idents
// }


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::quote;
	use sweet::prelude::*;
	#[test]
	fn directives() {
		expect(
			quote! {<div client:load/>}
				.xpipe(RsxMacroPipeline::no_errors())
				.to_string(),
		)
		//yes we now have client directives again!
		.to_be(
			quote! {{
					use beet::prelude::*;
					#[allow(unused_braces)]
					RsxElement {
				tag: "div".to_string(),
				attributes: vec![],
				children: Box::new(
						RsxFragment {
					nodes: vec![],
					meta: RsxNodeMeta::default(),
						}.into_node()
				),
				self_closing: true,
				meta: RsxNodeMeta {
						template_directives: vec![TemplateDirective::ClientLoad],
						location: None
				},
					}
					.into_node()
					.with_location(RsxMacroLocation::new(file!(), 1u32, 0u32))
			}}
			.to_string(),
		);
	}
}
