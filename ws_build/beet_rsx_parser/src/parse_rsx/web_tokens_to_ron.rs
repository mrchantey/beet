use crate::prelude::*;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use sweet::prelude::*;

/// Convert [`WebTokens`] to a ron format.
/// Rust block token streams will be hashed by [Span::start]
#[derive(Debug, Default)]
pub struct WebTokensToRon;

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
			WebTokens::Block {
				value: _,
				meta,
				tracker,
			} => {
				let meta = meta.into_ron_tokens();
				let tracker = tracker.into_ron_tokens();
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
				// this attributes-children order is important for rusty tracker indices
				// to be consistent with WebTokensToRust
				let attributes = attributes
					.into_iter()
					.map(|a| self.map_attribute(&a))
					.collect::<Vec<_>>();
				let children = self.map_node(*children);
				quote! { Element (
					tag: #tag,
					self_closing: #self_closing,
					attributes: [#(#attributes),*],
					children: #children,
					meta: #meta
				)}
			}
			WebTokens::Component {
				component,
				children,
				tracker,
			} => {
				let ElementTokens { tag, meta, .. } = &component;
				let meta = meta.into_ron_tokens();

				// components disregard all the context and rely on the tracker
				let tracker = tracker.into_ron_tokens();
				let slot_children = self.map_node(*children);
				quote! { Component (
					tracker: #tracker,
					tag: #tag,
					slot_children: #slot_children,
					meta: #meta
				)}
			}
		}
	}


	fn map_attribute(&mut self, attr: &RsxAttributeTokens) -> TokenStream {
		match attr {
			RsxAttributeTokens::Block { block: _, tracker } => {
				let tracker = tracker.into_ron_tokens();
				quote! { Block (#tracker)}
			}
			RsxAttributeTokens::Key { key } => {
				quote! {Key ( key: #key )}
			}
			RsxAttributeTokens::KeyValueLit { key, value } => {
				// ron stringifies all lit values?
				// tbh not sure why we need to do this but it complains need string
				let value = RsxAttributeTokens::lit_to_string(&value);
				quote! { KeyValue (
						key: #key,
						value: #value
						)
				}
			}
			// the attribute is a key value where the value
			// is not an [`Expr::Lit`]
			RsxAttributeTokens::KeyValueExpr {
				key,
				value: _,
				tracker,
			} => {
				let tracker = tracker.into_ron_tokens();
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
