use crate::prelude::*;
use beet_common::prelude::SerdeTokens;
use proc_macro2::TokenStream;
use quote::quote;
use sweet::prelude::*;
use syn::Expr;

/// Convert [`WebTokens`] to a ron format.
/// Rust block token streams will be hashed by [Span::start]
#[derive(Debug, Default)]
pub struct WebTokensToRon {
	rusty_tracker: RustyTrackerBuilder,
}

impl Pipeline<WebTokens, TokenStream> for WebTokensToRon {
	fn apply(mut self, node: WebTokens) -> TokenStream { self.map_node(node) }
}

impl WebTokensToRon {
	/// returns an RsxTemplateNode
	pub fn map_node(&mut self, node: WebTokens) -> TokenStream {
		match node {
			WebTokens::Fragment { nodes, meta } => {
				let meta = meta.into_ron_tokens();
				let nodes = nodes.into_iter().map(|n| self.map_node(n));
				quote! { Fragment (
					items:[#(#nodes),*],
					meta: #meta
				)}
			}
			WebTokens::Doctype { value: _, meta } => {
				let meta = meta.into_ron_tokens();
				quote! { Doctype (
					meta: #meta
				)}
			}
			WebTokens::Comment { value, meta } => {
				let meta = meta.into_ron_tokens();
				quote! { Comment (
					value: #value,
					meta: #meta
				)}
			}
			WebTokens::Text { value, meta } => {
				let meta = meta.into_ron_tokens();
				quote! { Text (
					value: #value,
					meta: #meta
				)}
			}
			WebTokens::Block { value, meta } => {
				let meta = meta.into_ron_tokens();
				let tracker = self.rusty_tracker.next_tracker_ron(&value);
				quote! { RustBlock (
					tracker:#tracker,
					meta: #meta
				)}
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
				let meta = meta.into_ron_tokens();

				let tag_str = tag.to_string();
				if tag_str.starts_with(|c: char| c.is_uppercase()) {
					// components disregard all the context and rely on the tracker
					// we rely on the hydrated node to provide the attributes and children
					let tracker =
						self.rusty_tracker.next_tracker_ron(&component);
					let slot_children = self.map_node(*children);

					quote! { Component (
						tracker: #tracker,
						tag: #tag_str,
						slot_children: #slot_children,
						meta: #meta
					)}
				} else {
					// this attributes-children order is important for rusty tracker indices
					// to be consistent with WebTokensToRust
					let attributes = attributes
						.into_iter()
						.map(|a| self.map_attribute(&a))
						.collect::<Vec<_>>();
					let children = self.map_node(*children);
					quote! { Element (
						tag: #tag_str,
						self_closing: #self_closing,
						attributes: [#(#attributes),*],
						children: #children,
						meta: #meta
					)}
				}
			}
		}
	}

	fn map_attribute(&mut self, attr: &RsxAttributeTokens) -> TokenStream {
		match attr {
			RsxAttributeTokens::Block { block } => {
				let tracker = self.rusty_tracker.next_tracker_ron(&block.value);
				quote! { Block (#tracker)}
			}
			RsxAttributeTokens::Key { key } => {
				let key_str = key.to_string();
				quote! {Key ( key: #key_str )}
			}
			RsxAttributeTokens::KeyValue { key, value }
				if let Expr::Lit(value) = &value.value =>
			{
				let key_str = key.to_string();
				// ron stringifies all lit values?
				// tbh not sure why we need to do this but it complains need string
				let value = lit_to_string(&value.lit);
				quote! { KeyValue (
						key: #key_str,
						value: #value
						)
				}
			}
			// the attribute is a key value where the value
			// is not an [`Expr::Lit`]
			RsxAttributeTokens::KeyValue { key, value } => {
				let tracker = self.rusty_tracker.next_tracker_ron(&value.value);
				let key_str = key.to_string();
				// we dont need to handle events for serialization,
				// thats an rstml_to_rsx concern so having the tracker is enough
				quote! { BlockValue (
					key: #key_str,
					tracker: #tracker
				)}
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
