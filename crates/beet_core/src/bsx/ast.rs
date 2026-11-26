//! The BSX syntax tree the cursor parser produces, before it lowers to a
//! [`SceneBody`].
//!
//! The tree is deliberately close to the markup surface: an element carries its
//! raw tag, its attributes (each a key plus an [`AttrValue`]), and its children.
//! The value grammar ([`DataLiteral`], [`ValueExpr`]) is shared by attribute
//! values, text-position `{..}` blocks, and bare-position spreads. The lowering
//! (`beet_scene`'s `BsxParser`) walks this tree; type inference happens there, against
//! the target's `TypeInfo`, not here.

use super::span::*;
use crate::prelude::*;
use alloc::boxed::Box;

/// A single node in the BSX tree.
#[derive(Debug, Clone, PartialEq)]
pub enum BsxNode {
	/// `<tag ..>..</tag>` or `<tag ../>`.
	Element(BsxElement),
	/// Literal prose between tags.
	Text(Spanned<String>),
	/// A `{..}` block in text position: a literal, an `@` binding, or a `$`
	/// reference.
	Expr(Spanned<ValueExpr>),
	/// `<!-- .. -->`.
	Comment(Spanned<String>),
	/// `<!DOCTYPE ..>`.
	Doctype(Spanned<String>),
	/// `<![CDATA[ .. ]]>`, its content as written.
	CData(Spanned<String>),
	/// `<? .. ?>`, ie the XML declaration, its content as written.
	ProcessingInstruction(Spanned<String>),
	/// A document of another format, already lowered, standing where it was
	/// written, ie a markdown fence a parser reads.
	Embedded(Spanned<ParsedDocument>),
}

/// An element: a tag, its attributes, and its children.
#[derive(Debug, Clone, PartialEq)]
pub struct BsxElement {
	/// The raw tag text, ie `div`, `MyTemplate`. For a
	/// tag-position literal (`<Name("x")/>`, `<LogLike::Message("hi")/>`) this is the
	/// base component name (`Name`, `Log`), with the full literal in
	/// [`tag_literal`](Self::tag_literal).
	pub tag: String,
	/// A tag-position component literal, ie `<Name("Malenia")/>`,
	/// `<LogLike::Message("running")/>`, `<Greet{name:"world"}/>`. Resolved exactly
	/// like the `{..}` spread position, building the component from the literal
	/// rather than over `Default::default()`. `None` for a bare tag like
	/// `<Sequence>`.
	pub tag_literal: Option<NamedLiteral>,
	/// The name its `bx:ref` gives the entity, which keys it in the body.
	pub key: Option<SmolStr>,
	/// The attributes in source order.
	pub attributes: Vec<BsxAttribute>,
	/// The child nodes in source order.
	pub children: Vec<BsxNode>,
	/// Whether the tag was self-closing (`<br/>`), so it has no children.
	pub self_closing: bool,
	/// Where it was written, its opening tag to its closing one.
	pub span: SourceSpan,
}

/// Read access over a parsed tree, for a caller that reads a document without
/// building it, ie an Office package's relationships or a protocol's xml
/// answer. A name is matched by its local part, whatever prefix it was written
/// with, and a prefix is never resolved against a namespace, so a document is
/// read by [`child`](Self::child) and [`text_at`](Self::text_at) rather than a
/// schema, and a field a service adds later is simply not looked at.
impl BsxElement {
	/// The document element of `xml`, read in the XML dialect
	/// ([`BsxParseConfig::xml`]).
	///
	/// ## Errors
	/// Errors when `xml` does not read or holds no element.
	pub fn parse_xml(xml: &str) -> Result<Self> {
		BsxNode::parse_document(xml, &BsxParseConfig::xml())?
			.into_iter()
			.find_map(|node| match node {
				BsxNode::Element(element) => Some(element),
				_ => None,
			})
			.ok_or_else(|| bevyhow!("the xml has no root element"))
	}

	/// The tag without its prefix, ie `p` for `w:p`.
	pub fn local_name(&self) -> &str {
		self.tag
			.split_once(':')
			.map_or(self.tag.as_str(), |(_, local)| local)
	}

	/// The element children, in order.
	pub fn elements(&self) -> impl Iterator<Item = &BsxElement> {
		self.children.iter().filter_map(|node| match node {
			BsxNode::Element(element) => Some(element),
			_ => None,
		})
	}

