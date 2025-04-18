use crate::parse_rsx::meta_builder::MetaBuilder;
use crate::parse_rsx::meta_builder::TemplateDirectiveTokens;
use crate::prelude::*;
use proc_macro2::LineColumn;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::quote;
use sweet::prelude::Pipeline;
use syn::Expr;
use syn::Ident;
use syn::spanned::Spanned;

/// Convert rsx nodes to an RsxNode token stream
/// we intentionally only set the location on the root node,
/// havent yet found a usecase that makes it worth setting on
/// every node
#[derive(Debug)]
pub struct HtmlTokensToRust {
	/// The entrypoint of the macro,
	pub location: LineColumn,
	pub idents: RsxIdents,
	// Additional error and warning messages.
	pub errors: Vec<TokenStream>,
	// Collect elements to provide semantic highlight based on element tag.
	// No differences between open tag and closed tag.
	// Also multiple tags with same name can be present,
	// because we need to mark each of them.
	pub rusty_tracker: RustyTrackerBuilder,
	/// do not insert compile errors into the output
	pub exclude_errors: bool,
}

impl Pipeline<HtmlTokens, TokenStream> for HtmlTokensToRust {
	fn apply(mut self, node: HtmlTokens) -> TokenStream {
		let node = self.map_node(node);

		let errors = if self.exclude_errors {
			Default::default()
		} else {
			self.errors
		};

		let line = self.location.line as u32;
		let col = self.location.column as u32;

		quote! {
			{
				#(#errors;)*
				use beet::prelude::*;
				#[allow(unused_braces)]
				#node.with_location(RsxMacroLocation::new(file!(), #line, #col))
			}
		}
	}
}
impl Default for HtmlTokensToRust {
	fn default() -> Self {
		Self::new(RsxIdents::default(), LineColumn { line: 0, column: 0 })
	}
}


impl HtmlTokensToRust {
	pub fn new_spanned(idents: RsxIdents, entry: &impl Spanned) -> Self {
		Self::new(idents, entry.span().start())
	}
	pub fn new(idents: RsxIdents, location: LineColumn) -> Self {
		Self {
			idents,
			location,
			errors: Vec::new(),
			rusty_tracker: Default::default(),
			exclude_errors: false,
		}
	}

	/// returns an RsxNode
	fn map_node(&mut self, node: HtmlTokens) -> TokenStream {
		match node {
			HtmlTokens::Fragment { nodes } => {
				let children = nodes.into_iter().map(|n| self.map_node(n));
				quote! { RsxFragment{
					nodes: vec![#(#children),*],
					meta: RsxNodeMeta::default(),
				}.into_node()}
			}
			HtmlTokens::Doctype { value: _ } => {
				quote!(
					RsxDoctype {
						meta: RsxNodeMeta::default()
					}
					.into_node()
				)
			}
			HtmlTokens::Comment { value } => {
				quote!(RsxComment {
					value: #value.to_string(),
					meta: RsxNodeMeta::default(),
				}.into_node())
			}
			HtmlTokens::Text { value } => {
				quote!(RsxText {
					value: #value.to_string(),
					meta: RsxNodeMeta::default(),
				}.into_node())
			}
			HtmlTokens::Block { value } => {
				let tracker = self.rusty_tracker.next_tracker(&value);

				let ident = &self.idents.runtime.effect;
				quote! {
					#ident::parse_block_node(#tracker, #value)
				}
			}
			HtmlTokens::Element {
				component,
				children,
				self_closing,
			} => {
				let RsxNodeTokens {
					tag,
					attributes,
					directives,
					..
				} = &component;


				// we must parse runtime attr before anything else
				self.parse_runtime_directive(&directives);
				let tag_str = tag.to_string();
				if tag_str.starts_with(|c: char| c.is_uppercase()) {
					self.map_component(component, *children)
				} else {
					let meta = MetaBuilder::build_with_directives(&directives);
					// this attributes-children order is important for rusty tracker indices
					// to be consistend with HtmlTokensToRon
					let attributes = attributes
						.iter()
						.map(|attr| self.map_attribute(attr))
						.collect::<Vec<_>>();
					let children = self.map_node(*children);

					quote!(RsxElement {
						tag: #tag_str.to_string(),
						attributes: vec![#(#attributes),*],
						children: Box::new(#children),
						self_closing: #self_closing,
						meta: #meta,
					}.into_node())
				}
			}
		}
	}

	fn map_attribute(&mut self, attr: &RsxAttributeTokens) -> TokenStream {
		let ident = &self.idents.runtime.effect;
		match attr {
			// The attribute is a block
			RsxAttributeTokens::Block { block } => {
				let tracker = self.rusty_tracker.next_tracker(&block);
				quote! {
					#ident::parse_attribute_block(
						#tracker,
						#block,
					)
				}
			}
			// The attribute is a key
			RsxAttributeTokens::Key { key } => {
				let key_str = key.to_string();
				quote!(RsxAttribute::Key {
					key: #key_str.to_string()
				})
			}
			// the attribute is a key value where
			// the value is a string literal
			RsxAttributeTokens::KeyValue { key, value }
				if let Expr::Lit(lit) = &value.value =>
			{
				let key_str = key.to_string();

				quote! {
					RsxAttribute::KeyValue {
						key: #key_str.to_string(),
						value: #lit.to_string()
					}
				}
			}
			// the attribute is a key value where the value
			// is not an [`Expr::Lit`]
			RsxAttributeTokens::KeyValue { key, value } => {
				let key_str = key.to_string();
				let tracker = self.rusty_tracker.next_tracker(value);
				// we need to handle events at the tokens level for inferred
				// event types and intellisense.
				if key_str.starts_with("on") {
					let register_func = syn::Ident::new(
						&format!("register_{key_str}"),
						value.span(),
					);
					let event_registry = &self.idents.runtime.event;
					quote! {
						RsxAttribute::BlockValue {
							key: #key_str.to_string(),
							initial: "event-placeholder".to_string(),
							effect: Effect::new(Box::new(move |cx| {
								#event_registry::#register_func(#key_str,cx,#value);
								Ok(())
							}), #tracker)
						}
					}
				} else {
					quote! {
						#ident::parse_attribute_value(
							#key_str,
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
		RsxNodeTokens {
			tag,
			tokens,
			attributes,
			directives,
		}: RsxNodeTokens,
		children: HtmlTokens,
	) -> TokenStream {
		let tag_str = tag.to_string();

		let tracker = self.rusty_tracker.next_tracker(&tokens);
		let mut prop_assignments = Vec::new();
		let mut prop_names = Vec::new();
		// currently unused but we could allow setting component directly,
		// like <Component {component} />
		let mut block_attr = None;

		for attr in attributes.iter() {
			match attr {
				RsxAttributeTokens::Block { block } => {
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
					// for components no value means a bool flag
					prop_assignments.push(quote! {.#key(true)});
				}
				RsxAttributeTokens::KeyValue { key, value } => {
					prop_names.push(key);
					prop_assignments.push(quote! {.#key(#value)});
				}
			}
		}

		let meta = MetaBuilder::build_with_directives(&directives);

		let ident = syn::Ident::new(&tag_str, tokens.span());
		let slot_children = self.map_node(children);

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
			tokens.span(),
		);

		quote::quote!({
				let _ = #ide_helper::default();

				let component = #component;

				RsxComponent{
					tag: #tag_str.to_string(),
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
	fn parse_runtime_directive(
		&mut self,
		directives: &[TemplateDirectiveTokens],
	) {
		for directive in directives.iter() {
			if let TemplateDirectiveTokens::Runtime(runtime) = directive {
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
mod test {}
