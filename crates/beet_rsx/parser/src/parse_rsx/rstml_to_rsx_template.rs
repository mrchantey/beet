use crate::prelude::*;
use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use rstml::atoms::OpenTag;
use rstml::node::CustomNode;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeComment;
use rstml::node::NodeElement;
use rstml::node::NodeFragment;
use rstml::node::NodeText;
use sweet::prelude::*;
use syn::spanned::Spanned;

/// Convert rstml nodes to a ron file.
/// Rust block token streams will be hashed by [Span::start]
#[derive(Debug)]
pub struct RstmlToRsxTemplate {
	rusty_tracker: RustyTrackerBuilder,
	/// this will be taken by the first node
	location: Option<TokenStream>,
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
		let location_tokens = file.map(|file| {
			let span = tokens.span();
			let file = file.to_string_lossy();
			let line = Literal::usize_unsuffixed(span.start().line);
			let col = Literal::usize_unsuffixed(span.start().column);

			quote! {Some(RsxMacroLocation(
				file: (#file),
				line: #line,
				col: #col
			))}
		});

		let mut this = Self {
			rusty_tracker: RustyTrackerBuilder::default(),
			location: location_tokens,
		};
		let (rstml_nodes, _rstml_errors) = tokens_to_rstml(tokens);
		let nodes = this.map_nodes(rstml_nodes);
		nodes
		// quote! {
		// 	RsxTemplateRoot (
		// 		location: #location,
		// 		node: #node
		// 	)
		// }
	}

	fn get_location(&mut self) -> TokenStream {
		std::mem::take(&mut self.location).unwrap_or(quote! {None})
	}

	/// wraps in fragment if multiple nodes
	pub fn map_nodes<C: CustomNode>(
		&mut self,
		nodes: Vec<Node<C>>,
	) -> TokenStream {
		// if itll be a fragment, we need to take the location before
		// visiting
		let location = if nodes.len() != 1 {
			Some(self.get_location())
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
			let location = location.unwrap_or_else(|| self.get_location());
			quote! { Fragment (
				items: [#(#nodes),*],
				location: #location
			)}
		}
	}

	/// returns an RsxTemplateNode
	pub fn map_node<C: CustomNode>(&mut self, node: Node<C>) -> TokenStream {
		let location = self.get_location();
		match node {
			Node::Doctype(_) => quote! { Doctype (
			)},
			Node::Comment(NodeComment { value, .. }) => {
				quote! { Comment (
					value: #value,
					location: #location
				)}
			}
			Node::Text(NodeText { value }) => {
				quote! { Text (
					value: #value,
					location: #location
				)}
			}
			Node::RawText(raw) => {
				let value = raw.to_string_best();
				quote! { Text (
					value: #value,
					location: #location
				)}
			}
			Node::Fragment(NodeFragment { children, .. }) => {
				let children = children.into_iter().map(|n| self.map_node(n));
				quote! { Fragment (
					items:[#(#children),*],
					location: #location
				)}
			}
			Node::Block(block) => {
				let tracker = self.rusty_tracker.next_tracker_ron(&block);
				quote! { RustBlock (
					tracker:#tracker,
					location: #location
				)}
			}
			Node::Element(NodeElement {
				open_tag,
				children,
				close_tag,
			}) => {
				let tag = open_tag.name.to_string();
				let self_closing = close_tag.is_none();
				if tag.starts_with(|c: char| c.is_uppercase()) {
					self.map_component(tag, open_tag, children)
				} else {
					let attributes = open_tag
						.attributes
						.into_iter()
						.map(|a| self.map_attribute(a))
						.collect::<Vec<_>>();
					let children = self.map_nodes(children);
					quote! { Element (
						tag: #tag,
						self_closing: #self_closing,
						attributes: [#(#attributes),*],
						children: #children,
						location: #location
					)}
				}
			}
			Node::Custom(_) => unimplemented!(),
		}
	}

	fn map_attribute(&mut self, attr: NodeAttribute) -> TokenStream {
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
	fn map_component<C: CustomNode>(
		&mut self,
		tag: String,
		open_tag: OpenTag,
		children: Vec<Node<C>>,
	) -> TokenStream {
		let location = self.get_location();

		let tracker = self.rusty_tracker.next_tracker_ron(&open_tag);
		// components disregard all the context and rely on the tracker
		// we rely on the hydrated node to provide the attributes and children
		let slot_children = self.map_nodes(children);

		let template_directives = open_tag
			.attributes
			.into_iter()
			.filter_map(|a| match a {
				NodeAttribute::Attribute(attr) => {
					let key = &attr.key.to_string();
					if key.contains(":") {
						let value = match attr.value() {
							Some(expr) => {
								quote! {Some(#expr)}
							}
							None => quote! {None},
						};
						let mut parts = key.split(':');
						let prefix = parts.next().expect(
							"expected colon prefix in template directive",
						);
						let suffix = parts.next().expect(
							"expected colon suffix in template directive",
						);

						Some(quote! {TemplateDirective (
							prefix: #prefix,
							suffix: #suffix,
							value: #value
						)})
					} else {
						None
					}
				}
				_ => None,
			})
			.collect::<Vec<_>>();

		quote! { Component (
			tracker: #tracker,
			tag: #tag,
			slot_children: #slot_children,
			template_directives: [#(#template_directives),*],
			location: #location
		)}
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
