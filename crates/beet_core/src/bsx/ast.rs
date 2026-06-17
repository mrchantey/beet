//! The BSX syntax tree the cursor parser produces, before resolution into a
//! [`DynamicTemplate`](beet_core::prelude::DynamicTemplate).
//!
//! The tree is deliberately close to the markup surface: an element carries its
//! raw tag, its attributes (each a key plus an [`AttrValue`]), and its children.
//! The value grammar ([`DataLiteral`], [`ValueExpr`]) is shared by attribute
//! values, text-position `{..}` blocks, and bare-position spreads. Resolution
//! ([`super::resolve`]) walks this tree; type inference happens there, against
//! the target's `TypeInfo`, not here.

use crate::prelude::*;

/// A single node in the BSX tree.
#[derive(Debug, Clone, PartialEq)]
pub enum BsxNode {
	/// `<tag ..>..</tag>` or `<tag ../>`.
	Element(BsxElement),
	/// Literal prose between tags.
	Text(String),
	/// A `{..}` block in text position: a literal, an `@` binding, or a `$`
	/// reference.
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

/// What an attribute can hold, position-disambiguated by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
	/// A bare flag with no value, ie `disabled`.
	Flag,
	/// A quoted string literal, ie `class="card"`. Kept distinct from a braced
	/// literal so HTML mode (no value grammar) still accepts string attributes.
	Str(String),
	/// A literal, `@` binding, or `$` reference, from `key={..}` or unbraced
	/// `key=@doc:foo=42`.
	Expr(ValueExpr),
	/// A bare-position spread `<el {..}>`: one or more components/templates.
	Spread(SpreadExpr),
	/// A `bx:<event>` directive's verb call, eg `bx:click=increment{ field: @doc:count }`.
	Verb(VerbCall),
	/// A `bx:style` directive's one-off rule declarations, eg
	/// `bx:style="display=Flex max-width=Rem(40.0)"`. The raw declaration text is
	/// kept verbatim (parsed downstream where the style types live), paired with
	/// the source [`FileSpan`] so the minted inline class is stable across spawns
	/// of the same callsite, the markup twin of `inline_class!`'s `panic::Location`.
	Style {
		/// The raw `prop=value ..` declaration text, parsed downstream.
		source: SmolStr,
		/// The directive's source span, mapped onto the minted inline class.
		span: FileSpan,
	},
}

/// A value in attribute-value or text position: a literal or a reference.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueExpr {
	/// An inline literal (scalar, struct, list, enum), resolved against the
	/// target's type.
	Literal(DataLiteral),
	/// `$name`, referencing a `bx:ref`-named entity.
	EntityRef(SmolStr),
	/// An `@source:path` reactive binding, eg `@doc:count` or
	/// `@res:PackageConfig.title`.
	Binding(BindingExpr),
}

/// An `@` reactive binding: `@source ":" path init?`.
///
/// ```text
/// binding   = "@doc"  ":" fieldpath init?     eg @doc:count, @doc:count=0
///           | "@prop" ":" fieldpath           eg @prop:title
///           | "@res"  ":" ShortTypePath "." fieldpath   eg @res:PackageConfig.title
///           | "@comp" ":" ShortTypePath "." fieldpath   eg @comp:Bar.boo (this entity)
///           | "@entity" ":" RefName "::" ShortTypePath "." fieldpath
///                                              eg @entity:slider::Slider.value
/// fieldpath = a dotted field path, eg count or user.name
/// init      = "=" literal             @doc only, eg {@doc:count=0}
/// ```
///
/// `@entity:` is `@comp` retargeted to a named entity: the [`selector`] names a
/// `bx:ref` entity, or one of the reserved well-known entities `BuildRoot`,
/// `SnippetRoot`, `PageRoot`, `Router` (see `ReservedRef` in the resolver), eg
/// `@entity:PageRoot::ArticleMeta.title`. `@comp:` (no `@entity:`) binds the
/// current entity.
///
/// [`selector`]: Self::selector
#[derive(Debug, Clone, PartialEq)]
pub struct BindingExpr {
	/// What the binding reads from and writes to.
	pub source: BindingSource,
	/// The `@entity:` selector retargeting a component binding to a `bx:ref`-named
	/// entity, or a reserved well-known entity (`BuildRoot`, `SnippetRoot`,
	/// `PageRoot`, `Router`). Only [`BindingSource::Comp`] carries one.
	pub selector: Option<SmolStr>,
	/// The short type path of the bound resource/component, eg `PackageConfig`.
	pub type_path: Option<SmolStr>,
	/// The field path within the source, eg `count`, `user.name`.
	pub field_path: FieldPath,
	/// The `=init` initializer literal, `@doc` only.
	pub init: Option<DataLiteral>,
}

