use crate::prelude::*;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use sweet::prelude::*;

#[derive(Default)]
pub struct RsxMacroPipeline {
	pub source_file: WorkspacePathBuf,
	pub no_errors: bool,
}
impl RsxMacroPipeline {
	pub fn new(source_file: WorkspacePathBuf) -> Self {
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
		let (rstml, rstml_errors) = tokens.xpipe(TokensToRstml::default());
		let (html, html_errors) =
			rstml.xpipe(RstmlToWebTokens::new(self.source_file));
		let block = match html.xpipe(ParseWebTokens::default()) {
			Ok(val) => val.xpipe(WebTokensToRust::default()),
			Err(err) => {
				let err_str = err.to_string();
				return quote! {
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
	pub source_file: WorkspacePathBuf,
}
impl RsxTemplateMacroPipeline {
	pub fn new(source_file: WorkspacePathBuf) -> Self { Self { source_file } }
}

impl<T: Into<TokenStream>> Pipeline<T, TokenStream>
	for RsxTemplateMacroPipeline
{
	fn apply(self, value: T) -> TokenStream {
		let tokens: TokenStream = value.into();

		tokens
			// .xpipe(RsxRonPipeline::new(span))
			.xpipe(TokensToRstml::default())
			.0
			.xpipe(RstmlToWebTokens::new(self.source_file))
			.0
			.xpipe(ParseWebTokens::default())
			.map(|tokens| {
				tokens
					.xpipe(WebTokensToTemplate::default())
					.xmap(|template| {
						let str_tokens = ron::ser::to_string(&template)?;
						//TODO here we should embed errors like the rsx macro
						quote! {WebNodeTemplate::from_ron(#str_tokens).unwrap()}
							.xok()
					})
			})
			.flatten()
			.xmap(|result| match result {
				Ok(tokens) => tokens,
				Err(err) => {
					let err_str = err.to_string();
					quote! {
						compile_error!(#err_str);
					}
				}
			})
	}
}


#[derive(Default)]
pub struct WebTokensPipeline {
	/// ideally we'd get this from proc_macro2::Span::source_file
	/// but thats not yet supported
	pub source_file: WorkspacePathBuf,
}
impl WebTokensPipeline {
	pub fn new(source_file: WorkspacePathBuf) -> Self { Self { source_file } }
}

impl<T: Into<TokenStream>> Pipeline<T, TokenStream> for WebTokensPipeline {
	fn apply(self, value: T) -> TokenStream {
		let tokens: TokenStream = value.into();

		tokens
			.xpipe(TokensToRstml::default())
			.0
			.xpipe(RstmlToWebTokens::new(self.source_file))
			.0
			.xpipe(ParseWebTokens::default())
			.map(|tokens| {
				let tokens = tokens.into_rust_tokens();
				quote! {{
					use beet::prelude::*;
					#tokens
				}}
			})
			.unwrap_or_else(|err| {
				let err_str = err.to_string();
				quote! {
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
					RsxMacroPipeline::new(WorkspacePathBuf::new(file!()))
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
						meta: NodeMeta::new(
							FileSpan::new("crates/beet_rsx_parser/src/parse_rsx/rsx_pipeline.rs",
								LineCol::new(1, 0),
								LineCol::new(1, 0))
								, vec![]
							),
							}.into_node()
					),
					self_closing: true,
					meta: NodeMeta::new(
						FileSpan::new("crates/beet_rsx_parser/src/parse_rsx/rsx_pipeline.rs",
							LineCol::new(1, 0),
							LineCol::new(1, 0))
							, vec![
								TemplateDirective::NodeTemplate,
								TemplateDirective::ClientLoad
							]
						),
					}
					.into_node()
			}}
			.to_string(),
		);
	}
}
