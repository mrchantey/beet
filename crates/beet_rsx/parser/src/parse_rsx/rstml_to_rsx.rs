use crate::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::quote;
use quote::ToTokens;
use rapidhash::RapidHashSet as HashSet;
use rstml::atoms::OpenTag;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeElement;
use rstml::node::NodeFragment;
use rstml::node::NodeName;
use syn::spanned::Spanned;

/// given a span, for example the inner block
/// of an rsx! or rsx_template! macro,
/// return a RsxLocation token stream
pub fn rsx_location_tokens(tokens: impl Spanned) -> TokenStream {
	let span = tokens.span();
	let line = span.start().line;
	let col = span.start().column;
	quote! {
		{
			RsxLocation::new(std::file!(), #line, #col)
		}
	}
}

/// Convert rstml nodes to a Vec<RsxNode> token stream
#[derive(Debug, Default)]
pub struct RstmlToRsx {
	pub idents: RsxIdents,
	// Additional error and warning messages.
	pub errors: Vec<TokenStream>,
	// Collect elements to provide semantic highlight based on element tag.
	// No differences between open tag and closed tag.
	// Also multiple tags with same name can be present,
	// because we need to mark each of them.
	pub collected_elements: Vec<NodeName>,
	pub self_closing_elements: HashSet<&'static str>,
	pub rusty_tracker: RustyTrackerBuilder,
}

impl RstmlToRsx {
	pub fn new(idents: RsxIdents) -> Self {
		Self {
			idents,
			..Default::default()
		}
	}

	pub fn map_tokens(&mut self, tokens: TokenStream) -> TokenStream {
		let (nodes, rstml_errors) = tokens_to_rstml(tokens.clone());
		let node = self.map_nodes(nodes);

		let location = rsx_location_tokens(tokens);
		let rstml_to_rsx_errors = &self.errors;
		quote! {
			{
				#(#rstml_errors;)*
				#(#rstml_to_rsx_errors;)*
				use beet::prelude::*;
				#[allow(unused_braces)]
				RsxRoot{
					location: #location,
					node: #node
				}

			}
		}
	}
	/// the number of actual html nodes will likely be different
	/// due to fragments, blocks etc
	pub fn map_nodes<C>(&mut self, nodes: Vec<Node<C>>) -> TokenStream {
		let mut nodes = nodes
			.into_iter()
			.map(|node| self.map_node(node))
			.collect::<Vec<_>>();
		if nodes.len() == 1 {
			nodes.pop().unwrap().to_token_stream()
		} else {
			quote!(RsxNode::Fragment(Vec::from([#(#nodes),*])))
		}
	}

	/// returns an RsxNode
	fn map_node<C>(&mut self, node: Node<C>) -> TokenStream {
		match node {
			Node::Doctype(_) => quote!(RsxNode::Doctype),
			Node::Comment(comment) => {
				let comment = comment.value.value();
				quote!(RsxNode::Comment(#comment.to_string()))
			}
			Node::Text(text) => {
				let text = text.value_string();
				quote!(RsxNode::Text(#text.to_string()))
			}
			Node::RawText(raw) => {
				let text = raw.to_string_best();
				quote!(RsxNode::Text(#text.to_string()))
			}
			Node::Block(block) => {
				let tracker = self.rusty_tracker.next_tracker(&block);
				let ident = &self.idents.effect;
				// block is a {block} so assign to a value to unwrap
				quote! {
					{
						let value = #block;
						RsxNode::Block (RsxBlock{
							initial: Box::new(value.clone().into_rsx()),
							effect: Effect::new(#ident::register_block(value), #tracker),
						})
					}
				}
			}
			Node::Fragment(NodeFragment { children, .. }) => {
				self.map_nodes(children)
			}
			Node::Element(el) => {
				self.check_self_closing_children(&el);
				let NodeElement {
					mut open_tag,
					children,
					close_tag,
				} = el;
				// we must parse runtime attr before anything else
				self.parse_runtime_attribute(&mut open_tag.attributes);

				self.collected_elements.push(open_tag.name.clone());
				let self_closing = close_tag.is_none();
				if let Some(close_tag) = close_tag {
					self.collected_elements.push(close_tag.name.clone());
				}
				let tag = open_tag.name.to_string();

				if tag.starts_with(|c: char| c.is_uppercase()) {
					self.map_component(tag, open_tag, children)
				} else {
					let attributes = open_tag
						.attributes
						.into_iter()
						.map(|attr| self.map_attribute(attr))
						.collect::<Vec<_>>();
					let children = self.map_nodes(children);
					quote!(RsxNode::Element(RsxElement {
						tag: #tag.to_string(),
						attributes: vec![#(#attributes),*],
						children: Box::new(#children),
						self_closing: #self_closing,
					}))
				}
			}
			Node::Custom(_) => unimplemented!("Custom nodes not yet supported"),
		}
	}

	fn map_attribute(&mut self, attr: NodeAttribute) -> TokenStream {
		let ident = &self.idents.effect;
		match attr {
			NodeAttribute::Block(block) => {
				let tracker = self.rusty_tracker.next_tracker(&block);
				quote! {
					RsxAttribute::Block{
						initial: vec![#block.clone().into_rsx()],
						effect: Effect::new(#ident::register_attribute_block(#block), #tracker)
					}
				}
			}
			NodeAttribute::Attribute(attr) => {
				let key = attr.key.to_string();
				match attr.value() {
					None => quote!(RsxAttribute::Key {
						key: #key.to_string()
					}),
					Some(syn::Expr::Lit(expr_lit)) => {
						let value = lit_to_string(&expr_lit.lit);
						quote! {
							RsxAttribute::KeyValue {
								key: #key.to_string(),
								value: #value.to_string()
							}
						}
					}
					Some(block) => {
						let tracker = self.rusty_tracker.next_tracker(&block);
						if key.starts_with("on") {
							let key = key.to_string();

							let register_func = syn::Ident::new(
								&format!("register_{key}"),
								block.span(),
							);
							let register_event = &self.idents.event;
							quote! {
								RsxAttribute::BlockValue {
									key: #key.to_string(),
									initial: "event-placeholder".to_string(),
									effect: Effect::new(Box::new(move |cx| {
										#register_event::#register_func(#key,cx,#block);
										Ok(())
									}), #tracker)
								}
							}
						} else {
							quote! {
								RsxAttribute::BlockValue{
									key: #key.to_string(),
									initial: #block.clone().into_attribute_value(),
									effect: Effect::new(#ident::register_attribute_value(#key, #block), #tracker)
								}
							}
						}
					}
				}
			}
		}
	}
	fn map_component<C>(
		&mut self,
		tag: String,
		open_tag: OpenTag,
		children: Vec<Node<C>>,
	) -> TokenStream {
		let tracker = self.rusty_tracker.next_tracker(&open_tag);
		let props = open_tag.attributes.into_iter().map(|attr| match attr {
			NodeAttribute::Block(node_block) => {
				quote! {#node_block}
			}
			NodeAttribute::Attribute(attr) => {
				if let Some(value) = attr.value() {
					let key = &attr.key;
					// apply the value to the field
					quote! {
						#key: #value
					}
				} else {
					let key = &attr.key;
					// for components a key is treated as bool
					quote! {#key: true}
				}
			}
		});
		let ident = syn::Ident::new(&tag, tag.span());
		let slot_children = self.map_nodes(children);
		quote!({
			RsxNode::Component(RsxComponent{
				tag: #tag.to_string(),
				tracker: #tracker,
				root: Box::new(#ident{
					#(#props,)*
				}
				.render()),
				slot_children: Box::new(#slot_children)
			})
		})
	}

	/// Ensure that self-closing elements do not have children.
	fn check_self_closing_children<C>(&mut self, element: &NodeElement<C>) {
		if element.children.is_empty()
			|| !self
				.self_closing_elements
				.contains(element.open_tag.name.to_string().as_str())
		{
			return;
		}
		let warning = Diagnostic::spanned(
			element.open_tag.name.span(),
			Level::Warning,
			"Element is processed as empty, and cannot have any child",
		);
		self.errors.push(warning.emit_as_expr_tokens());
	}


	/// Update [`Self::idents`] with the specified runtime and removes it from
	/// the list of attributes. See [`RsxIdents::set_runtime`] for more information.
	fn parse_runtime_attribute(&mut self, attrs: &mut Vec<NodeAttribute>) {
		attrs.retain(|attr| {
			if let NodeAttribute::Attribute(attr) = attr {
				let key = attr.key.to_string();
				if key.starts_with("runtime:") {
					let runtime = key.replace("runtime:", "");
					if let Err(err) = self.idents.set_runtime(&runtime) {
						let diagnostic = Diagnostic::spanned(
							attr.span(),
							Level::Error,
							err.to_string(),
						);
						self.errors.push(diagnostic.emit_as_expr_tokens());
					}
					return false;
				}
			}
			true
		});
	}
}
