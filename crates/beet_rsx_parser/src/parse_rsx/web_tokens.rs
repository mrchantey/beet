use super::ElementTokens;
use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Block;
use syn::Token;

/// [`WebTokens`] is a superset of [`ElementTokens`], and
/// includes several types of information including html, css,
/// wasm code and various template directives related to web rendering
/// like islands.
///
///
/// ## Example inputs:
/// - rsx! macros
/// - mdx files
/// ## Example outputs:
/// - WebNode TokenStream
/// - WebNodeTemplate TokenStream (ron)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WebTokens {
	Fragment {
		nodes: Vec<WebTokens>,
		meta: NodeMeta,
	},
	Doctype {
		/// the opening angle bracket in <!DOCTYPE html>
		value: Token![<],
		meta: NodeMeta,
	},
	Comment {
		value: SpannerOld<String>,
		meta: NodeMeta,
	},
	Text {
		value: SpannerOld<String>,
		meta: NodeMeta,
	},
	Block {
		value: Block,
		meta: NodeMeta,
		tracker: RustyTracker,
	},
	/// An element `<div>`
	Element {
		component: ElementTokens,
		children: Box<WebTokens>,
		self_closing: bool,
	},
	/// A component `<MyComponent>`
	Component {
		component: ElementTokens,
		children: Box<WebTokens>,
		tracker: RustyTracker,
	},
}


impl Default for WebTokens {
	fn default() -> Self {
		Self::Fragment {
			nodes: Vec::new(),
			meta: NodeMeta::default(),
		}
	}
}

impl GetNodeMeta for WebTokens {
	fn meta(&self) -> &NodeMeta {
		match self {
			WebTokens::Fragment { meta, .. } => meta,
			WebTokens::Doctype { meta, .. } => meta,
			WebTokens::Comment { meta, .. } => meta,
			WebTokens::Text { meta, .. } => meta,
			WebTokens::Block { meta, .. } => meta,
			WebTokens::Element { component, .. } => &component.meta,
			WebTokens::Component { component, .. } => &component.meta,
		}
	}
	fn meta_mut(&mut self) -> &mut NodeMeta {
		match self {
			WebTokens::Fragment { meta, .. } => meta,
			WebTokens::Doctype { meta, .. } => meta,
			WebTokens::Comment { meta, .. } => meta,
			WebTokens::Text { meta, .. } => meta,
			WebTokens::Block { meta, .. } => meta,
			WebTokens::Element { component, .. } => &mut component.meta,
			WebTokens::Component { component, .. } => &mut component.meta,
		}
	}
}


impl AsRef<WebTokens> for WebTokens {
	fn as_ref(&self) -> &WebTokens { self }
}

impl WebTokens {
	/// check if this is made up of only [`WebTokens::Fragment`] nodes
	pub fn is_empty(&self) -> bool {
		match self {
			WebTokens::Fragment { nodes, .. } => {
				for node in nodes {
					if !node.is_empty() {
						return false;
					}
				}
				true
			}
			_ => false,
		}
	}
	/// Visit each [`WebTokens`] node in the tree.
	pub fn walk_web_tokens<E>(
		&self,
		mut visit: impl FnMut(&WebTokens) -> Result<(), E>,
	) -> Result<(), E> {
		self.walk_web_tokens_inner(&mut visit)
	}
	fn walk_web_tokens_inner<E>(
		&self,
		visit: &mut impl FnMut(&WebTokens) -> Result<(), E>,
	) -> Result<(), E> {
		visit(self)?;
		match self {
			WebTokens::Fragment { nodes, .. } => {
				for child in nodes.iter() {
					child.walk_web_tokens_inner(visit)?;
				}
			}
			WebTokens::Element { children, .. } => {
				children.walk_web_tokens_inner(visit)?;
			}
			WebTokens::Component { children, .. } => {
				children.walk_web_tokens_inner(visit)?;
			}
			_ => {}
		}
		Ok(())
	}
	/// Mutably visit each [`WebTokens`] node in the tree.
	pub fn walk_web_tokens_mut<E>(
		&mut self,
		mut visit: impl FnMut(&mut WebTokens) -> Result<(), E>,
	) -> Result<(), E> {
		self.walk_web_tokens_mut_inner(&mut visit)
	}
	fn walk_web_tokens_mut_inner<E>(
		&mut self,
		visit: &mut impl FnMut(&mut WebTokens) -> Result<(), E>,
	) -> Result<(), E> {
		visit(self)?;
		match self {
			WebTokens::Fragment { nodes, .. } => {
				for child in nodes.iter_mut() {
					child.walk_web_tokens_mut_inner(visit)?;
				}
			}
			WebTokens::Element { children, .. } => {
				children.walk_web_tokens_mut_inner(visit)?;
			}
			WebTokens::Component { children, .. } => {
				children.walk_web_tokens_mut_inner(visit)?;
			}
			_ => {}
		}
		Ok(())
	}
	// /// Collapse a vector of `WebTokens` into a single `WebTokens`.
	// pub fn collapse(nodes: Vec<WebTokens>) -> WebTokens {
	// 	if nodes.len() == 1 {
	// 		nodes.into_iter().next().unwrap()
	// 	} else {
	// 		WebTokens::Fragment { nodes }
	// 	}
	// }