	/// The element children whose local name is `local`, in order.
	pub fn children_named<'a>(
		&'a self,
		local: &'a str,
	) -> impl Iterator<Item = &'a BsxElement> {
		self.elements()
			.filter(move |element| element.local_name() == local)
	}

	/// The first element child whose local name is `local`.
	pub fn child(&self, local: &str) -> Option<&BsxElement> {
		self.elements()
			.find(|element| element.local_name() == local)
	}

	/// Every descendant element whose local name is `local`, in document
	/// order.
	pub fn descendants_named(&self, local: &str) -> Vec<&BsxElement> {
		let mut out = Vec::new();
		self.collect_named(local, &mut out);
		out
	}

	fn collect_named<'a>(&'a self, local: &str, out: &mut Vec<&'a BsxElement>) {
		for element in self.elements() {
			if element.local_name() == local {
				out.push(element);
			}
			element.collect_named(local, out);
		}
	}

	/// The string value of the attribute written `key`, ie `r:id`.
	pub fn attribute(&self, key: &str) -> Option<&str> {
		self.attributes.iter().find_map(|attribute| {
			match (&attribute.value, attribute.key == key) {
				(AttrValue::Str(value), true) => Some(value.as_str()),
				_ => None,
			}
		})
	}

	/// The string value of the attribute whose local name is `local`,
	/// whatever its prefix, ie `val` for `w:val`.
	pub fn attribute_local(&self, local: &str) -> Option<&str> {
		self.attributes.iter().find_map(|attribute| {
			let name = attribute
				.key
				.split_once(':')
				.map_or(attribute.key.as_str(), |(_, name)| name);
			match (&attribute.value, name == local) {
				(AttrValue::Str(value), true) => Some(value.as_str()),
				_ => None,
			}
		})
	}

	/// The string value of the prefixed attribute whose local name is
	/// `local`, ie `r:id`, never an unprefixed one, ie a slide's own `id`.
	pub fn prefixed_attribute(&self, local: &str) -> Option<&str> {
		self.attributes.iter().find_map(|attribute| {
			let (_, name) = attribute.key.split_once(':')?;
			match (&attribute.value, name == local) {
				(AttrValue::Str(value), true) => Some(value.as_str()),
				_ => None,
			}
		})
	}

	/// The element at the `/` separated `path` of local names below this one.
	pub fn at(&self, path: &str) -> Option<&BsxElement> {
		path.split('/')
			.filter(|name| !name.is_empty())
			.try_fold(self, |element, name| element.child(name))
	}

	/// The first element whose local name is `local` anywhere below this one,
	/// depth first.
	pub fn find(&self, local: &str) -> Option<&BsxElement> {
		self.elements()
			.find_map(|child| match child.local_name() == local {
				true => Some(child),
				false => child.find(local),
			})
	}

	/// The trimmed text of the element at `path` (see [`at`](Self::at)), empty
	/// for an empty element, ie a field's value in a protocol answer.
	pub fn text_at(&self, path: &str) -> Option<&str> {
		self.at(path).map(|element| {
			element
				.children
				.iter()
				.find_map(|node| match node {
					BsxNode::Text(text) | BsxNode::CData(text) => {
						Some(text.trim())
					}
					_ => None,
				})
				.unwrap_or_default()
		})
	}

	/// The text of every descendant text and CDATA node, joined.
	pub fn text(&self) -> String {
		let mut out = String::new();
		self.collect_text(&mut out);
		out
	}

	fn collect_text(&self, out: &mut String) {
		for node in &self.children {
			match node {
				BsxNode::Element(element) => element.collect_text(out),
				BsxNode::Text(text) | BsxNode::CData(text) => {
					out.push_str(text)
				}
				_ => {}
			}
		}
	}
}

/// Reading an element whose directives are rewritten.
impl BsxElement {
	/// The slot routing attribute, HTML's own.
	const SLOT: &'static str = "slot";

	/// Whether the tag resolves by name (a component, template or resource)
	/// rather than as an HTML element: a capitalized tag, or a path whose
	/// final segment is capitalized.
	pub fn is_named(&self) -> bool {
		self.tag
			.rsplit("::")
			.next()
			.unwrap_or(&self.tag)
			.starts_with(|ch: char| ch.is_uppercase())
	}

