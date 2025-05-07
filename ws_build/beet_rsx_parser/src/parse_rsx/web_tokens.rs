use super::ElementTokens;
use crate::prelude::*;
use anyhow::Result;
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
	},
	Doctype {
		/// the opening bracket
		value: Spanner<Lt>,
	},
	Comment {
		value: Spanner<LitStr>,
	},
	Text {
		value: Spanner<LitStr>,
	},
	Block {
		value: Spanner<Block>,
	},
	/// An element `<div>` or a component `<MyComponent>`
	Element {
		component: ElementTokens,
		children: Box<WebTokens>,
		self_closing: bool,
	},
}


impl Default for WebTokens {
	fn default() -> Self { Self::Fragment { nodes: Vec::new() } }
}


impl WebTokens {
	pub fn walk_web_tokens<E>(
		&mut self,
		mut visit: impl FnMut(&mut WebTokens) -> Result<(), E>,
	) -> Result<(), E> {
		self.walk_web_tokens_inner(&mut visit)
	}
	fn walk_web_tokens_inner<E>(
		&mut self,
		visit: &mut impl FnMut(&mut WebTokens) -> Result<(), E>,
	) -> Result<(), E> {
		visit(self)?;
		match self {
			WebTokens::Fragment { nodes } => {
				for child in nodes.iter_mut() {
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
	/// Collapse a vector of `WebTokens` into a single `WebTokens`.
	pub fn collapse(nodes: Vec<WebTokens>) -> WebTokens {
		if nodes.len() == 1 {
			nodes.into_iter().next().unwrap()
		} else {
			WebTokens::Fragment { nodes }
		}
	}
}


impl<E> ElementTokensVisitor<E> for WebTokens {
	fn walk_rsx_tokens_inner(
		&mut self,
		visit: &mut impl FnMut(&mut ElementTokens) -> Result<(), E>,
	) -> anyhow::Result<(), E> {
		match self {
			WebTokens::Fragment { nodes } => {
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