	/// When testing for equality sometimes we dont want to compare spans and trackers.
	pub fn reset_spans_and_trackers(mut self) -> Self {
		use std::convert::Infallible;

		self.walk_web_tokens_mut::<Infallible>(|node| {
			*node.meta_mut().span_mut() = FileSpan::default();
			match node {
				WebTokens::Block { tracker, .. } => {
					*tracker = RustyTracker::PLACEHOLDER;
				}
				WebTokens::Element { component, .. } => {
					Self::reset_component(component);
				}
				WebTokens::Component {
					tracker, component, ..
				} => {
					*tracker = RustyTracker::PLACEHOLDER;
					Self::reset_component(component);
				}
				_ => {}
			}
			Ok(())
		})
		.ok();
		self
	}
	fn reset_component(component: &mut ElementTokens) {
		component.tag.tokens_span = proc_macro2::Span::call_site();
		*component.meta_mut().span_mut() = FileSpan::default();
		component.attributes.iter_mut().for_each(|attr| {
			attr.reset_spans_and_trackers();
		});
	}
}


impl<E> ElementTokensVisitor<E> for WebTokens {
	fn walk_rsx_tokens_inner(
		&mut self,
		visit: &mut impl FnMut(&mut ElementTokens) -> Result<(), E>,
	) -> anyhow::Result<(), E> {
		self.walk_web_tokens_mut(|node| {
			match node {
				WebTokens::Element { component, .. } => {
					visit(component)?;
				}
				WebTokens::Component { component, .. } => {
					visit(component)?;
				}
				_ => {}
			}
			Ok(())
		})
	}
}


impl RustTokens for WebTokens {
	fn into_rust_tokens(&self) -> TokenStream {
		match self {
			WebTokens::Fragment { nodes, meta } => {
				let nodes = nodes.iter().map(|node| node.into_rust_tokens());
				let meta = meta.into_rust_tokens();
				quote! {
					WebTokens::Fragment {
						nodes: vec![#(#nodes),*],
						meta: #meta,
					}
				}
			}
			WebTokens::Doctype { value, meta } => {
				let meta = meta.into_rust_tokens();
				quote! {
					WebTokens::Doctype {
						value: #value,
						meta: #meta,
					}
				}
			}
			WebTokens::Comment { value, meta } => {
				let meta = meta.into_rust_tokens();
				let value = value.into_rust_tokens();
				quote! {
					WebTokens::Comment {
						value: #value,
						meta: #meta,
					}
				}
			}
			WebTokens::Text { value, meta } => {
				let value = value.into_rust_tokens();
				let meta = meta.into_rust_tokens();
				quote! {
					WebTokens::Text {
						value: #value,
						meta: #meta,
					}
				}
			}
			WebTokens::Block {
				value,
				meta,
				tracker,
			} => {
				let meta = meta.into_rust_tokens();
				let tracker = tracker.into_rust_tokens();
				quote! {
					WebTokens::Block {
						value: #value,
						meta: #meta,
						tracker: #tracker,
					}
				}
			}
			WebTokens::Element {
				component,
				children,
				self_closing,
			} => {
				let component = component.into_rust_tokens();
				let children = children.into_rust_tokens();
				quote! {
					WebTokens::Element {
						component: #component,
						children: Box::new(#children),
						self_closing: #self_closing,
					}
				}
			}
			WebTokens::Component {
				component,
				children,
				tracker,
			} => {
				let component = component.into_rust_tokens();
				let children = children.into_rust_tokens();
				let tracker = tracker.into_rust_tokens();
				quote! {
					WebTokens::Component {
						component: #component,
						children: Box::new(#children),
						tracker: #tracker,
					}
				}
			}
		}
	}
}