	/// The attributes that are values of the element's own: neither a spread
	/// nor the `slot` routing it.
	pub fn plain_attributes(&self) -> impl Iterator<Item = &BsxAttribute> {
		self.attributes
			.iter()
			.filter(|attr| !attr.key.is_empty() && attr.key != Self::SLOT)
	}

	/// The [`SlotChild`] its `slot` routes it by, if any.
	pub fn slot_routing(&self) -> Option<SlotChild> {
		self.attributes
			.iter()
			.find(|attr| attr.key == Self::SLOT)
			.map(|attr| match &attr.value {
				AttrValue::Str(name) => SlotChild::named(name.clone()),
				_ => SlotChild::new(),
			})
	}
}

/// Edits a directive's rewrite makes.
impl BsxElement {
	/// An element as a rewrite authors it, self closing when it has no
	/// children.
	pub fn new(
		tag: impl Into<String>,
		attributes: Vec<BsxAttribute>,
		children: Vec<BsxNode>,
	) -> Self {
		Self {
			tag: tag.into(),
			tag_literal: None,
			key: None,
			attributes,
			self_closing: children.is_empty(),
			children,
			span: default(),
		}
	}

	/// This element, written at `span`.
	pub fn with_span(mut self, span: impl Into<SourceSpan>) -> Self {
		self.span = span.into();
		self
	}

	/// Every child a directive wrote at no place of its own takes this
	/// element's, the directive's host.
	pub fn place_children(&mut self) {
		for child in &mut self.children {
			if !child.span().is_known() {
				child.set_span(self.span.clone());
			}
		}
	}

	/// Remove the attribute keyed `key`, answering its value.
	pub fn take_attribute(&mut self, key: &str) -> Option<AttrValue> {
		let index = self.attributes.iter().position(|attr| attr.key == key)?;
		Some(self.attributes.remove(index).value)
	}

	/// Remove the attribute keyed `key`, which must hold a quoted string.
	pub fn take_str_attribute(&mut self, key: &str) -> Result<String> {
		match self.take_attribute(key) {
			Some(AttrValue::Str(value)) => Ok(value),
			Some(_) => bevybail!(
				"`{key}` on `<{}>` expects a quoted string, ie `{key}=\"..\"`",
				self.tag
			),
			None => bevybail!("`<{}>` carries no `{key}`", self.tag),
		}
	}

	/// Append a bare spread of `item`.
	pub fn push_spread(&mut self, item: SpreadItem) {
		self.push_spread_items([item]);
	}

	/// Append a bare spread of `items`: one component as itself, as
	/// `{Foo}`, anything else a tuple, as `{(Foo, @doc:bar)}`.
	pub fn push_spread_items(
		&mut self,
		items: impl IntoIterator<Item = SpreadItem>,
	) {
		let spread = match <[SpreadItem; 1]>::try_from(
			items.into_iter().collect::<Vec<_>>(),
		) {
			Ok([SpreadItem::Named(named)]) => SpreadExpr::Named(named),
			Ok([item]) => SpreadExpr::Tuple(vec![item]),
			Err(items) => SpreadExpr::Tuple(items),
		};
		self.attributes.push(BsxAttribute::spread(spread));
	}
}

impl BsxNode {
	/// The first element of a parsed document, its document element.
	pub fn document_element(nodes: &[BsxNode]) -> Option<&BsxElement> {
		nodes.iter().find_map(|node| match node {
			BsxNode::Element(element) => Some(element),
			_ => None,
		})
	}

	/// Where the node was written.
	pub fn span(&self) -> &SourceSpan {
		match self {
			Self::Element(element) => &element.span,
			Self::Text(text)
			| Self::Comment(text)
			| Self::Doctype(text)
			| Self::CData(text)
			| Self::ProcessingInstruction(text) => &text.span,
			Self::Expr(expr) => &expr.span,
			Self::Embedded(document) => &document.span,
		}
	}

	/// Declare where the node was written.
	pub fn set_span(&mut self, span: SourceSpan) {
		match self {
			Self::Element(element) => element.span = span,
			Self::Text(text)
			| Self::Comment(text)
			| Self::Doctype(text)
			| Self::CData(text)
			| Self::ProcessingInstruction(text) => text.span = span,
			Self::Expr(expr) => expr.span = span,
			Self::Embedded(document) => document.span = span,
		}
	}

