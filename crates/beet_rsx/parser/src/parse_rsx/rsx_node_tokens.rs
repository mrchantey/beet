use crate::prelude::*;
use anyhow::Result;
use proc_macro2::LineColumn;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Block;
use syn::Expr;
use syn::ExprBlock;
use syn::ExprPath;
use syn::LitStr;
use syn::spanned::Spanned;


/// Visit all [`RsxNodeTokens`] in a tree, the nodes
/// should be visited before children are walked in DFS preorder.
pub trait RsxNodeTokensVisitor<E = anyhow::Error> {
	fn walk_rsx_tokens(
		&mut self,
		mut visit: impl FnMut(&mut RsxNodeTokens) -> Result<(), E>,
	) -> Result<(), E> {
		self.walk_rsx_tokens_inner(&mut visit)
	}
	fn walk_rsx_tokens_inner(
		&mut self,
		visit: &mut impl FnMut(&mut RsxNodeTokens) -> Result<(), E>,
	) -> Result<(), E>;
}


/// Intermediate representation of an RSX Node.
///
/// The tag is used to identify the node,
/// parsers may use it to extend the node types, ie
/// `tag: "fragment"`, tag: "doctype" etc.
#[derive(Debug, Clone)]
pub struct RsxNodeTokens {
	/// the name of the component, ie <MyComponent/>
	pub tag: NameExpr,
	/// All tokens involved in the definition of the component,
	/// but not its children or closing tags.
	/// In Rstml this is the `OpenTag`
	/// Maybe this should be a string or just a hash, so we get Eq back
	pub tokens: TokenStream,
	/// fields of the component, ie <MyComponent foo=bar bazz/>
	pub attributes: Vec<RsxAttributeTokens>,
	/// special directives for use by both
	/// parser and RsxNode pipelines, ie <MyComponent client:load/>
	pub directives: Vec<TemplateDirectiveTokens>,
}

// used when a recoverable error is emitted
// impl Default for RsxNodeTokens {
// 	fn default() -> Self { Self::fragment(Default::default()) }
// }

impl RsxNodeTokens {
	// pub fn new(tag: impl Into<NameExpr>) -> Self {
	// 	Self {
	// 		tag: tag.into(),
	// 		attributes: Vec::new(),
	// 		directives: Vec::new(),
	// 	}
	// }
	// pub fn string_spanned(
	// 	name: impl Into<String>,
	// 	span: &impl Spanned,
	// ) -> Self {
	// 	Self::new(NameExpr::string_spanned(name, span))
	// }
	// pub fn with_attribute(
	// 	mut self,
	// 	attribute: impl Into<RsxAttributeTokens>,
	// ) -> Self {
	// 	self.attributes.push(attribute.into());
	// 	self
	// }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RsxAttributeTokens {
	/// A block attribute
	Block { block: Spanner<Block> },
	/// A key attribute created by [`TokenStream`]
	Key { key: NameExpr },
	/// A key value attribute created by [`TokenStream`]
	KeyValue { key: NameExpr, value: Spanner<Expr> },
}

impl RsxAttributeTokens {
	pub fn key_value(
		key: impl Into<NameExpr>,
		value: impl Into<Spanner<Expr>>,
	) -> Self {
		Self::KeyValue {
			key: key.into(),
			value: value.into(),
		}
	}
}

// #[derive(Debug, Clone)]
// pub enum SpanOrLoc {
// 	Span(Span),
// 	Location { start: LineColumn, end: LineColumn },
// }

/// A value whose location can be retrieved either
/// from the token stream or from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spanner<Spannable, Custom = String> {
	Spanned {
		value: Spannable,
	},
	Custom {
		value: Custom,
		start: LineColumn,
		end: LineColumn,
	},
}


impl<S: ToTokens, C: ToString> Spanner<S, C> {
	/// if the value is a token stream, convert it to a string
	/// otherwise, convert it to a string
	pub fn to_tokens_or_string(&self) -> String {
		match self {
			Spanner::Spanned { value } => value.to_token_stream().to_string(),
			Spanner::Custom { value, .. } => value.to_string(),
		}
	}
}
impl<C: ToString> Spanner<Expr, C> {
	/// if the value is a token stream, convert it to a string
	/// otherwise, convert it to a string
	pub fn try_lit_str(&self) -> Option<String> {
		match self {
			Spanner::Spanned { value } => {
				if let Expr::Lit(expr_lit) = value {
					if let syn::Lit::Str(ref lit) = expr_lit.lit {
						return Some(lit.value());
					}
				}
				None
			}
			Spanner::Custom { value, .. } => Some(value.to_string()),
		}
	}
}
impl<C: ToString> Spanner<LitStr, C> {
	/// if the value is a token stream, convert it to a string
	/// otherwise, convert it to a string
	pub fn to_string(&self) -> String {
		match self {
			Spanner::Spanned { value } => value.value(),
			Spanner::Custom { value, .. } => value.to_string(),
		}
	}
}

