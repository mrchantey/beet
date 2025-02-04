use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeComment;
use rstml::node::NodeElement;
use rstml::node::NodeText;
use syn::spanned::Spanned;

/// Convert rstml nodes to serializable html nodes.
/// Rust block token streams will be hashed by [Span::start]
#[derive(Debug, Default, Clone)]
pub struct RstmlToRsxTemplate {}


impl RstmlToRsxTemplate {
	/// returns a Vec<HtmlNode>
	pub fn map_tokens(&self, tokens: TokenStream) -> TokenStream {
		let (nodes, rstml_errors) = tokens_to_rstml(tokens);
		let nodes = self.map_nodes(nodes);
		quote! {
			{
				#(#rstml_errors;)*
				use beet::prelude::*;
				vec![#(#nodes),*]
			}
		}
	}
	/// comma separated RsxTemplateNode
	pub fn map_nodes<C>(&self, nodes: Vec<Node<C>>) -> Vec<TokenStream> {
		nodes.into_iter().map(|node| self.map_node(node)).collect()
	}

	/// comma sepereated RsxTemplateNode, due to fragments
	pub fn map_node<C>(&self, node: Node<C>) -> TokenStream {
		match node {
			Node::Doctype(_) => quote! {RsxTemplateNode::Doctype},
			Node::Comment(NodeComment { value, .. }) => {
				quote! {RsxTemplateNode::Comment(#value.to_string())}
			}
			Node::Fragment(node_fragment) => {
				let children = node_fragment
					.children
					.into_iter()
					.map(|n| self.map_node(n));
				quote! {
					#(#children),*
				}
			}
			Node::Block(node_block) => {
				let hash = span_to_line_col(&node_block.span());
				quote! {RsxTemplateNode::RustBlock(#hash)}
			}
			Node::Text(NodeText { value }) => {
				quote! {RsxTemplateNode::Text(#value.to_string())}
			}
			Node::RawText(raw) => {
				let val = raw.to_string_best();
				quote! {RsxTemplateNode::Text(#val.to_string())}
			}
			Node::Element(NodeElement {
				open_tag,
				children,
				close_tag,
			}) => {
				let span = open_tag.span();
				let tag_name = open_tag.name.to_string();
				let self_closing = close_tag.is_none();
				let attributes = open_tag
					.attributes
					.into_iter()
					.map(|a| self.map_attribute(a));
				let children = self.map_nodes(children);

				let is_component = tag_name
					.chars()
					.next()
					.map(char::is_uppercase)
					.unwrap_or(false);
				if is_component {
					// components disregard all the context, they are known
					// to the rsx node
					let hash = span_to_line_col(&span);
					quote! {
						RsxTemplateNode::Component {
							hash: #hash,
							tag: #tag_name.to_string(),
							self_closing: #self_closing,
							attributes: vec![#(#attributes),*],
							children: vec![#(#children),*],
						}
					}
				} else {
					quote! {
							RsxTemplateNode::Element {
								tag: #tag_name.to_string(),
								self_closing: #self_closing,
								attributes: vec![#(#attributes),*],
								children: vec![#(#children),*],
						}
					}
				}
			}
			Node::Custom(_) => unimplemented!(),
		}
	}
	fn map_attribute(&self, attr: NodeAttribute) -> TokenStream {
		match attr {
			NodeAttribute::Block(block) => {
				let hash = span_to_line_col(&block.span());
				quote! {RsxTemplateAttribute::Block(#hash)}
			}
			NodeAttribute::Attribute(attr) => {
				let key = attr.key.to_string();
				match attr.value() {
					None => {
						quote! {RsxTemplateAttribute::Key { key: #key.to_string() }}
					}
					Some(syn::Expr::Lit(expr_lit)) => {
						let value = match &expr_lit.lit {
							syn::Lit::Str(s) => s.to_token_stream(),
							other => other.to_token_stream(),
						};
						quote! {
								RsxTemplateAttribute::KeyValue {
								key: #key.to_string(),
								value: #value.to_string(),
							}
						}
					}
					Some(tokens) => {
						let hash = span_to_line_col(&tokens.span());
						quote! {
							RsxTemplateAttribute::BlockValue {
								key: #key.to_string(),
								value: #hash,
							}
						}
					}
				}
			}
		}
	}
}