	/// Whether the node builds anything at a document's root: a comment or
	/// blank text beside the root element documents the file.
	pub fn is_significant(&self) -> bool {
		match self {
			BsxNode::Comment(_) => false,
			BsxNode::Text(text) => !text.trim().is_empty(),
			_ => true,
		}
	}

	/// The single root element among `nodes`, its comments and blank text
	/// aside, or none when there are several or the one is not an element.
	pub fn single_root(nodes: &[BsxNode]) -> Option<&BsxNode> {
		let mut roots = nodes.iter().filter(|node| node.is_significant());
		match (roots.next(), roots.next()) {
			(Some(root @ BsxNode::Element(_)), None) => Some(root),
			_ => None,
		}
	}
}

/// One attribute on an element.
#[derive(Debug, Clone, PartialEq)]
pub struct BsxAttribute {
	/// The attribute key, eg `class`, `value`, `bx:scope`, `onclick`. Empty for a
	/// bare-position spread `<el {..}>`.
	pub key: String,
	/// What the attribute holds.
	pub value: AttrValue,
	/// Where it was written, its key and value.
	pub span: SourceSpan,
}

impl BsxAttribute {
	/// `key` holding `value`, as a rewrite authors one.
	pub fn new(key: impl Into<String>, value: AttrValue) -> Self {
		Self {
			key: key.into(),
			value,
			span: default(),
		}
	}

	/// This attribute, written at `span`.
	pub fn with_span(mut self, span: impl Into<SourceSpan>) -> Self {
		self.span = span.into();
		self
	}

	/// Whether the value is an `@` binding.
	pub fn is_binding(&self) -> bool {
		matches!(self.value, AttrValue::Expr(ValueExpr::Binding(_)))
	}

	/// A bare-position spread, `<el {..}>`, which has no key.
	pub fn spread(spread: SpreadExpr) -> Self {
		Self::new(String::new(), AttrValue::Spread(spread))
	}
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

impl SpreadItem {
	/// The component `name` with `fields`, see [`NamedLiteral::component`].
	pub fn component<'a>(
		name: &str,
		fields: impl IntoIterator<Item = (&'a str, DataLiteral)>,
	) -> Self {
		Self::Named(NamedLiteral::component(name, fields))
	}
}

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

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A protocol answer reads by local names and paths, its text unescaped
	/// and trimmed, an undeclared prefix read all the same.
	#[crate::test]
	fn reads_an_xml_answer() {
		let xml = BsxElement::parse_xml(
			r#"<?xml version="1.0" encoding="UTF-8"?>
			<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
				<Name>bucket</Name>
				<Contents><Key>a &amp; b</Key><Size>3</Size></Contents>
				<Contents><Key>c&#47;d</Key><Size>4</Size></Contents>
				<s3:IsTruncated/>
				<Note><![CDATA[x < y]]></Note>
			</ListBucketResult>"#,
		)
		.unwrap();
		xml.local_name().xpect_eq("ListBucketResult");
		xml.text_at("Name").xpect_eq(Some("bucket"));
		xml.children_named("Contents")
			.map(|contents| contents.text_at("Key").unwrap().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["a & b".to_string(), "c/d".to_string()]);
		xml.text_at("IsTruncated").xpect_eq(Some(""));
		xml.text_at("Note").xpect_eq(Some("x < y"));
		xml.find("Size").unwrap().text().xpect_eq("3");
		BsxElement::parse_xml("<?xml version=\"1.0\"?>")
			.unwrap_err()
			.to_string()
			.xpect_contains("no root element");
	}

	/// An attribute is found as written, by its local name, or by its local
	/// name behind a prefix alone.
	#[crate::test]
	fn reads_attributes_by_name() {
		let element =
			BsxElement::parse_xml("<w:p a=\"1\" w:val=\"v\"/>").unwrap();
		element.attribute("a").xpect_eq(Some("1"));
		element.attribute_local("val").xpect_eq(Some("v"));
		element.prefixed_attribute("val").xpect_eq(Some("v"));
		element.prefixed_attribute("a").xpect_none();
	}
}