impl BindingExpr {
	/// An `@doc:path` binding without an initializer.
	pub fn doc<M>(field_path: impl IntoFieldPath<M>) -> Self {
		Self {
			source: BindingSource::Doc,
			selector: None,
			type_path: None,
			field_path: field_path.into_field_path(),
			init: None,
		}
	}
}

/// A parsed `verb{ arg: value, .. }` event-verb call, the value of a
/// `bx:<event>` directive (eg `bx:click=increment{ field: @doc:count, amount: 3 }`).
///
/// The verb name resolves against the [`VerbRegistry`](crate::prelude::VerbRegistry)
/// at build time; each named argument is a literal value or an `@` binding,
/// kept distinct so a binding argument resolves to a live source rather than a
/// frozen [`Value`].
#[derive(Debug, Clone, PartialEq)]
pub struct VerbCall {
	/// The verb name, eg `increment`.
	pub verb: SmolStr,
	/// The named arguments in author order.
	pub args: Vec<(SmolStr, VerbArg)>,
}

/// A single argument of a [`VerbCall`]: a literal value or an `@` binding.
#[derive(Debug, Clone, PartialEq)]
pub enum VerbArg {
	/// A literal value, eg `amount: 3`.
	Literal(DataLiteral),
	/// An `@` binding to a live source, eg `field: @doc:count`.
	Binding(BindingExpr),
}

/// The source kinds of an `@` binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingSource {
	/// `@doc:` the nearest ancestor user [`Document`](crate::prelude::Document).
	Doc,
	/// `@res:` a reflected resource field.
	Res,
	/// `@comp:` a reflected component field.
	Comp,
	/// `@prop:` the nearest ancestor template props store.
	Prop,
}

/// A bare-position spread: a typed struct/tuple naming components or templates.
#[derive(Debug, Clone, PartialEq)]
pub enum SpreadExpr {
	/// A single named component/template, eg `{MyComponent{foo:"bar"}}` or
	/// `{MyComponent}`.
	Named(NamedLiteral),
	/// A tuple of components/templates and bindings, eg
	/// `{(Bar{boo:"bazz"}, @comp:Bar.boo)}`.
	Tuple(Vec<SpreadItem>),
}

/// One item of a tuple spread: a named component/template or an `@` binding
/// applied to the same entity.
#[derive(Debug, Clone, PartialEq)]
pub enum SpreadItem {
	/// A named component/template, eg `Bar{boo:"bazz"}`.
	Named(NamedLiteral),
	/// An `@` binding, eg `@comp:Bar.boo`.
	Binding(BindingExpr),
}

/// A literal in the BSX value grammar, mirroring the `bsn!` value surface.
#[derive(Debug, Clone, PartialEq)]
pub enum DataLiteral {
	/// A scalar resolved to the field's type, eg `42`, `"text"`, `true`.
	Scalar(Value),
	/// `[a, b, c]`.
	List(Vec<DataLiteral>),
	/// `{ key: value, .. }`.
	Struct(Vec<(SmolStr, DataLiteral)>),
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
	pub name: SmolStr,
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
	Struct(Vec<(SmolStr, DataLiteral)>),
}
