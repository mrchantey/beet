use crate::prelude::*;
use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use rstml::node::CustomNode;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeComment;
use rstml::node::NodeElement;
use rstml::node::NodeFragment;
use rstml::node::NodeText;
use sweet::prelude::*;
use syn::spanned::Spanned;

use super::meta_builder::MetaBuilder;

/// Convert rstml nodes to a ron file.
/// Rust block token streams will be hashed by [Span::start]
#[derive(Debug)]
pub struct RstmlToRsxTemplate {
	rusty_tracker: RustyTrackerBuilder,
	/// root location of the macro, this will be taken by the first node
	root_location: Option<TokenStream>,
}


impl RstmlToRsxTemplate {
	/// for use with rsx_template! macro, which is usually just used for
	/// tests, routers use [RstmlToRsxTemplate::map_tokens]
	pub fn from_macro(tokens: TokenStream) -> TokenStream {
		let str_tokens =
			Self::map_tokens(tokens, None).to_string().to_token_stream();
		quote! {
			RsxTemplateNode::from_ron(#str_tokens).unwrap()
		}
	}
	/// The entry point for parsing the content of an rsx! macro
	/// into a serializable RON format.
	pub fn map_tokens(
		tokens: TokenStream,
		file: Option<&WorkspacePathBuf>,
	) -> TokenStream {
		let root_location = file.map(|file| {
			let span = tokens.span();
			let file = file.to_string_lossy();
			let line = Literal::usize_unsuffixed(span.start().line);
			let col = Literal::usize_unsuffixed(span.start().column);

			quote! { Some(RsxMacroLocation(
				file: (#file),
				line: #line,
				col: #col
			))}
		});

		let mut this = Self {
			rusty_tracker: RustyTrackerBuilder::default(),
			root_location,
		};
		let (rstml_nodes, _rstml_errors) = tokens_to_rstml(tokens);
		this.map_nodes(rstml_nodes)
	}


	/// the first to call this gets the real location, this mirrors
	/// `RstmlToRsx` behavior, only root has location.
	fn location(&mut self) -> TokenStream {
		std::mem::take(&mut self.root_location).unwrap_or(quote! {None})
	}

	/// meta without template directives
	fn basic_meta(&mut self) -> TokenStream {
		MetaBuilder::build_ron(self.location())
	}

	/// wraps in fragment if multiple nodes
	pub fn map_nodes<C: CustomNode>(
		&mut self,
		nodes: Vec<Node<C>>,
	) -> TokenStream {
		// if itll be a fragment, we need to take the location before
		// visiting
		let root_fragment_meta = if nodes.len() != 1 {
			Some(self.basic_meta())
		} else {
			None
		};
		let mut nodes = nodes
			.into_iter()
			.map(|node| self.map_node(node))
			.collect::<Vec<_>>();
		if nodes.len() == 1 {
			nodes.pop().unwrap().to_token_stream()
		} else {
			let meta = root_fragment_meta.unwrap_or_else(|| self.basic_meta());
			quote! { Fragment (
				items: [#(#nodes),*],
				meta: #meta
			)}
		}
	}

	/// returns an RsxTemplateNode
	pub fn map_node<C: CustomNode>(&mut self, node: Node<C>) -> TokenStream {
		match node {
			Node::Doctype(_) => {
				let meta = self.basic_meta();
				quote! { Doctype (
					meta: #meta
				)}
			}
			Node::Comment(NodeComment { value, .. }) => {
				let meta = self.basic_meta();
				quote! { Comment (
					value: #value,
					meta: #meta
				)}
			}
			Node::Text(NodeText { value }) => {
				let meta = self.basic_meta();
				quote! { Text (
					value: #value,
					meta: #meta
				)}
			}
			Node::RawText(raw) => {
				let meta = self.basic_meta();
				let value = raw.to_string_best();
				quote! { Text (
					value: #value,
					meta: #meta
				)}
			}
			Node::Fragment(NodeFragment { children, .. }) => {
				let meta = self.basic_meta();
				let children = children.into_iter().map(|n| self.map_node(n));
				quote! { Fragment (
					items:[#(#children),*],
					meta: #meta
				)}
			}
			Node::Block(block) => {
				let meta = self.basic_meta();
				let tracker = self.rusty_tracker.next_tracker_ron(&block);
				quote! { RustBlock (
					tracker:#tracker,
					meta: #meta
				)}
			}
			Node::Element(NodeElement {
				open_tag,
				children,
				close_tag,
			}) => {
				let location = self.location();
				let (directives, attributes) =
					MetaBuilder::parse_attributes(&open_tag.attributes);


				let meta = MetaBuilder::build_ron_with_directives(
					location,
					&directives,
				);

				let tag = open_tag.name.to_string();
				let self_closing = close_tag.is_none();
				if tag.starts_with(|c: char| c.is_uppercase()) {
					let tracker =
						self.rusty_tracker.next_tracker_ron(&open_tag);
					// components disregard all the context and rely on the tracker
					// we rely on the hydrated node to provide the attributes and children
					let slot_children = self.map_nodes(children);

					quote! { Component (
						tracker: #tracker,
						tag: #tag,
						slot_children: #slot_children,
						meta: #meta
					)}
				} else {
					let attributes = attributes
						.into_iter()
						.map(|a| self.map_attribute(a))
						.collect::<Vec<_>>();
					let children = self.map_nodes(children);
					quote! { Element (
						tag: #tag,
						self_closing: #self_closing,
						attributes: [#(#attributes),*],
						children: #children,
						meta: #meta
					)}
				}
			}
			Node::Custom(_) => unimplemented!(),
		}
	}

	fn map_attribute(&mut self, attr: &NodeAttribute) -> TokenStream {
		match attr {
			NodeAttribute::Block(block) => {
				let tracker = self.rusty_tracker.next_tracker_ron(&block);
				quote! { Block (#tracker)}
			}
			NodeAttribute::Attribute(attr) => {
				let key = attr.key.to_string();
				match attr.value() {
					None => {
						quote! {Key ( key: #key )}
					}
					Some(syn::Expr::Lit(expr_lit)) => {
						let value = lit_to_string(&expr_lit.lit);
						quote! { KeyValue (
						key: #key,
						value: #value
						)}
					}
					Some(value) => {
						let tracker =
							self.rusty_tracker.next_tracker_ron(&value);
						// we dont need to handle events for serialization,
						// thats an rstml_to_rsx concern so having the tracker is enough
						quote! { BlockValue (
							key: #key,
							tracker: #tracker
						)}
					}
				}
			}
		}
	}
}
pub fn lit_to_string(lit: &syn::Lit) -> String {
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
