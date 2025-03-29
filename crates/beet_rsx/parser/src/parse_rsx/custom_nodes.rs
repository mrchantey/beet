// use crate::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::ToTokens;
use rstml::atoms::CloseTag;
use rstml::atoms::CloseTagStart;
use rstml::atoms::OpenTag;
use rstml::node::CustomNode;
use rstml::node::Node;
use rstml::node::NodeElement;
// use rstml::node::RawText;
use rstml::recoverable::ParseRecoverable;
use rstml::recoverable::RecoverableContext;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::spanned::Spanned;

#[derive(Debug)]
struct StyleNode {
	node: Node,
}

type Inner = Node;

impl StyleNode {
	pub fn parse_children(
		parser: &mut RecoverableContext,
		input: ParseStream,
		open_tag: &OpenTag,
	) -> Option<(Vec<Inner>, Option<CloseTag>)> {
		let (children, close_tag) = {
			// If node is not raw use any closing tag as separator, to early report about
			// invalid closing tags.
			// Also parse only </ part to recover parser as soon as user types </
			let (children, close_tag) = parser
				.parse_tokens_until_call::<Inner, _, _>(
					input,
					CloseTagStart::parse,
				);

			let close_tag =
				CloseTag::parse_with_start_tag(parser, input, close_tag);

			(children, close_tag)
		};

		// let open_tag_end = open_tag.end_tag.token_gt.span();
		// let close_tag_start =
		// 	close_tag.as_ref().map(|c| c.start_tag.token_lt.span());
		// let children =
		// 	RawText::vec_set_context(open_tag_end, close_tag_start, children);

		let Some(close_tag) = close_tag else {
			let mut diagnostic = Diagnostic::spanned(
				open_tag.span(),
				Level::Error,
				"open tag has no corresponding close tag",
			);
			if !children.is_empty() {
				let mut note_span = TokenStream::new();
				children.iter().for_each(|v| v.to_tokens(&mut note_span));
				diagnostic = diagnostic.span_note(
					note_span.span(),
					"treating all inputs after open tag as it content",
				);
			}

			parser.push_diagnostic(diagnostic);
			return Some((children, None));
		};

		if close_tag.name != open_tag.name {
			let diagnostic = Diagnostic::spanned(
				close_tag.span(),
				Level::Error,
				"wrong close tag found",
			)
			.spanned_child(
				open_tag.span(),
				Level::Help,
				"open tag that should be closed; it's started here",
			);

			parser.push_diagnostic(diagnostic)
		}
		Some((children, Some(close_tag)))
	}
}

impl ToTokens for StyleNode {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		self.node.to_tokens(tokens)
	}
}


impl ParseRecoverable for StyleNode {
	fn parse_recoverable(
		parser: &mut RecoverableContext,
		input: ParseStream,
	) -> Option<Self> {
		let open_tag: OpenTag = parser.parse_recoverable(input)?;

		// let tag_name_str = &*open_tag.name.to_string();
		let (children, close_tag) =
			Self::parse_children(parser, input, &open_tag)?;

		// i guess do something here?

		// children = Vec::default();
		let element = NodeElement {
			open_tag,
			children,
			close_tag,
		};
		Some(Self {
			node: Node::Element(element),
		})
	}
}

impl CustomNode for StyleNode {
	fn peek_element(input: ParseStream) -> bool {
		// Peek for < token
		if !input.peek(Token![<]) {
			return false;
		}

		// Fork the stream to look ahead without consuming
		let fork = input.fork();

		// Try to parse < token
		if fork.parse::<Token![<]>().is_err() {
			return false;
		}

		// Check if "style" identifier follows
		fork.peek(syn::Ident)
			&& fork.parse::<syn::Ident>().map_or(false, |id| id == "style")
	}
}

#[cfg(test)]
mod test {
	use quote::quote;
	use rstml::Parser;
	use rstml::ParserConfig;

	use super::StyleNode;

	#[test]
	fn works() {
		let tokens = quote! {
			<div>
				<h1>{hello}</h1>
				<p>world</p>
				<style>"
					div{
						padding: 1em;
					};
				"</style>
			</div>
		};

		let config = ParserConfig::new()
			.recover_block(true)
			.raw_text_elements(["script", "style"].into_iter().collect())
			.custom_node::<StyleNode>()
			.transform_block(|block| {
				println!("block: {:?}", block);
				Ok(None)
			});

		let parser = Parser::<StyleNode>::new(config);
		let _ = parser.parse_recoverable(tokens).split_vec();
	}
}