impl<S: ToString, C: ToString> ToString for Spanner<S, C> {
	fn to_string(&self) -> String {
		match self {
			Spanner::Spanned { value } => value.to_string(),
			Spanner::Custom { value, .. } => value.to_string(),
		}
	}
}


impl<S: ToTokens, C: ToTokens> ToTokens for Spanner<S, C> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		match self {
			Spanner::Spanned { value } => value.to_tokens(tokens),
			Spanner::Custom { value, .. } => value.to_tokens(tokens),
		}
	}
}

impl<Spannable: Spanned> Spanner<Spannable> {
	pub fn start(&self) -> LineColumn {
		match self {
			Spanner::Spanned { value } => value.span().start(),
			Spanner::Custom { start, .. } => start.clone(),
		}
	}
	pub fn end(&self) -> LineColumn {
		match self {
			Spanner::Spanned { value } => value.span().end(),
			Spanner::Custom { end, .. } => end.clone(),
		}
	}
}


impl<S, C> From<S> for Spanner<S, C> {
	fn from(value: S) -> Self { Spanner::Spanned { value } }
}
impl<C> From<LitStr> for Spanner<Expr, C> {
	fn from(value: LitStr) -> Self {
		Spanner::Spanned {
			value: Expr::Lit(syn::ExprLit {
				lit: value.into(),
				attrs: Vec::new(),
			}),
		}
	}
}
impl<C> From<Block> for Spanner<Expr, C> {
	fn from(value: Block) -> Self {
		Spanner::Spanned {
			value: Expr::Block(ExprBlock {
				block: value,
				label: None,
				attrs: Vec::new(),
			}),
		}
	}
}

impl<C> From<&str> for Spanner<Expr, C> {
	fn from(value: &str) -> Self {
		Spanner::Spanned {
			value: Expr::Lit(syn::ExprLit {
				lit: syn::Lit::Str(syn::LitStr::new(
					value,
					proc_macro2::Span::call_site(),
				)),
				attrs: Vec::new(),
			}),
		}
	}
}




// impl<T, S, C> From<T> for Spanner<S, C>
// where
// 	T: Into<S>,
// {
// 	fn from(value: S) -> Self { Spanner::Spanned { value } }
// }


impl<S, C> Spanner<S, C> {
	pub fn new_spanned(value: S) -> Self { Spanner::Spanned { value } }
	pub fn new_custom_spanned(
		value: impl Into<C>,
		span: &impl Spanned,
	) -> Self {
		Self::Custom {
			value: value.into(),
			start: span.span().start(),
			end: span.span().end(),
		}
	}
	pub fn new_custom(value: C, start: LineColumn, end: LineColumn) -> Self {
		Spanner::Custom { value, start, end }
	}
}

/// A restricted subtype of [`Expr`], often created by [`rstml::node::NodeName`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameExpr {
	String(Spanner<LitStr>),
	/// A valid path expression
	ExprPath(Spanner<ExprPath>),
	Block(Spanner<Block>),
}

impl NameExpr {
	pub fn string_spanned(
		value: impl Into<String>,
		span: &impl Spanned,
	) -> Self {
		NameExpr::String(Spanner::new_custom_spanned(value, span))
	}

	/// If the variant would be fairly represented as a string,
	/// return the string value, otherwise return None.
	/// [`ExprPath`] and [`String`] are valid, but [`Block`] is not.
	pub fn try_lit_str(&self) -> Option<String> {
		match self {
			NameExpr::String(value) => Some(value.to_tokens_or_string()),
			NameExpr::ExprPath(value) => Some(value.to_tokens_or_string()),
			NameExpr::Block(_) => None,
		}
	}
}

impl Into<NameExpr> for &str {
	fn into(self) -> NameExpr { NameExpr::string_spanned(self, &self) }
}


impl ToTokens for NameExpr {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		match self {
			NameExpr::ExprPath(expr) => expr.to_tokens(tokens),
			NameExpr::Block(block) => block.to_tokens(tokens),
			NameExpr::String(string) => string.to_tokens(tokens),
		}
	}
}
