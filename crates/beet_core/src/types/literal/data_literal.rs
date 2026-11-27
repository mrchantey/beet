//! An authored literal: the value model of beet's value grammar, what markup,
//! frontmatter and every other authored source resolve against a type.
use crate::prelude::*;
use alloc::boxed::Box;

/// A literal in the BSX value grammar, beet's own grammar inspired by bevy's
/// `bsn!` value surface.
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
	/// An `@` binding nested inside a literal, ie a binding valued prop of an
	/// [`Instance`], `props={{title:@doc:title}}`, and nowhere else.
	Binding(Box<BindingExpr>),
}

impl DataLiteral {
	/// The plain [`Value`] of this literal, or `None` for a non-literal (an
	/// entity ref carries no inline value).
	///
	/// Used at build time to resolve an authored literal against its target.
	pub fn value(&self) -> Option<Value> {
		match self {
			DataLiteral::Scalar(value) => Some(value.clone()),
			DataLiteral::List(items) => items
				.iter()
				.map(DataLiteral::value)
				.collect::<Option<Vec<_>>>()
				.map(Value::List),
			DataLiteral::Struct(fields) => {
				let mut map = Map::default();
				for (key, item) in fields {
					map.insert(key.clone(), item.value()?);
				}
				Some(Value::Map(map))
			}
			DataLiteral::Enum(named)
				if matches!(named.fields, NamedFields::Unit) =>
			{
				Some(Value::Str(named.name.clone()))
			}
			DataLiteral::Enum(_)
			| DataLiteral::EntityRef(_)
			| DataLiteral::Binding(_) => None,
		}
	}
}

/// A plain value as the literal that authors it, the inverse of
/// [`DataLiteral::value`]: a list as a [`List`](DataLiteral::List), a map as a
/// [`Struct`](DataLiteral::Struct), anything else a
/// [`Scalar`](DataLiteral::Scalar), so a frontmatter list lands in a `Vec`
/// field as a markup `[a, b]` does.
impl From<Value> for DataLiteral {
	fn from(value: Value) -> Self {
		match value {
			Value::List(items) => {
				Self::List(items.into_iter().map(Self::from).collect())
			}
			Value::Map(map) => Self::Struct(
				map.0
					.into_iter()
					.map(|(key, value)| (key, Self::from(value)))
					.collect(),
			),
			other => Self::Scalar(other),
		}
	}
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

impl NamedLiteral {
	/// The component `name` with named `fields`, as `Foo{a:1}` authors it, or
	/// bare, as `Foo`, when there are none.
	pub fn component<'a>(
		name: &str,
		fields: impl IntoIterator<Item = (&'a str, DataLiteral)>,
	) -> Self {
		let fields = fields
			.into_iter()
			.map(|(key, value)| (SmolStr::new(key), value))
			.collect::<Vec<_>>();
		Self {
			name: name.into(),
			fields: match fields.is_empty() {
				true => NamedFields::Unit,
				false => NamedFields::Struct(fields),
			},
		}
	}
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
/// `@entity:PageRoot::DocMeta.title`. `@comp:` (no `@entity:`) binds the
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
	/// The bound resource or component, as written, eg `PackageConfig`.
	pub bound_type: Option<ReflectName>,
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
			bound_type: None,
			field_path: field_path.into_field_path(),
			init: None,
		}
	}
}

/// The source kinds of an `@` binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingSource {
	/// `@doc:` the nearest ancestor user [`Document`](beet_core::prelude::Document).
	Doc,
	/// `@res:` a reflected resource field.
	Res,
	/// `@comp:` a reflected component field.
	Comp,
	/// `@prop:` what the [`Instance`] using the document passed.
	Prop,
}
