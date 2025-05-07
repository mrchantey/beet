use crate::prelude::*;
use beet_common::node::NodeSpan;
use proc_macro2::TokenStream;
use quote::ToTokens;
use sweet::prelude::*;
use syn::spanned::Spanned;

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
		let (html, html_errors) = rstml.xpipe(RstmlToWebTokens::new());
		let block = match html.xpipe(ParseWebTokens::default()) {
			Ok(val) => val.xpipe(WebTokensToRust::new_spanned(
				RsxIdents::default(),
				&span,
			)),
			Err(err) => {
				let err_str = err.to_string();
				return quote::quote! {
					compile_error!(#err_str);
				};
			}
		};
		if self.no_errors {
			block.to_token_stream()
		} else {
			quote::quote! {{
				#(#rstml_errors;)*
				#(#html_errors;)*
				#block
			}}
		}
	}
}

#[derive(Default)]
pub struct RsxTemplateMacroPipeline;


impl<T: Into<TokenStream>> Pipeline<T, TokenStream>
	for RsxTemplateMacroPipeline
{
	fn apply(self, value: T) -> TokenStream {
		value.xpipe(RsxRonPipeline::default()).xmap(|tokens| {
			let str_tokens = tokens.to_string();
			//TODO here we should embed errors like the rsx macro
			quote::quote! {RsxTemplateNode::from_ron(#str_tokens).unwrap()}
		})
	}
}

#[derive(Default)]
pub struct RsxRonPipeline<'a> {
	pub file: Option<&'a WorkspacePathBuf>,
}

impl<'a> RsxRonPipeline<'a> {
	pub fn new(file: &'a WorkspacePathBuf) -> Self { Self { file: Some(file) } }
}


impl<'a, T: Into<TokenStream>> Pipeline<T, TokenStream> for RsxRonPipeline<'a> {
	fn apply(self, tokens: T) -> TokenStream {
		let tokens = tokens.into();
		let span = tokens.span();
		tokens
			.xpipe(TokensToRstml::default())
			.0
			.xpipe(RstmlToWebTokens::new())
			.0
			.xpipe(ParseWebTokens::default())
			.map(|html| {
				let web_tokens_to_ron = if let Some(file) = self.file {
					WebTokensToRon::new(&NodeSpan::new_from_spanned(
						file.clone(),
						&span,
					))
				} else {
					WebTokensToRon::new_no_location()
				};


				html.xpipe(web_tokens_to_ron)
			})
			.unwrap_or_else(|err| {
				let err_str = err.to_string();
				quote::quote! {
					compile_error!(#err_str);
				}
			})
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
					meta: NodeMeta {
						template_directives: Vec::new(),
						location: None
				},
						}.into_node()
				),
				self_closing: true,
				meta: NodeMeta {
						template_directives: vec![TemplateDirective::ClientLoad],
						location: Some(NodeSpan::new(file!(), 1u32, 0u32))
				},
					}
					.into_node()
			}}
			.to_string(),
		);
	}
}
