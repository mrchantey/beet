use crate::prelude::*;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use sweet::prelude::*;


#[derive(Default)]
pub struct RsxMacroPipeline {
	/// ideally we'd get this from proc_macro2::Span::source_file
	/// but thats not yet supported
	pub source_file: Option<WorkspacePathBuf>,
	pub no_errors: bool,
}
impl RsxMacroPipeline {
	pub fn new(source_file: Option<WorkspacePathBuf>) -> Self {
		Self {
			source_file,
			no_errors: false,
		}
	}
	pub fn no_errors(mut self) -> Self {
		self.no_errors = true;
		self
	}
}

impl<T: Into<TokenStream>> Pipeline<T, TokenStream> for RsxMacroPipeline {
	fn apply(self, tokens: T) -> TokenStream {
		let tokens: TokenStream = tokens.into();
		let span = self
			.source_file
			.map(|file| FileSpan::new_from_span(file, &tokens));
		let (rstml, rstml_errors) = tokens.xpipe(TokensToRstml::default());
		let (html, html_errors) = rstml.xpipe(RstmlToWebTokens::new(span));
		let block = match html.xpipe(ParseWebTokens::default()) {
			Ok(val) => val.xpipe(WebTokensToRust::default()),
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
pub struct RsxTemplateMacroPipeline {
	/// ideally we'd get this from proc_macro2::Span::source_file
	/// but thats not yet supported
	pub source_file: Option<WorkspacePathBuf>,
}
impl RsxTemplateMacroPipeline {
	pub fn new(source_file: Option<WorkspacePathBuf>) -> Self {
		Self { source_file }
	}
}

impl<T: Into<TokenStream>> Pipeline<T, TokenStream>
	for RsxTemplateMacroPipeline
{
	fn apply(self, value: T) -> TokenStream {
		let tokens: TokenStream = value.into();
		let span = self
			.source_file
			.map(|file| FileSpan::new_from_span(file, &tokens));

		tokens.xpipe(RsxRonPipeline::new(span)).xmap(|tokens| {
			let str_tokens = tokens.to_string();
			//TODO here we should embed errors like the rsx macro
			quote::quote! {RsxTemplateNode::from_ron(#str_tokens).unwrap()}
		})
	}
}

#[derive(Default)]
pub struct RsxRonPipeline {
	pub span: Option<FileSpan>,
}

impl RsxRonPipeline {
	pub fn new(span: Option<FileSpan>) -> Self { Self { span } }
}


impl<T: Into<TokenStream>> Pipeline<T, TokenStream> for RsxRonPipeline {
	fn apply(self, tokens: T) -> TokenStream {
		let tokens = tokens.into();
		tokens
			.xpipe(TokensToRstml::default())
			.0
			.xpipe(RstmlToWebTokens::new(self.span))
			.0
			.xpipe(ParseWebTokens::default())
			.map(|html| html.xpipe(WebTokensToRon::default()))
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
				.xpipe(
					RsxMacroPipeline::new(Some(WorkspacePathBuf::new(file!())))
						.no_errors(),
				)
				.to_string(),
		)
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
							template_directives: vec![],
							location: None
					},
							}.into_node()
					),
					self_closing: true,
					meta: NodeMeta {
							template_directives: vec![TemplateDirective::ClientLoad],
							location: Some (
								FileSpan::new(
									"ws_build/beet_rsx_parser/src/parse_rsx/rsx_pipeline.rs",
									LineCol::new(1, 0),
									LineCol::new(1, 0)
								)
							)
						},
					}
					.into_node()
			}}
			.to_string(),
		);
	}
}
