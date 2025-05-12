use crate::prelude::*;
use beet_common::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::quote;
use sweet::prelude::Pipeline;
use syn::Block;
use syn::Ident;
use syn::spanned::Spanned;

/// Convert web nodes to a [`WebNode`] token stream
/// we intentionally only set the location on the root node,
/// havent yet found a usecase that makes it worth setting on
/// every node, and we would need to pass locations of non-proc_macro
/// tokens too.
#[derive(Debug, Default)]
pub struct WebTokensToRust {
	pub idents: RsxIdents,
	// Additional error and warning messages.
	pub errors: Vec<TokenStream>,
	/// do not insert compile errors into the output
	pub exclude_errors: bool,
}

impl Pipeline<WebTokens, Block> for WebTokensToRust {
	fn apply(mut self, node: WebTokens) -> Block {
		let node = self.map_node(node);

		let errors = if self.exclude_errors {
			Default::default()
		} else {
			self.errors
		};


		syn::parse_quote! {
			{
				#(#errors;)*
				use beet::prelude::*;
				#[allow(unused_braces)]
				#node
			}
		}
	}
}

impl WebTokensToRust {
	pub fn with_idents(self, idents: RsxIdents) -> Self {
		Self { idents, ..self }
	}

	/// returns an WebNode
	fn map_node(&mut self, node: WebTokens) -> TokenStream {
		match node {
			WebTokens::Fragment { nodes, meta } => {
				let meta = meta.into_rust_tokens();
				let nodes = nodes.into_iter().map(|n| self.map_node(n));
				quote! { RsxFragment{
					nodes: vec![#(#nodes),*],
					meta: #meta,
				}.into_node()}
			}
			WebTokens::Doctype { value: _, meta } => {
				let meta = meta.into_rust_tokens();
				quote!(
					RsxDoctype {
						meta: #meta
					}
					.into_node()
				)
			}
			WebTokens::Comment { value, meta } => {
				let meta = meta.into_rust_tokens();
				quote!(RsxComment {
					value: #value.to_string(),
					meta: #meta,
				}.into_node())
			}
			WebTokens::Text { value, meta } => {
				let meta = meta.into_rust_tokens();
				quote!(RsxText {
					value: #value.to_string(),
					meta: #meta,
				}.into_node())
			}
			WebTokens::Block {
				value,
				// Block meta is not used in WebTokensToRust,
				// but it is used in WebTokensToRon
				meta: _,
				tracker,
			} => {
				let tracker = tracker.into_rust_tokens();
				let ident = &self.idents.runtime.effect;
				quote! {
					#ident::parse_block_node(#tracker, #value)
				}
			}
			WebTokens::Element {
				component,
				children,
				self_closing,
			} => {
				let ElementTokens {
					tag,
					attributes,
					meta,
					..
				} = &component;

				// we must parse runtime attr before anything else
				self.parse_runtime_directive(&meta);
				// this attributes-children order is important for rusty tracker indices
				// to be consistent with [`WebTokensToRon`]
				let attributes = attributes
					.iter()
					.map(|attr| self.map_attribute(attr))
					.collect::<Vec<_>>();
				let children = self.map_node(*children);
				let meta = meta.into_rust_tokens();
				quote!(RsxElement {
						tag: #tag.to_string(),
						attributes: vec![#(#attributes),*],
						children: Box::new(#children),
						self_closing: #self_closing,
						meta: #meta,
					}.into_node())
			}
			WebTokens::Component {
				component,
				children,
				tracker,
			} => self.map_component(component, *children, tracker),
		}
	}

	fn map_attribute(&mut self, attr: &RsxAttributeTokens) -> TokenStream {
		let ident = &self.idents.runtime.effect;
		match attr {
			// The attribute is a block
			RsxAttributeTokens::Block { block, tracker } => {
				let tracker = tracker.into_rust_tokens();
				quote! {
					#ident::parse_attribute_block(
						#tracker,
						#block,
					)
				}
			}
			// The attribute is a key
			RsxAttributeTokens::Key { key } => {
				quote!(RsxAttribute::Key {
					key: #key.to_string()
				})
			}
			// the attribute is a key value where
			// the value is a string literal
			RsxAttributeTokens::KeyValueLit { key, value } => {
				quote! {
					RsxAttribute::KeyValue {
						key: #key.to_string(),
						value: #value.to_string()
					}
				}
			}
			// the attribute is a key value where the value
			// is not an [`Expr::Lit`]
			RsxAttributeTokens::KeyValueExpr {
				key,
				value,
				tracker,
			} => {
				let tracker = tracker.into_rust_tokens();
				// we need to handle events at the tokens level for inferred
				// event types and intellisense.
				if key.as_str().starts_with("on") {
					let register_event = self
						.idents
						.runtime
						.register_event_tokens(&key.as_str(), value);
					quote! {
						RsxAttribute::BlockValue {
							key: #key.to_string(),
							initial: "event-placeholder".to_string(),
							effect: Effect::new(Box::new(move |loc| {
								#register_event
								Ok(())
							}), #tracker)
						}
					}
				} else {
					quote! {
						#ident::parse_attribute_value(
							#key,
							#tracker,
							#value
						)
					}
				}
			}
		}
	}

	fn map_component(
		&mut self,
		component: ElementTokens,
		children: WebTokens,
		tracker: RustyTracker,
	) -> TokenStream {
		let ElementTokens {
			tag,
			attributes,
			meta,
		} = component;
		// visiting slot children is safe here, we aren't pulling any more trackers
		let slot_children = self.map_node(children);

		let mut prop_assignments = Vec::new();
		let mut prop_names = Vec::new();
		// currently unused but we could allow setting component directly,
		// like <Component {component} />
		let mut block_attr = None;

		for attr in attributes.iter() {
			match attr {
				RsxAttributeTokens::Block { block, tracker: _ } => {
					if block_attr.is_some() {
						let diagnostic = Diagnostic::spanned(
							block.span(),
							Level::Error,
							"Only one block attribute is allowed per component",
						);
						self.errors.push(diagnostic.emit_as_expr_tokens());
					}
					block_attr = Some(block);
				}
				RsxAttributeTokens::Key { key } => {
					prop_names.push(key);
					let key = key.into_ident();
					// for components no value means a bool flag
					prop_assignments.push(quote! {.#key(true)});
				}
				RsxAttributeTokens::KeyValueLit { key, value } => {
					prop_names.push(key);
					let key = key.into_ident();
					prop_assignments.push(quote! {.#key(#value)});
				}
				RsxAttributeTokens::KeyValueExpr { key, value, .. } => {
					prop_names.push(key);
					let key = key.into_ident();
					prop_assignments.push(quote! {.#key(#value)});
				}
			}
		}


		let ident = Ident::new(tag.as_str(), tag.tokens_span());

		// ensures all required fields are set
		// doesnt work because we cant tell whether its an optional or default
		// just by its name
		// let impl_required = quote::quote_spanned! {open_tag.span()=>
		// 			let _ = <#ident as Props>::Required{
		// 				#(#prop_names: Default::default()),*
		// 			};
		// };

		// TODO spread, ie allow the block component to be turned
		// into a builder and apply props
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

		let ron = if meta.is_client_reactive() {
			quote! {{
				#[cfg(target_arch = "wasm32")]
				{None}
				#[cfg(not(target_arch = "wasm32"))]
				{Some(beet::exports::ron::ser::to_string(&component).unwrap())}
			}}
		} else {
			quote! {None}
		};

		// attempt to get ide to show the correct type by using
		// the component as the first spanned quote
		let ide_helper =
			Ident::new(&format!("{}Required", tag.as_str()), tag.tokens_span());

		let meta = meta.into_rust_tokens();
		let tracker = tracker.into_rust_tokens();
		quote::quote!({
				let _ = #ide_helper::default();

				let component = #component;

				RsxComponent{
					tag: #tag.to_string(),
					type_name: std::any::type_name::<#ident>().to_string(),
					tracker: #tracker,
					ron: #ron,
					node: Box::new(component.into_node()),
					slot_children: Box::new(#slot_children),
					meta: #meta
				}.into_node()
		})
	}

	/// Update [`Self::idents`] with the specified runtime and removes it from
	/// the list of attributes. See [`RsxIdents::set_runtime`] for more information.
	fn parse_runtime_directive(&mut self, meta: &NodeMeta) {
		if let Some(runtime) = meta.runtime() {
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


#[cfg(test)]
mod test {}
