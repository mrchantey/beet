use crate::prelude::*;
use anyhow::Result;
use proc_macro2::LineColumn;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Block;
use syn::Expr;
use syn::ExprPath;
use syn::LitStr;
use syn::spanned::Spanned;

/// Intermediate representation of an 'element' in an rsx tree.
/// Despite the web terminology, this is also used to represent
/// other types like Bevy entities.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ElementTokens {
	/// the name of the component, ie <MyComponent/>
	pub tag: NameExpr,
	/// fields of the component, ie <MyComponent foo=bar bazz/>
	pub attributes: Vec<RsxAttributeTokens>,
	/// special directives for use by both
	/// parser and WebNode pipelines, ie <MyComponent client:load/>
	pub directives: Vec<TemplateDirectiveTokens>,
}

// used when a recoverable error is emitted
// impl Default for ElementTokens {
// 	fn default() -> Self { Self::fragment(Default::default()) }
// }

impl ElementTokens {
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

/// Visit all [`ElementTokens`] in a tree, the nodes
/// should be visited before children, ie walked in DFS preorder.
pub trait ElementTokensVisitor<E = anyhow::Error> {
	fn walk_rsx_tokens(
		&mut self,
		mut visit: impl FnMut(&mut ElementTokens) -> Result<(), E>,
	) -> Result<(), E> {
		self.walk_rsx_tokens_inner(&mut visit)
	}
	fn walk_rsx_tokens_inner(
		&mut self,
		visit: &mut impl FnMut(&mut ElementTokens) -> Result<(), E>,
	) -> Result<(), E>;
}



#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RsxAttributeTokens {
	/// A block attribute, in jsx this is known as a spread attribute
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Spanner<Spannable> {
	pub value: Spannable,
	/// If the value was created from a token stream
	/// this will be None
	pub loc: Option<SpanLike>,
}

impl<S> AsRef<S> for Spanner<S> {
	fn as_ref(&self) -> &S { &self.value }
}

impl<S: ToTokens> ToTokens for Spanner<S> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		self.value.to_tokens(tokens);
	}
}


/// For non rust tokens that still need a location, ie markdown
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpanLike {
	pub start: LineColumn,
	pub end: LineColumn,
}

impl<S> Spanner<S> {
	pub fn new(value: S) -> Self { Self { value, loc: None } }
}
impl Spanner<LitStr> {
	pub fn new_lit_str(value: impl Into<String>, loc: SpanLike) -> Self {
		Self {
			value: LitStr::new(&value.into(), Span::call_site()),
			loc: Some(loc),
		}
	}
}

impl Spanner<Expr> {
	/// if the value is a string literal return its value
	pub fn try_lit_str(&self) -> Option<String> {
		if let Expr::Lit(expr_lit) = &self.value {
			if let syn::Lit::Str(lit) = &expr_lit.lit {
				return Some(lit.value());
			}
		}
		None
	}
}

impl<Spannable: Spanned> Spanner<Spannable> {
	/// Prefers the location of the value
	/// because if thats set the span will be Callsite
	pub fn start(&self) -> LineColumn {
		if let Some(loc) = &self.loc {
			loc.start.clone()
		} else {
			self.value.span().start()
		}
	}
	/// Prefers the location of the value
	/// because if thats set the span will be Callsite
	pub fn end(&self) -> LineColumn {
		if let Some(loc) = &self.loc {
			loc.end.clone()
		} else {
			self.value.span().end()
		}
	}
}

impl<S> From<S> for Spanner<S> {
	fn from(value: S) -> Self { Self::new(value) }
}



// impl<T, S, C> From<T> for Spanner<S, C>
// where
// 	T: Into<S>,
// {
// 	fn from(value: S) -> Self { Spanner::Spanned { value } }
// }


/// A restricted subtype of [`Expr`], often created by [`rstml::node::NodeName`]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NameExpr {
	/// A name that must be a string because its not a valid path expression,
	/// like <foo-bar/>
	LitStr(Spanner<LitStr>),
	/// A valid path expression like `my_component::MyComponent`
	ExprPath(Spanner<ExprPath>),
}

impl NameExpr {
	/// Returns the inner span, this will be [`Span::call_site`] if the value was
	/// created from strings not tokens, ie a markdown file
	pub fn span(&self) -> Span {
		match self {
			NameExpr::LitStr(value) => value.value.span(),
			NameExpr::ExprPath(value) => value.value.span(),
		}
	}
}


/// force expr into string literal
impl ToString for NameExpr {
	fn to_string(&self) -> String {
		match self {
			NameExpr::LitStr(value) => value.value.value(),
			NameExpr::ExprPath(value) => {
				value.value.to_token_stream().to_string()
			}
		}
	}
}

impl ToTokens for NameExpr {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		match self {
			NameExpr::ExprPath(expr) => expr.to_tokens(tokens),
			NameExpr::LitStr(string) => string.to_tokens(tokens),
		}
	}
}


impl Into<NameExpr> for String {
	fn into(self) -> NameExpr {
		NameExpr::LitStr(Spanner::new(LitStr::new(&self, Span::call_site())))
	}
}
