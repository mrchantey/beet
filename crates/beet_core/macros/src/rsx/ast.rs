//! The lean node tree the [`rsx!`](super) parser produces.
//!
//! Unlike rstml's token-faithful tree, these nodes hold only what the lowering
//! ([`super::lower`]) needs: the node vocabulary plus spans for diagnostics. They
//! carry no `ToTokens` round-trip and no `CustomNode` generic, so a new node kind
//! is a direct variant rather than a plugin trait.
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use proc_macro2::Span;

/// A single node in the markup tree.
pub enum RsxNode {
	/// `<tag ..>..</tag>` or `<tag ../>`.
	Element(RsxElement),
	/// A quoted text literal, ie `"hello"`. Dynamic text is a [`RsxNode::Block`].
	Text(syn::LitStr),
	/// A text-position `{expr}`: a Rust expression lifted into the tree.
	Block(syn::Block),
	/// `<!-- "comment" -->`.
	Comment(String),
	/// `<!DOCTYPE html>`, the `DOCTYPE` keyword stripped.
	Doctype(String),
	/// `<>..</>`.
	Fragment(RsxFragment),
}

/// An element: a name, its attributes, and its children.
pub struct RsxElement {
	/// The tag name, ie `div`, `Foo`, `path::to::Foo`, `Slot`.
	pub name: RsxName,
	/// The attributes in source order.
	pub attributes: Vec<RsxAttr>,
	/// The child nodes in source order.
	pub children: Vec<RsxNode>,
}

/// A `<>..</>` fragment: children with no enclosing element.
pub struct RsxFragment {
	/// The child nodes in source order.
	pub children: Vec<RsxNode>,
}

/// One attribute on an element.
pub enum RsxAttr {
	/// A keyed attribute: `key`, `key=value`.
	Keyed(RsxKeyedAttr),
	/// A bare `{..}` spread: components/templates inserted onto the entity.
	Spread(syn::Block),
}

/// A keyed attribute, ie `class="card"`, `disabled`, `onclick={..}`.
pub struct RsxKeyedAttr {
	/// The attribute key, ie `class`, `disabled`, `bx:slot`.
	pub key: RsxName,
	/// What the key holds.
	pub value: RsxAttrValue,
}

impl RsxKeyedAttr {
	/// The key rendered as a string.
	pub fn key_str(&self) -> String {
		self.key.value()
	}

	/// The value expression, if the attribute has one.
	pub fn value_expr(&self) -> Option<&syn::Expr> {
		match &self.value {
			RsxAttrValue::Expr(expr) => Some(expr),
			RsxAttrValue::None => None,
		}
	}

	/// The value as a displayable literal string, if it is one (`"x"`, `1`,
	/// `true`, ..). Used to read `name=`/`slot=` routing literals.
	pub fn value_literal_string(&self) -> Option<String> {
		self.value_expr().and_then(expr_literal_string)
	}
}

/// What a keyed attribute holds: nothing (a flag) or a value expression.
pub enum RsxAttrValue {
	/// A bare flag, ie `disabled`.
	None,
	/// A value, ie `="card"`, `=foo()`, `={block}`. Held opaque and re-emitted.
	Expr(syn::Expr),
}

/// A tag or attribute name.
///
/// Covers a Rust path (`div`, `Foo`, `path::to::Foo`) and an SGML-style
/// punctuated name (`foo-bar`, `bx:slot`, `data-foo`). The slot/fragment
/// specials are their own nodes, not names.
pub enum RsxName {
	/// A path of `::`-separated idents, ie `div`, `Foo`, `path::to::Foo`.
	Path {
		/// The parsed path, reused directly for component/template tags.
		path: syn::Path,
		/// The span of the first segment, for diagnostics.
		span: Span,
	},
	/// A name punctuated by `-`, `:`, or `.`, ie `foo-bar`, `bx:slot`.
	Punctuated {
		/// The rendered name, ie `foo-bar`.
		value: String,
		/// The span of the first fragment, for diagnostics.
		span: Span,
	},
}

impl RsxName {
	/// The span of the name's first fragment.
	pub fn span(&self) -> Span {
		match self {
			Self::Path { span, .. } | Self::Punctuated { span, .. } => *span,
		}
	}

	/// The name rendered as a string, ie `div`, `path::to::Foo`, `bx:slot`.
	pub fn value(&self) -> String {
		match self {
			Self::Path { path, .. } => path
				.segments
				.iter()
				.map(|seg| seg.ident.to_string())
				.collect::<Vec<_>>()
				.join("::"),
			Self::Punctuated { value, .. } => value.clone(),
		}
	}

	/// The underlying path, for a component/template tag.
	pub fn as_path(&self) -> Option<&syn::Path> {
		match self {
			Self::Path { path, .. } => Some(path),
			Self::Punctuated { .. } => None,
		}
	}
}

/// The displayable string of a literal expression (`"x"`, `'c'`, `1`, `0.5`,
/// `true`), or `None` for any other expression. Adapted from rstml/leptos.
fn expr_literal_string(expr: &syn::Expr) -> Option<String> {
	let syn::Expr::Lit(lit) = expr else {
		return None;
	};
	match &lit.lit {
		syn::Lit::Str(value) => Some(value.value()),
		syn::Lit::Char(value) => Some(value.value().to_string()),
		syn::Lit::Int(value) => Some(value.base10_digits().to_string()),
		syn::Lit::Float(value) => Some(value.base10_digits().to_string()),
		syn::Lit::Bool(value) => Some(value.value.to_string()),
		_ => None,
	}
}
