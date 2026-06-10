//! The BSX syntax tree the cursor parser produces, before resolution into a
//! [`DynamicTemplate`](beet_core::prelude::DynamicTemplate).
//!
//! The tree is deliberately close to the markup surface: an element carries its
//! raw tag, its attributes (each a key plus an [`AttrValue`]), and its children.
//! The value grammar ([`DataLiteral`], [`ValueExpr`]) is shared by attribute
//! values, text-position `{..}` blocks, and bare-position spreads. Resolution
//! ([`super::resolve`]) walks this tree; type inference happens there, against
//! the target's `TypeInfo`, not here.

use beet_core::prelude::*;

/// A single node in the BSX tree.
#[derive(Debug, Clone, PartialEq)]
pub enum BsxNode {
	/// `<tag ..>..</tag>` or `<tag ../>`.
	Element(BsxElement),
	/// Literal prose between tags.
	Text(String),
	/// A `{..}` block in text position: a literal or a `#`/`$` reference.
	Expr(ValueExpr),
	/// `<!-- .. -->`.
	Comment(String),
	/// `<!DOCTYPE ..>`.
	Doctype(String),
}

/// An element: a tag, its attributes, and its children.
#[derive(Debug, Clone, PartialEq)]
pub struct BsxElement {
	/// The raw tag text, ie `div`, `MyTemplate`, `path::to::X`, `Slot`.
	pub tag: String,
	/// The attributes in source order.
	pub attributes: Vec<BsxAttribute>,
	/// The child nodes in source order.
	pub children: Vec<BsxNode>,
	/// Whether the tag was self-closing (`<br/>`), so it has no children.
	pub self_closing: bool,
}

/// One attribute on an element.
#[derive(Debug, Clone, PartialEq)]
pub struct BsxAttribute {
	/// The attribute key, eg `class`, `value`, `bx:scope`, `onclick`. Empty for a
	/// bare-position spread `<el {..}>`.
	pub key: String,
	/// What the attribute holds.
	pub value: AttrValue,
}

/// The three things an attribute can hold, position-disambiguated by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
	/// A bare flag with no value, ie `disabled`.
	Flag,
	/// A quoted string literal, ie `class="card"`. Kept distinct from a braced
	/// literal so HTML mode (no value grammar) still accepts string attributes.
	Str(String),
	/// A literal or `#`/`$` reference, from `key={..}` or unbraced `key=#foo=42`.
	Expr(ValueExpr),
	/// A bare-position spread `<el {..}>`: one or more components/templates.
	Spread(SpreadExpr),
}

/// A value in attribute-value or text position: a literal or a reference.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueExpr {
	/// An inline literal (scalar, struct, list, enum), resolved against the
	/// target's type.
	Literal(DataLiteral),
	/// `#field.path`, optionally `=init`, lowering to a
	/// [`FieldRef`](beet_core::prelude::FieldRef).
	FieldRef {
		/// The dotted field path, eg `count`, `user.name`.
		path: FieldPath,
		/// The initializer literal from `=init`, if present.
		init: Option<DataLiteral>,
	},
	/// `$name`, referencing a `bx:ref`-named entity.
	EntityRef(SmolStr),
}

/// A bare-position spread: a typed struct/tuple naming components or templates.
#[derive(Debug, Clone, PartialEq)]
pub enum SpreadExpr {
	/// A single named component/template, eg `{MyComponent{foo:"bar"}}` or
	/// `{MyComponent}`.
	Named(NamedLiteral),
	/// A tuple of named components/templates, eg `{(A, B)}`.
	Tuple(Vec<NamedLiteral>),
}

/// A literal in the BSX value grammar, mirroring the `bsn!` value surface.
#[derive(Debug, Clone, PartialEq)]
pub enum DataLiteral {
	/// A scalar resolved to the field's type, eg `42`, `"text"`, `true`.
	Scalar(Value),
	/// `[a, b, c]`.
	List(Vec<DataLiteral>),
	/// `{ key: value, .. }`.
	Struct(Vec<(String, DataLiteral)>),
	/// An enum variant, eg `Center`, `Rgb(1,2,3)`, `Point { x: 1 }`.
	Enum(NamedLiteral),
	/// A `$name` entity reference nested inside a literal, eg an `Entity`-typed
	/// field of a spread component (`Linked{to:$target}`). Resolved against the
	/// one entity model at build time.
	EntityRef(SmolStr),
}

/// A name plus its fields, used for enum variants (`Center`, `Rgb(..)`) and
/// spread components/templates (`MyComponent { .. }`). The name disambiguates a
/// bare enum variant from a typed component only at resolution, against the
/// target's `TypeInfo`.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedLiteral {
	/// The variant or type name, eg `Center`, `Rgb`, `MyComponent`.
	pub name: String,
	/// The variant/struct fields.
	pub fields: NamedFields,
}

/// The fields of a [`NamedLiteral`]: unit, tuple, or named-struct.
#[derive(Debug, Clone, PartialEq)]
pub enum NamedFields {
	/// No fields, ie a unit variant `Center` or a bare component `MyComponent`.
	Unit,
	/// Positional fields, ie a tuple variant `Rgb(1, 2, 3)`.
	Tuple(Vec<DataLiteral>),
	/// Named fields, ie a struct variant `Point { x: 1, y: 2 }`.
	Struct(Vec<(String, DataLiteral)>),
}
