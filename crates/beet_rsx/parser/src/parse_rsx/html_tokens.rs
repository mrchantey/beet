use super::RsxNodeTokens;
use crate::prelude::*;
use anyhow::Result;
use syn::Block;
use syn::LitStr;
use syn::token::Lt;
#[derive(Debug, Clone)]
pub enum HtmlTokens {
	Fragment {
		nodes: Vec<HtmlTokens>,
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
	Element {
		component: RsxNodeTokens,
		children: Box<HtmlTokens>,
		self_closing: bool,
	},
}


impl Default for HtmlTokens {
	fn default() -> Self { Self::Fragment { nodes: Vec::new() } }
}


impl HtmlTokens {
	pub fn walk_html_tokens<E>(
		&mut self,
		mut visit: impl FnMut(&mut HtmlTokens) -> Result<E>,
	) -> Result<()> {
		self.walk_html_tokens_inner(&mut visit)
	}
	fn walk_html_tokens_inner<E>(
		&mut self,
		visit: &mut impl FnMut(&mut HtmlTokens) -> Result<E>,
	) -> Result<()> {
		visit(self)?;
		match self {
			HtmlTokens::Fragment { nodes } => {
				for child in nodes.iter_mut() {
					child.walk_html_tokens_inner(visit)?;
				}
			}
			HtmlTokens::Element { children, .. } => {
				children.walk_html_tokens_inner(visit)?;
			}
			_ => {}
		}
		Ok(())
	}
	/// Collapse a vector of `HtmlTokens` into a single `HtmlTokens`.
	pub fn collapse(nodes: Vec<HtmlTokens>) -> HtmlTokens {
		if nodes.len() == 1 {
			nodes.into_iter().next().unwrap()
		} else {
			HtmlTokens::Fragment { nodes }
		}
	}
}


impl<E> RsxNodeTokensVisitor<E> for HtmlTokens {
	fn walk_rsx_tokens_inner(
		&mut self,
		visit: &mut impl FnMut(&mut RsxNodeTokens) -> Result<(), E>,
	) -> anyhow::Result<(), E> {
		match self {
			HtmlTokens::Fragment { nodes } => {
				for child in nodes.iter_mut() {
					child.walk_rsx_tokens_inner(visit)?;
				}
			}
			HtmlTokens::Element {
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
