use crate::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::quote;
use quote::ToTokens;
use rstml::node::KeyedAttribute;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeElement;
use rstml::node::NodeFragment;
use rstml::node::NodeName;
use std::collections::HashMap;
use std::collections::HashSet;
use syn::spanned::Spanned;


/// Convert rstml nodes to a Vec<RsxNode> token stream
#[derive(Debug, Default, Clone)]
pub struct RstmlToRsx {
	pub hash_location: bool,
	pub idents: RsxIdents,
	// Additional error and warning messages.
	pub errors: Vec<TokenStream>,
	// Collect elements to provide semantic highlight based on element tag.
	// No differences between open tag and closed tag.
	// Also multiple tags with same name can be present,
	// because we need to mark each of them.
	pub collected_elements: Vec<NodeName>,
	pub self_closing_elements: HashSet<&'static str>,
}

impl RstmlToRsx {
	pub fn new(idents: RsxIdents) -> Self {
		Self {
			idents,
			..Default::default()
		}
	}

	pub fn location_hash(&self, span: impl Spanned) -> TokenStream {
		if self.hash_location {
			let tokens = span_to_line_col(&span.span());
			quote! {Some(#tokens)}
		} else {
			quote! {None}
		}
	}

	pub fn map_tokens(&mut self, tokens: TokenStream) -> TokenStream {
		let (nodes, rstml_errors) = tokens_to_rstml(tokens.clone());
		let nodes = self.map_nodes(nodes);
		let span = tokens.span();
		let line = span.start().line;
		let col = span.start().column;

		quote! {
			{
				#(#rstml_errors;)*
				use beet::prelude::*;
				#[allow(unused_braces)]
				RsxNode::Root{
					location: RsxLocation::new(std::file!(), #line, #col),
					nodes: Vec::from([#(#nodes),*])
				}

			}
		}
	}
	/// the number of actual html nodes will likely be different
	/// due to fragments, blocks etc
	pub fn map_nodes<C>(&mut self, nodes: Vec<Node<C>>) -> Vec<TokenStream> {
		nodes.into_iter().map(|node| self.map_node(node)).collect()
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
				let ident = &self.idents.effect;
				// block is a {block} so assign to a value to unwrap
				let location = self.location_hash(&block);
				quote! {
					{
						let value = #block;
						RsxNode::Block {
							initial: Box::new(value.clone().into_rsx()),
							effect: Effect::new(#ident::register_block(value), #location),
						}
					}
				}
			}
			Node::Fragment(NodeFragment { children, .. }) => {
				let children = self.map_nodes(children);
				quote!(RsxNode::Fragment(Vec::from([#(#children),*])))
			}
			Node::Element(el) => {
				self.check_self_closing_children(&el);
				let NodeElement {
					open_tag,
					children,
					close_tag,
				} = el;

				self.collected_elements.push(open_tag.name.clone());
				let self_closing = close_tag.is_none();
				if let Some(close_tag) = close_tag {
					self.collected_elements.push(close_tag.name.clone());
				}
				let tag = open_tag.name.to_string();

				let is_component = tag.starts_with(|c: char| c.is_uppercase());
				if is_component {
					self.map_component(&tag, &open_tag.attributes, children)
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
						children: vec![#(#children),*],
						self_closing: #self_closing,
					}))
				}
			}
			Node::Custom(_) => unimplemented!("Custom nodes not yet supported"),
		}
	}

	fn map_component<C>(
		&mut self,
		tag: &str,
		attributes: &Vec<NodeAttribute>,
		children: Vec<Node<C>>,
	) -> TokenStream {
		let props = attributes.into_iter().map(|attr| match attr {
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
		let children_slot = self.map_slots(children);

		quote!({
			RsxNode::Component{
				tag: #tag.to_string(),
				node: Box::new(#ident{
					#(#props,)*
				}
				.into_rsx()#children_slot)
			}
		})
	}

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

	fn map_slots<C>(&mut self, children: Vec<Node<C>>) -> TokenStream {
		if children.is_empty() {
			TokenStream::default()
		} else {
			let mut slot_buckets = HashMap::new();
			for child in children.into_iter() {
				let slot_name = self.slot_name(&child);
				slot_buckets
					.entry(slot_name)
					.or_insert_with(Vec::new)
					.push(self.map_node(child));
			}
			let with_slots = slot_buckets.into_iter().map(
				|(name, children)| quote!(.with_slots(#name,vec![#(#children),*])),
			);
			quote! {#(#with_slots)*}
		}
	}

	/// given a node, try to find the attribute called 'slot' and
	/// return its value, otherwise return 'default'
	fn slot_name<C>(&self, node: &Node<C>) -> String {
		match node {
			Node::Element(NodeElement { open_tag, .. }) => {
				for attr in &open_tag.attributes {
					match attr {
						NodeAttribute::Attribute(attr) => {
							match attr.key.to_string().as_str() {
								"slot" => {
									return attr_val_str(&attr).expect(
										"slot values must be string literals",
									)
								}
								_ => {}
							}
						}
						_ => {}
					}
				}
			}
			_ => {}
		}
		"default".to_string()
	}


	fn map_attribute(&mut self, attr: NodeAttribute) -> TokenStream {
		let ident = &self.idents.effect;
		match attr {
			NodeAttribute::Block(block) => {
				let location = self.location_hash(&block);
				quote! {
					RsxAttribute::Block{
						initial: vec![#block.clone().into_rsx()],
						effect: Effect::new(#ident::register_attribute_block(#block), #location)
					}
				}
			}
			NodeAttribute::Attribute(attr) => {
				let key = attr.key.to_string();
				match attr.value() {
					None => quote!(RsxAttribute::Key {
						key: #key.to_string()
					}),
					Some(block) => {
						if let Some(val) = attr_val_str(&attr) {
							quote! {
								RsxAttribute::KeyValue {
									key: #key.to_string(),
									value: #val.to_string()
								}
							}
						} else if key.starts_with("on") {
							let key = key.to_string();

							let register_func = syn::Ident::new(
								&format!("register_{key}"),
								block.span(),
							);
							let register_event = &self.idents.event;
							let location = self.location_hash(&block);
							quote! {
								RsxAttribute::BlockValue {
									key: #key.to_string(),
									initial: "needs-event-cx".to_string(),
									effect: Effect::new(Box::new(move |cx| {
										#register_event::#register_func(#key,cx,#block);
										Ok(())
									}), #location)
								}
							}
						} else {
							let location = self.location_hash(&block);
							quote! {
								RsxAttribute::BlockValue{
									key: #key.to_string(),
									initial: #block.clone().into_attribute_value(),
									effect: Effect::new(#ident::register_attribute_value(#key, #block), #location)
								}
							}
						}
					}
				}
			}
		}
	}
}

/// if the value is a literal, parse as string
fn attr_val_str(attr: &KeyedAttribute) -> Option<String> {
	match &attr.value() {
		Some(syn::Expr::Lit(expr_lit)) => {
			let value = match &expr_lit.lit {
				syn::Lit::Str(s) => s.value(),
				other => other.to_token_stream().to_string(),
			};
			Some(value)
		}
		_ => None,
	}
}
