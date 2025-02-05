use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeComment;
use rstml::node::NodeElement;
use rstml::node::NodeText;

/// Convert rstml nodes to serializable html nodes.
/// Rust block token streams will be hashed by [Span::start]
#[derive(Debug, Default)]
pub struct RstmlToRsxTemplate {
	tracker: RustyTrackerBuilder,
}


impl RstmlToRsxTemplate {
	/// returns a Vec<HtmlNode>
	pub fn map_tokens(&mut self, tokens: TokenStream) -> TokenStream {
		let (nodes, rstml_errors) = tokens_to_rstml(tokens);
		let mut nodes = self.map_nodes(nodes);

		let node = if nodes.len() == 1 {
			nodes.pop().unwrap()
		} else {
			quote! {RsxTemplateNode::Fragment(vec![#(#nodes),*])}
		};
		quote! {
			{
				#(#rstml_errors;)*
				use beet::prelude::*;
				#node
			}
		}
	}
	/// comma separated RsxTemplateNode
	pub fn map_nodes<C>(&mut self, nodes: Vec<Node<C>>) -> Vec<TokenStream> {
		nodes.into_iter().map(|node| self.map_node(node)).collect()
	}

	/// comma sepereated RsxTemplateNode, due to fragments
	pub fn map_node<C>(&mut self, node: Node<C>) -> TokenStream {
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
					RsxTemplateNode::Fragment(vec![#(#children),*])
				}
			}
			Node::Block(node_block) => {
				let tracker = self.tracker.next_tracker(&node_block);
				quote! {RsxTemplateNode::RustBlock(#tracker)}
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
				let tag_name = open_tag.name.to_string();
				let self_closing = close_tag.is_none();

				let is_component = tag_name
					.chars()
					.next()
					.map(char::is_uppercase)
					.unwrap_or(false);
				if is_component {
					// components disregard all the context, they are known
					// to the rsx node
					let tracker = self.tracker.next_tracker(open_tag);
					// we rely on the hydrated node to provide the attributes and children
					quote! {
						RsxTemplateNode::Component {
							tracker: #tracker,
							tag: #tag_name.to_string(),
						}
					}
				} else {
					let children = self.map_nodes(children);
					let attributes = open_tag
						.attributes
						.into_iter()
						.map(|a| self.map_attribute(a));
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
	fn map_attribute(&mut self, attr: NodeAttribute) -> TokenStream {
		match attr {
			NodeAttribute::Block(block) => {
				let tracker = self.tracker.next_tracker(&block);
				quote! {RsxTemplateAttribute::Block(#tracker)}
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
						let tracker = self.tracker.next_tracker(&tokens);
						quote! {
							RsxTemplateAttribute::BlockValue {
								key: #key.to_string(),
								tracker: #tracker,
							}
						}
					}
				}
			}
		}
	}
}
