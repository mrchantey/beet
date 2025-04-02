use crate::parse_rsx::meta_builder::MetaBuilder;
use crate::parse_rsx::meta_builder::ParsedTemplateDirective;
use crate::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::ToTokens;
use quote::quote;
use rapidhash::RapidHashSet as HashSet;
use rstml::atoms::OpenTag;
use rstml::node::CustomNode;
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
	let line = span.start().line as u32;
	let col = span.start().column as u32;
	quote! {
		{
			RsxMacroLocation::new(file!(), #line, #col)
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

		let location = macro_location_tokens(tokens);
		let rstml_to_rsx_errors = &self.errors;

		// we intentionally only set the location on the root node,
		// havent yet found a usecase that makes it worth setting on
		// every node
		quote! {
			{
				#(#rstml_errors;)*
				#(#rstml_to_rsx_errors;)*
				use beet::prelude::*;
				#[allow(unused_braces)]
				#node.with_location(#location)
			}
		}
	}

	/// the number of actual html nodes will likely be different
	/// due to fragments, blocks etc
	pub fn map_nodes<C: CustomNode>(
		&mut self,
		nodes: Vec<Node<C>>,
	) -> TokenStream {
		let mut nodes = nodes
			.into_iter()
			.map(|node| self.map_node(node))
			.collect::<Vec<_>>();
		if nodes.len() == 1 {
			nodes.pop().unwrap().to_token_stream()
		} else {
			quote!( RsxFragment {
				nodes: Vec::from([#(#nodes),*]),
				meta: RsxNodeMeta::default(),
			}.into_node())
		}
	}

	/// returns an RsxNode
	fn map_node<C: CustomNode>(&mut self, node: Node<C>) -> TokenStream {
		match node {
			Node::Doctype(_) => {
				quote!(
					RsxDoctype {
						meta: RsxNodeMeta::default()
					}
					.into_node()
				)
			}
			Node::Comment(comment) => {
				let comment = comment.value.value();
				quote!(RsxComment {
					value: #comment.to_string(),
					meta: RsxNodeMeta::default(),
				}.into_node())
			}
			Node::Text(text) => {
				let text = text.value_string();
				quote!(RsxText {
					value: #text.to_string(),
					meta: RsxNodeMeta::default(),
				}.into_node())
			}
			Node::RawText(raw) => {
				let text = raw.to_string_best();
				quote!(RsxText {
					value: #text.to_string(),
					meta: RsxNodeMeta::default(),
				}.into_node())
			}
			Node::Fragment(NodeFragment { children, .. }) => {
				let children = children.into_iter().map(|n| self.map_node(n));
				quote! { RsxFragment{
					nodes: vec![#(#children),*],
					meta: RsxNodeMeta::default(),
				}.into_node()}
			}
			Node::Block(block) => {
				let tracker = self.rusty_tracker.next_tracker(&block);

				let ident = &self.idents.runtime.effect;
				quote! {
					#ident::parse_block_node(#tracker, #block)
				}
			}
			Node::Element(el) => {
				self.check_self_closing_children(&el);

				let NodeElement {
					open_tag,
					children,
					close_tag,
				} = el;


				let (directives, attributes) =
					MetaBuilder::parse_attributes(&open_tag.attributes);

				// we must parse runtime attr before anything else
				self.parse_runtime_directive(&directives);

				self.collected_elements.push(open_tag.name.clone());
				let self_closing = close_tag.is_none();
				if let Some(close_tag) = close_tag {
					self.collected_elements.push(close_tag.name.clone());
				}
				let tag = open_tag.name.to_string();

				if tag.starts_with(|c: char| c.is_uppercase()) {
					self.map_component(
						tag,
						&open_tag,
						&attributes,
						&directives,
						children,
					)
				} else {
					// panic!();
					// println!("its a style tag");
					#[cfg(feature = "css")]
					if tag == "style" {
						if let Err(err) = validate_style_node(&children) {
							self.errors.push(
								Diagnostic::spanned(err.0, Level::Error, err.1)
									.emit_as_expr_tokens(),
							);
						}
					}

					let attributes = attributes
						.iter()
						.map(|attr| self.map_attribute(attr))
						.collect::<Vec<_>>();


					let meta = MetaBuilder::build_with_directives(&directives);

					let children = self.map_nodes(children);
					quote!(RsxElement {
						tag: #tag.to_string(),
						attributes: vec![#(#attributes),*],
						children: Box::new(#children),
						self_closing: #self_closing,
						meta: #meta,
					}.into_node())
				}
			}
			Node::Custom(_) => unimplemented!("Custom nodes not yet supported"),
		}
	}

	fn map_attribute(&mut self, attr: &NodeAttribute) -> TokenStream {
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
	fn map_component<C: CustomNode>(
		&mut self,
		tag: String,
		open_tag: &OpenTag,
		attributes: &[&NodeAttribute],
		directives: &[ParsedTemplateDirective],
		children: Vec<Node<C>>,
	) -> TokenStream {
		let tracker = self.rusty_tracker.next_tracker(&open_tag);
		let mut prop_assignments = Vec::new();
		let mut prop_names = Vec::new();
		// currently unused but we could allow setting component directly,
		// like <Component {component} />
		let mut block_attr = None;

		for attr in attributes.iter() {
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
					prop_names.push(attr_key);
					let value = match attr.value() {
						Some(value) => quote! {#value},
						// for components no value means a bool flag
						None => quote! {true},
					};
					prop_assignments.push(quote! {.#attr_key(#value)});
				}
			}
		}

		let meta = MetaBuilder::build_with_directives(&directives);

		let ident = syn::Ident::new(&tag, open_tag.span());
		let slot_children = self.map_nodes(children);


		// ensures all required fields are set
		// doesnt work because we cant tell whether its an optional or default
		// just by its name
		// let impl_required = quote::quote_spanned! {open_tag.span()=>
		// 			let _ = <#ident as Props>::Required{
		// 				#(#prop_names: Default::default()),*
		// 			};
		// };

		let component = if let Some(node_block) = block_attr {
			quote! {
				#node_block
			}
		} else {
			quote!({
				// #impl_required
				<#ident as Props>::Builder::default()
				#(#prop_assignments)*
				.build()
			})
		};

		let ron = if directives.iter().any(|d| d.is_client_reactive()) {
			quote! {{
				#[cfg(target_arch = "wasm32")]
				{None}
				#[cfg(not(target_arch = "wasm32"))]
				{Some(ron::ser::to_string(&component).unwrap())}
			}}
		} else {
			quote! {None}
		};

		// attempt to get ide to show the correct type by using
		// the component as the first spanned quote
		let ide_helper = Ident::new(
			&format!("{}Required", &ident.to_string()),
			open_tag.span(),
		);

		quote::quote!({
				let _ = #ide_helper::default();

				let component = #component;

				RsxComponent{
					tag: #tag.to_string(),
					type_name: std::any::type_name::<#ident>().to_string(),
					tracker: #tracker,
					ron: #ron,
					node: Box::new(component.render()),
					slot_children: Box::new(#slot_children),
					meta: #meta
				}.into_node()
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
	fn parse_runtime_directive(
		&mut self,
		directives: &[ParsedTemplateDirective],
	) {
		for directive in directives.iter() {
			if let ParsedTemplateDirective::Runtime(runtime) = directive {
				if let Err(err) = self.idents.runtime.set(runtime) {
					let diagnostic = Diagnostic::spanned(
						Span::call_site(),
						Level::Error,
						err.to_string(),
					);
					self.errors.push(diagnostic.emit_as_expr_tokens());
				}
			}
		}
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
