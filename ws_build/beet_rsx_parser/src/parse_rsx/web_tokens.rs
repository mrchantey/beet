use super::ElementTokens;
use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use syn::Block;
use syn::LitStr;
use syn::token::Lt;

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
/// - RsxTemplateNode TokenStream (ron)
#[derive(Debug, Clone, Hash)]
pub enum WebTokens {
	Fragment {
		nodes: Vec<WebTokens>,
		meta: NodeMeta,
	},
	Doctype {
		/// the opening bracket
		value: Lt,
		meta: NodeMeta,
	},
	Comment {
		value: LitStr,
		meta: NodeMeta,
	},
	Text {
		value: LitStr,
		meta: NodeMeta,
	},
	Block {
		value: Block,
		meta: NodeMeta,
	},
	/// An element `<div>` or a component `<MyComponent>`
	Element {
		component: ElementTokens,
		children: Box<WebTokens>,
		self_closing: bool,
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
		}
	}
}


impl AsRef<WebTokens> for WebTokens {
	fn as_ref(&self) -> &WebTokens { self }
}

impl WebTokens {
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
}


impl<E> ElementTokensVisitor<E> for WebTokens {
	fn walk_rsx_tokens_inner(
		&mut self,
		visit: &mut impl FnMut(&mut ElementTokens) -> Result<(), E>,
	) -> anyhow::Result<(), E> {
		match self {
			WebTokens::Fragment { nodes, .. } => {
				for child in nodes.iter_mut() {
					child.walk_rsx_tokens_inner(visit)?;
				}
			}
			WebTokens::Element {
				children,
				component,
				..
			} => {
				visit(component)?;
				children.walk_rsx_tokens_inner(visit)?;
			}
			_ => {}
		}
		Ok(())
	}
}
