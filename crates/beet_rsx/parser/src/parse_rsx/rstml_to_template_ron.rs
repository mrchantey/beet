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

/// Convert rstml nodes to a ron file.
/// Rust block token streams will be hashed by [Span::start]
#[derive(Debug, Default, Clone)]
pub struct RstmlToRsxTemplateRon {}


impl RstmlToRsxTemplateRon {
	/// returns a "[RsxTemplateNode]" ron string
	pub fn map_tokens_to_string(&self, tokens: TokenStream) -> TokenStream {
		self.map_tokens(tokens)
			.to_string()
			.to_token_stream()
	}
	pub fn map_tokens(&self, tokens: TokenStream) -> TokenStream {
		let (nodes, _rstml_errors) = tokens_to_rstml(tokens);
		let nodes = self.map_nodes(nodes);
		quote! {[#(#nodes),*]}
	}
	/// comma separated RsxTemplateNode
	pub fn map_nodes<C>(&self, nodes: Vec<Node<C>>) -> Vec<TokenStream> {
		nodes.into_iter().map(|node| self.map_node(node)).collect()
	}

	/// comma sepereated RsxTemplateNode, due to fragments
	pub fn map_node<C>(&self, node: Node<C>) -> TokenStream {
		match node {
			Node::Doctype(_) => quote! {Doctype},
			Node::Comment(NodeComment { value, .. }) => {
				quote! {Comment(#value)}
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
				let hash = span_to_line_col_ron(&node_block.span());
				quote! {RustBlock(#hash)}
			}
			Node::Text(NodeText { value }) => {
				quote! {Text(#value)}
			}
			Node::RawText(raw) => {
				let val = raw.to_string_best();
				quote! {Text(#val)}
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
					let loc = span_to_line_col_ron(&span);
					quote! {
						Component (
							loc: #loc,
							tag: #tag_name,
							self_closing: #self_closing,
							attributes: [#(#attributes),*],
							children: [#(#children),*]
						)
					}
				} else {
					quote! {
							Element (
								tag: #tag_name,
								self_closing: #self_closing,
								attributes: [#(#attributes),*],
								children: [#(#children),*]
							)
					}
				}
			}
			Node::Custom(_) => unimplemented!(),
		}
	}
	fn map_attribute(&self, attr: NodeAttribute) -> TokenStream {
		match attr {
			NodeAttribute::Block(block) => {
				let hash = span_to_line_col_ron(&block.span());
				quote! {Block(#hash)}
			}
			NodeAttribute::Attribute(attr) => {
				let key = attr.key.to_string();
				match attr.value() {
					None => {
						quote! {Key ( key: #key )}
					}
					Some(syn::Expr::Lit(expr_lit)) => {
						let value = lit_to_string(&expr_lit.lit);
						quote! {
								KeyValue (
								key: #key,
								value: #value
								)
						}
					}
					Some(tokens) => {
						let hash = span_to_line_col_ron(&tokens.span());
						quote! {
							BlockValue (
								key: #key,
								value: #hash
							)
						}
					}
				}
			}
		}
	}
}
fn lit_to_string(lit: &syn::Lit) -> String {
	match lit {
		syn::Lit::Int(lit_int) => lit_int.base10_digits().to_string(),
		syn::Lit::Float(lit_float) => lit_float.base10_digits().to_string(),
		syn::Lit::Bool(lit_bool) => lit_bool.value.to_string(),
		syn::Lit::Str(lit_str) => lit_str.value(),
		syn::Lit::ByteStr(lit_byte_str) => {
			String::from_utf8_lossy(&lit_byte_str.value()).into_owned()
		}
		syn::Lit::Byte(lit_byte) => lit_byte.value().to_string(),
		syn::Lit::Char(lit_char) => lit_char.value().to_string(),
		syn::Lit::Verbatim(lit_verbatim) => lit_verbatim.to_string(),
		syn::Lit::CStr(_) => unimplemented!(),
		_ => unimplemented!(),
	}
}
