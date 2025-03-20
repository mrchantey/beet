use crate::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::ToTokens;
use quote::quote;
use rapidhash::RapidHashSet as HashSet;
use rstml::atoms::OpenTag;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeElement;
use rstml::node::NodeFragment;
use rstml::node::NodeName;
use syn::Ident;
use syn::spanned::Spanned;

/// given a span, for example the inner block
/// of an rsx! or rsx_template! macro,
/// return a [RsxMacroLocation] token stream
pub fn macro_location_tokens(tokens: impl Spanned) -> TokenStream {
	let span = tokens.span();
	let line = span.start().line;
	let col = span.start().column;
	quote! {
		{
			RsxMacroLocation::new(std::file!(), #line, #col)
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
	pub idx_incr: TokensRsxIdxIncr,
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

		let location = macro_location_tokens(tokens);
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
		// if we're creating a fragment it needs idx before children
		let fragment_idx = if nodes.len() == 1 {
			TokenStream::default()
		} else {
			self.idx_incr.next()
		};

		let mut nodes = nodes
			.into_iter()
			.map(|node| self.map_node(node))
			.collect::<Vec<_>>();
		if nodes.len() == 1 {
			nodes.pop().unwrap().to_token_stream()
		} else {
			quote!( RsxNode::Fragment {
				idx: #fragment_idx,
				nodes: Vec::from([#(#nodes),*])
			})
		}
	}

	/// returns an RsxNode
	fn map_node<C>(&mut self, node: Node<C>) -> TokenStream {
		let idx = self.idx_incr.next();
		match node {
			Node::Doctype(_) => quote!(RsxNode::Doctype{
				idx: #idx
			}),
			Node::Comment(comment) => {
				let comment = comment.value.value();
				quote!(RsxNode::Comment{
					idx: #idx,
					value: #comment.to_string()
				})
			}
			Node::Text(text) => {
				let text = text.value_string();
				quote!(RsxNode::Text {
					idx: #idx,
					value: #text.to_string()
				})
			}
			Node::RawText(raw) => {
				let text = raw.to_string_best();
				quote!(RsxNode::Text {
					idx: #idx,
					value: #text.to_string()
				})
			}
			// even if theres one child, fragments still map 1:1 from rstml
			// to keep rsxidx consistent
			Node::Fragment(NodeFragment { children, .. }) => {
				let children = children.into_iter().map(|n| self.map_node(n));
				quote! { RsxNode::Fragment{
					idx: #idx,
					nodes: vec![#(#children),*]
				}}
			}
			Node::Block(block) => {
				let tracker = self.rusty_tracker.next_tracker(&block);

				let ident = &self.idents.runtime.effect;
				quote! {
					#ident::parse_block_node(#idx, #tracker, #block)
				}
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
					self.map_component(idx, tag, open_tag, children)
				} else {
					let attributes = open_tag
						.attributes
						.into_iter()
						.map(|attr| self.map_attribute(attr))
						.collect::<Vec<_>>();
					let children = self.map_nodes(children);
					quote!(RsxNode::Element(RsxElement {
						idx: #idx,
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
		let ident = &self.idents.runtime.effect;
		match attr {
			NodeAttribute::Block(block) => {
				let tracker = self.rusty_tracker.next_tracker(&block);
				quote! {
					#ident::parse_attribute_block(
						#tracker,
						#block,
					)
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
						// we need to handle events at the tokens level for inferred
						// event types and intellisense.
						if key.starts_with("on") {
							let key = key.to_string();

							let register_func = syn::Ident::new(
								&format!("register_{key}"),
								block.span(),
							);
							let event_registry = &self.idents.runtime.event;
							quote! {
								RsxAttribute::BlockValue {
									key: #key.to_string(),
									initial: "event-placeholder".to_string(),
									effect: Effect::new(Box::new(move |cx| {
										#event_registry::#register_func(#key,cx,#block);
										Ok(())
									}), #tracker)
								}
							}
						} else {
							quote! {
								#ident::parse_attribute_value(
									#key,
									#tracker,
									#block
								)
							}
						}
					}
				}
			}
		}
	}
	fn map_component<C>(
		&mut self,
		idx: TokenStream,
		tag: String,
		open_tag: OpenTag,
		children: Vec<Node<C>>,
	) -> TokenStream {
		let tracker = self.rusty_tracker.next_tracker(&open_tag);
		let mut prop_assignments = Vec::new();
		let mut prop_names = Vec::new();
		let mut template_directives = Vec::new();
		// currently unused
		let mut block_attr = None;

		for attr in open_tag.attributes.iter() {
			match attr {
				NodeAttribute::Block(node_block) => {
					if block_attr.is_some() {
						let diagnostic = Diagnostic::spanned(
							node_block.span(),
							Level::Error,
							"Only one block attribute is allowed per component",
						);
						self.errors.push(diagnostic.emit_as_expr_tokens());
					}
					block_attr = Some(node_block);
				}
				NodeAttribute::Attribute(attr) => {
					let attr_key = &attr.key;
					let attr_key_str = attr_key.to_string();
					match (attr_key_str.contains(":"), attr.value()) {
						// its a client directive
						(true, None) => {
							template_directives.push(
								quote! {TemplateDirective::new(#attr_key_str, None)},
							);
						}
						(true, Some(value)) => {
							template_directives.push(
								quote! {TemplateDirective::new(#attr_key_str, Some(#value))},
							);
						}
						// its a prop assignemnt
						(false, None) => {
							prop_names.push(attr_key);
							// for components no value means a bool flag
							prop_assignments.push(quote! {.#attr_key(true)});
						}
						(false, Some(value)) => {
							prop_names.push(attr_key);
							prop_assignments.push(quote! {.#attr_key(#value)});
						}
					}
				}
			}
		}

		let ident = syn::Ident::new(&tag, open_tag.span());
		let slot_children = self.map_nodes(children);


		// ensures all required fields are set
		let impl_required = quote::quote_spanned! {open_tag.span()=>
					let _ = <#ident as Props>::Required{
						#(#prop_names: Default::default()),*
					};
		};

		let root = if let Some(node_block) = block_attr {
			quote! {
				#node_block
				.render()
			}
		} else {
			quote!({
				#impl_required
				<#ident as Props>::Builder::default()
				#(#prop_assignments)*
				.build()
				.render()
			})
		};

		// attempt to get ide to show the correct type by using
		// the component as the first spanned quote
		let ide_helper = Ident::new(
			&format!("{}Required", &ident.to_string()),
			open_tag.span(),
		);

		quote::quote!({
			let _ = #ide_helper::default();
			RsxNode::Component(RsxComponent{
				idx: #idx,
				tag: #tag.to_string(),
				tracker: #tracker,
				root: Box::new(#root),
				slot_children: Box::new(#slot_children),
				template_directives: vec![#(#template_directives),*]
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
					if let Err(err) = self.idents.runtime.set(&runtime) {
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;

	fn map(tokens: TokenStream) -> TokenStream {
		let (nodes, _) = tokens_to_rstml(tokens.clone());
		RstmlToRsx::default().map_nodes(nodes)
	}

	#[test]
	fn block() { let _block = map(quote! {{7}}); }
	// #[test]
	// fn style() { let _block = map(quote! {
	// 	<style>
	// 		main {
	// 			/* min-height:100dvh; */
	// 			min-height: var(--bm-main-height);
	// 			padding: 1em var(--content-padding-width);
	// 		}
	// </style>
	// }); }
}
