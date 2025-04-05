use crate::prelude::*;
use proc_macro2::LineColumn;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Block;
use syn::Expr;
use syn::ExprPath;
use syn::LitStr;
use syn::spanned::Spanned;


/// Intermediate representation of an RSX Node.
#[derive(Debug, Clone)]
pub enum NodeTokens<C> {
	/// A fragment node, containing more fragments.
	/// ie `rsx!(<>"foo"</>)`
	Fragment {
		nodes: Vec<NodeTokens<C>>,
		directives: Vec<TemplateDirectiveTokens>,
	},
	/// A text node, containing a string.
	/// ie `rsx!("foo")`
	Text {
		text: String,
		directives: Vec<TemplateDirectiveTokens>,
	},
	/// A block node, containing a block of code
	/// ie `rsx!({ foo })`
	Block {
		block: Spanner<Block>,
		directives: Vec<TemplateDirectiveTokens>,
	},
	/// A component node, ie `rsx!{<MyComponent/>}`
	Component {
		/// the name of the component, ie <MyComponent/>
		tag: NameExpr,
		/// fields of the component, ie <MyComponent foo=bar bazz/>
		attributes: Vec<RsxAttributeTokens>,
		/// special directives, ie <MyComponent client:load/>
		directives: Vec<TemplateDirectiveTokens>,
		/// the children of the component, ie <MyComponent>foo</MyComponent>
		children: Box<NodeTokens<C>>,
	},
	Custom(C),
}

/// used when a recoverable error is emitted
impl<C> Default for NodeTokens<C> {
	fn default() -> Self {
		NodeTokens::Fragment {
			nodes: Vec::new(),
			directives: Vec::new(),
		}
	}
}
/// A value whose location can be retrieved either
/// from the token stream or from a string
#[derive(Debug, Clone)]
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

impl<C: ToTokens> ToTokens for Spanner<C> {
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


impl<C: AsRef<Vec<TemplateDirectiveTokens>>> AsRef<Vec<TemplateDirectiveTokens>>
	for NodeTokens<C>
{
	fn as_ref(&self) -> &Vec<TemplateDirectiveTokens> {
		match self {
			NodeTokens::Fragment { directives, .. }
			| NodeTokens::Text { directives, .. }
			| NodeTokens::Block { directives, .. }
			| NodeTokens::Component { directives, .. } => directives,
			NodeTokens::Custom(c) => c.as_ref(),
		}
	}
}

#[derive(Debug, Clone)]
pub enum RsxAttributeTokens {
	/// A block attribute
	Block { block: Spanner<Block> },
	/// A key attribute created by [`TokenStream`]
	Key { key: NameExpr },
	/// A key value attribute created by [`TokenStream`]
	KeyValue { key: NameExpr, value: Spanner<Expr> },
}

/// A restricted subtype of [`Expr`], often created by [`rstml::node::NodeName`]
#[derive(Debug, Clone)]
pub enum NameExpr {
	/// A valid path expression
	ExprPath(Spanner<ExprPath>),
	Block(Spanner<Block>),
	String(Spanner<LitStr>),
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
