//! Frontmatter: a document's leading metadata block, YAML between `---` fences
//! or TOML between `+++`, read and written through serde
//! ([`MediaType::Yaml`], [`MediaType::Toml`]).
//!
//! The block is the markdown surface for a document's root component
//! declarations exactly as a root spread is the BSX surface, so both resolve
//! through one reflect path rather than a hand-written key mapping. Its
//! unsectioned keys declare the document's [`FrontmatterType`], [`PageMeta`]
//! by default, and each table (`[Section]` in TOML, a key holding a map in
//! YAML) the component it names.
//!
//! What is frontmatter's own is small: the fences, the unsectioned keys
//! declaring the document's [`FrontmatterType`], a section naming the
//! component its keys declare, and default fields left out when writing. A
//! component is written into a block when its type data says it is
//! frontmatter, `#[reflect(Frontmatter)]` ([`ReflectFrontmatter`]), and a
//! written block is TOML.

use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "serde")]
use bevy::reflect::CreateTypeData;
use bevy::reflect::PartialReflect;
#[cfg(feature = "serde")]
use bevy::reflect::TypeRegistry;
use bevy::reflect::list::DynamicList;
use bevy::reflect::structs::DynamicStruct;

/// Parsed frontmatter metadata from a YAML or TOML block.
///
/// Inserted on the root entity when frontmatter is present in the markdown
/// source. The `value` field is a [`DynamicStruct`] built from the unsectioned
/// key-value pairs, for reflection-based access by a document's own systems; the
/// route-metadata path instead reads [`declarations`](Self::declarations).
///
/// ## Example
/// ```rust
/// # use beet_ui::prelude::*;
/// # use beet_core::prelude::*;
/// let fm = Frontmatter::parse("title: Hello\nauthor: World", FrontmatterKind::Yaml).unwrap();
/// fm.kind.xpect_eq(FrontmatterKind::Yaml);
/// ```
#[derive(Debug, Component)]
pub struct Frontmatter {
	/// The parsed metadata as a dynamic struct for reflection.
	pub value: DynamicStruct,
	/// The frontmatter format that was parsed.
	pub kind: FrontmatterKind,
	/// The parsed pairs grouped by the component their section names, in source
	/// order. Unsectioned pairs carry `None` and target the document's
	/// [`FrontmatterType`].
	sections: Vec<FrontmatterSection>,
}

/// The component a document's unsectioned frontmatter keys declare, resolved by
/// self-or-ancestor from whichever entity discovered the document.
///
/// Defaults to [`PageMeta`], the general page metadata beet ships. A site
/// declaring its own names it on the dir that discovers those documents,
/// `<RoutesDir src="routes" {FrontmatterType{component:"MyMeta"}}/>`, and a
/// section names a component explicitly, winning over this default.
///
/// A component is frontmatter when its type data says so,
/// `#[reflect(Component, Default, Frontmatter)]` beside its `Serialize`
/// derive, which [`ReflectFrontmatter`] reads: the markdown renderer asked
/// for frontmatter writes every such component a root carries, this one's
/// keys unsectioned and every other as its own `[Component]` table, and the
/// parser reads each back through the section naming it. A downstream
/// component declares itself the same way and registers its type.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct FrontmatterType {
	/// Short type path of the component, resolved against the type registry
	/// exactly as a BSX tag is.
	pub component: SmolStr,
}

impl Default for FrontmatterType {
	fn default() -> Self {
		Self {
			component: type_ext::short_name::<PageMeta>(),
		}
	}
}

/// Type data marking a component as frontmatter, see [`FrontmatterType`]:
/// derived by `#[reflect(Frontmatter)]` on a `Component + Default + Serialize`
/// type, it reads the fields a written block declares, those the component's
/// default does not hold, in field order.
#[cfg(feature = "serde")]
#[derive(Clone)]
pub struct ReflectFrontmatter {
	/// The fields of the component on an entity its default does not hold,
	/// `None` when the entity lacks it.
	fields: fn(EntityRef) -> Option<Result<Vec<(SmolStr, Value)>>>,
}

#[cfg(feature = "serde")]
impl<T: Component + Default + serde::Serialize> CreateTypeData<T>
	for ReflectFrontmatter
{
	fn create_type_data(_input: ()) -> Self {
		Self {
			fields: |entity| entity.get::<T>().map(Frontmatter::fields_of),
		}
	}
}

/// One `[Component]` group of a frontmatter block.
#[derive(Debug)]
struct FrontmatterSection {
	/// The component this group's keys declare, `None` for the unsectioned
	/// group targeting the document's [`FrontmatterType`].
	component: Option<SmolStr>,
	pairs: Vec<(String, Value)>,
}

/// The format of a frontmatter metadata block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrontmatterKind {
	/// YAML frontmatter delimited by `---`.
	Yaml,
	/// TOML frontmatter delimited by `+++`.
	Toml,
}

impl FrontmatterKind {
	/// The media type the block is written in.
	pub fn media_type(&self) -> MediaType {
		match self {
			Self::Yaml => MediaType::Yaml,
			Self::Toml => MediaType::Toml,
		}
	}
}

impl Frontmatter {
	/// The key the `toml` crate carries a datetime under, which a block's
	/// `created = 2025-07-11` reads as.
	const TOML_DATETIME: &str = "$__toml_private_datetime";

	/// Extract and parse a leading frontmatter block from a raw source: a
	/// `---` fence parses as YAML, a `+++` fence as TOML. Returns `None` when
	/// the source has no leading fence.
	///
	/// The full markdown parser extracts frontmatter as part of its pass; this
	/// standalone form serves scan-time metadata readers (eg route discovery)
	/// that must not pay for a content parse.
	pub fn extract(source: &str) -> Result<Option<Self>> {
		let trimmed = source.trim_start();
		let (fence, kind) = match trimmed {
			s if s.starts_with("---") => ("---", FrontmatterKind::Yaml),
			s if s.starts_with("+++") => ("+++", FrontmatterKind::Toml),
			_ => return Ok(None),
		};
		// the fence must be a whole line, ie not a thematic break `--- foo`
		let body = &trimmed[fence.len()..];
		if !body.starts_with('\n') && !body.starts_with("\r\n") {
			return Ok(None);
		}
		let Some(end) = body.find(&format!("\n{fence}")) else {
			return Ok(None);
		};
		Self::parse(&body[..end], kind).map(Some)
	}

	/// Parse a raw frontmatter block, the text between its fences, through
	/// serde in the format `kind` names: its keys holding a map are sections,
	/// the rest unsectioned.
	pub fn parse(content: &str, kind: FrontmatterKind) -> Result<Self> {
		let value = match content.trim().is_empty() {
			true => Value::Map(default()),
			false => {
				kind.media_type().deserialize::<Value>(content.as_bytes())?
			}
		};
		let Value::Map(map) = Self::normalize(value) else {
			bevybail!("a frontmatter block is a map of keys");
		};
		let mut sections = vec![FrontmatterSection {
			component: None,
			pairs: Vec::new(),
		}];
		for (key, value) in map.0 {
			match value {
				Value::Map(table) => sections.push(FrontmatterSection {
					component: Some(key),
					pairs: table
						.0
						.into_iter()
						.map(|(key, value)| (key.to_string(), value))
						.collect(),
				}),
				value => sections[0].pairs.push((key.to_string(), value)),
			}
		}
		// the reflect surface mirrors the default component's keys, ie the
		// unsectioned ones; a section is read through `declarations`.
		let value = sections[0].pairs.clone().xmap(build_dynamic_struct)?;
		Ok(Self {
			value,
			kind,
			sections,
		})
	}

	/// A value typed alike whichever format wrote it: a TOML datetime, which
	/// the `toml` crate hands serde as a one-key map, as the text it was
	/// written as, so it coerces like a quoted date, and a whole number that
	/// is not negative unsigned, as the YAML subset reads it.
	fn normalize(value: Value) -> Value {
		match value {
			Value::Map(map) => match map.0.get(Self::TOML_DATETIME) {
				Some(Value::Str(text)) if map.0.len() == 1 => {
					Value::Str(text.clone())
				}
				_ => Value::Map(Map(map
					.0
					.into_iter()
					.map(|(key, value)| (key, Self::normalize(value)))
					.collect())),
			},
			Value::List(items) => {
				Value::List(items.into_iter().map(Self::normalize).collect())
			}
			Value::Int(number) if number >= 0 => Value::Uint(number as u64),
			value => value,
		}
	}

	/// This block as document root declarations: each section names its own
	/// component by short type path, and every unsectioned key declares
	/// `default_component` (the document's [`FrontmatterType`]).
	///
	/// The one lowering, so a frontmatter key coerces to its field type through
	/// the same [`DataLiteral`] rules a BSX spread does — `created = "2025-07-11"`
	/// becomes a [`Date`], `order = 2` a `u32`. A key with no value declares
	/// nothing and is dropped rather than resolving to null.
	pub fn declarations(&self, default_component: &str) -> RootDeclarations {
		self.sections
			.iter()
			.filter_map(|section| {
				let fields = section
					.pairs
					.iter()
					.filter(|(_, value)| !matches!(value, Value::Null))
					.map(|(key, value)| {
						(SmolStr::new(key), DataLiteral::from(value.clone()))
					})
					.collect::<Vec<_>>();
				(!fields.is_empty()).then(|| NamedLiteral {
					name: section
						.component
						.clone()
						.unwrap_or_else(|| SmolStr::new(default_component)),
					fields: NamedFields::Struct(fields),
				})
			})
			.collect::<Vec<_>>()
			.xmap(RootDeclarations)
	}

	/// The TOML block declaring every frontmatter component `entity` carries,
	/// the inverse of [`extract`](Self::extract) and
	/// [`declarations`](Self::declarations): `default_component`, the
	/// document's [`FrontmatterType`], unsectioned, every other as its own
	/// `[Component]` table in name order, each with only the fields its
	/// default does not hold, in field order. `None` when no field is
	/// written, as in a world registering no frontmatter type.
	#[cfg(feature = "serde")]
	pub fn write(
		world: &World,
		entity: Entity,
		default_component: &str,
	) -> Result<Option<String>> {
		let Some(registry) = world.get_resource::<AppTypeRegistry>() else {
			return Ok(None);
		};
		let mut block =
			Self::block(world, entity, &registry.read(), default_component)?;
		block.sections.retain(|(_, fields)| !fields.is_empty());
		if block.fields.is_empty() && block.sections.is_empty() {
			return Ok(None);
		}
		let text = MediaType::Toml
			.serialize_with_options(&block, SerializeOptions {
				pretty: false,
			})?;
		format!("+++\n{}+++\n", String::from_utf8(text)?)
			.xsome()
			.xok()
	}

	/// What [`write`](Self::write) writes, read off `entity`'s frontmatter
	/// components.
	#[cfg(feature = "serde")]
	fn block(
		world: &World,
		entity: Entity,
		registry: &TypeRegistry,
		default_component: &str,
	) -> Result<WrittenBlock> {
		let mut block = WrittenBlock::default();
		for (_, info) in world.inspect_entity(entity)? {
			let Some(registration) =
				info.type_id().and_then(|type_id| registry.get(type_id))
			else {
				continue;
			};
			let Some(frontmatter) = registration.data::<ReflectFrontmatter>()
			else {
				continue;
			};
			let Some(fields) = (frontmatter.fields)(world.entity(entity))
			else {
				continue;
			};
			let name = SmolStr::new(
				registration.type_info().type_path_table().short_path(),
			);
			match name == default_component {
				true => block.fields = fields?,
				false => block.sections.push((name, fields?)),
			}
		}
		block.sections.sort_by(|left, right| left.0.cmp(&right.0));
		block.xok()
	}

	/// The fields of `component` its default does not hold, in field order,
	/// an absent value left out since a block has no spelling for it.
	#[cfg(feature = "serde")]
	fn fields_of<T: Default + serde::Serialize>(
		component: &T,
	) -> Result<Vec<(SmolStr, Value)>> {
		let default = Value::from_serde(T::default())?;
		Value::from_serde(component)?
			.as_map()?
			.into_iter()
			.filter(|(key, value)| {
				!value.is_null() && default.get(key) != Some(*value)
			})
			.map(|(key, value)| (key.clone(), value.clone()))
			.collect::<Vec<_>>()
			.xok()
	}

	/// Get a string field from the frontmatter by name.
	///
	/// Returns `None` if the field does not exist or is not a string.
	pub fn get_str(&self, key: &str) -> Option<&str> {
		self.value
			.field(key)
			.and_then(|field| field.try_downcast_ref::<String>())
			.map(|s| s.as_str())
	}

	/// Get a boolean field from the frontmatter by name.
	///
	/// Returns `None` if the field does not exist or is not a bool.
	pub fn get_bool(&self, key: &str) -> Option<bool> {
		self.value
			.field(key)
			.and_then(|field| field.try_downcast_ref::<bool>())
			.copied()
	}

	/// Get an unsigned-integer field from the frontmatter by name.
	///
	/// Unsigned scalars parse to [`Value::Uint`](beet_core::prelude::Value::Uint)
	/// (stored as `u64`); returns `None` if the field is missing or non-uint.
	pub fn get_uint(&self, key: &str) -> Option<u64> {
		self.value
			.field(key)
			.and_then(|field| field.try_downcast_ref::<u64>())
			.copied()
	}
}

/// A block as [`Frontmatter::write`] writes it, its entries in their own
/// order where a [`Value`] map sorts its keys: the unsectioned fields, then
/// each section's table.
#[cfg(feature = "serde")]
#[derive(Default)]
struct WrittenBlock {
	fields: Vec<(SmolStr, Value)>,
	sections: Vec<(SmolStr, Vec<(SmolStr, Value)>)>,
}

/// A table's fields in their own order.
#[cfg(feature = "serde")]
struct InOrder<'a>(&'a [(SmolStr, Value)]);

#[cfg(feature = "serde")]
impl serde::Serialize for InOrder<'_> {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		use serde::ser::SerializeMap;
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (key, value) in self.0 {
			map.serialize_entry(key.as_str(), value)?;
		}
		map.end()
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for WrittenBlock {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		use serde::ser::SerializeMap;
		let mut map = serializer
			.serialize_map(Some(self.fields.len() + self.sections.len()))?;
		for (key, value) in &self.fields {
			map.serialize_entry(key.as_str(), value)?;
		}
		for (name, fields) in &self.sections {
			map.serialize_entry(name.as_str(), &InOrder(fields))?;
		}
		map.end()
	}
}

/// Build a [`DynamicStruct`] from a list of key-value pairs.
fn build_dynamic_struct(pairs: Vec<(String, Value)>) -> Result<DynamicStruct> {
	let mut dynamic = DynamicStruct::default();
	for (key, value) in pairs {
		let field = reflect_value(&key, value)?;
		dynamic.insert_boxed(key, field);
	}
	dynamic.xok()
}

/// One frontmatter value as a reflect value: a scalar as its rust type, a list
/// as a [`DynamicList`] of them. A map belongs to a section.
fn reflect_value(key: &str, value: Value) -> Result<Box<dyn PartialReflect>> {
	match value {
		Value::Null => Box::new(()) as Box<dyn PartialReflect>,
		Value::Bool(val) => Box::new(val),
		Value::Int(val) => Box::new(val),
		Value::Uint(val) => Box::new(val),
		Value::Float(val) => Box::new(val),
		Value::Str(val) => Box::new(val.to_string()),
		Value::Bytes(val) => Box::new(val),
		Value::List(items) => Box::new(
			items
				.into_iter()
				.map(|item| reflect_value(key, item))
				.collect::<Result<Vec<_>>>()?
				.into_iter()
				.collect::<DynamicList>(),
		),
		Value::Map(_) => {
			bevybail!("Unsupported map value for frontmatter key '{key}'")
		}
	}
	.xok()
}

#[cfg(test)]
mod test {
	use super::*;

	/// The unsectioned pairs of a block, ie the group targeting the default
	/// component.
	fn pairs(content: &str, kind: FrontmatterKind) -> Vec<(String, Value)> {
		Frontmatter::parse(content, kind)
			.unwrap()
			.sections
			.into_iter()
			.filter(|section| section.component.is_none())
			.flat_map(|section| section.pairs)
			.collect()
	}

	/// Both formats read through serde to the same pairs in the order they
	/// were written, scalars typed and lists in either YAML spelling.
	#[beet_core::test]
	fn reads_both_formats() {
		let expected = vec![
			("title".to_string(), Value::str("Hello, World")),
			("count".into(), Value::Uint(42)),
			("draft".into(), Value::Bool(true)),
			(
				"tags".into(),
				Value::List(vec![Value::str("ecs"), Value::str("web, http")]),
			),
		];
		pairs(
			"title: \"Hello, World\"\ncount: 42 # a comment\ndraft: true\ntags:\n  - ecs\n  - \"web, http\"",
			FrontmatterKind::Yaml,
		)
		.xpect_eq(expected.clone());
		pairs(
			"title = \"Hello, World\"\ncount = 42 # a comment\ndraft = true\ntags = [\"ecs\", \"web, http\",]",
			FrontmatterKind::Toml,
		)
		.xpect_eq(expected);
		// a TOML date reads as the text a quoted one does
		pairs("created = 2025-07-11", FrontmatterKind::Toml)
			.xpect_eq(vec![("created".into(), Value::str("2025-07-11"))]);
	}

	/// A block beyond its format, or beyond YAML's subset, is refused with
	/// the reason.
	#[beet_core::test]
	fn refuses_what_its_format_refuses() {
		Frontmatter::parse("logo: &shared x", FrontmatterKind::Yaml)
			.unwrap_err()
			.to_string()
			.xpect_contains("an anchor or alias");
		Frontmatter::parse("title = Hello", FrontmatterKind::Toml).xpect_err();
	}

	/// Every key lowers to a literal against the default component, so nothing
	/// hand-maps frontmatter to a struct.
	#[beet_core::test]
	fn declares_the_default_component() {
		let fm = Frontmatter::parse(
			"title: Hello\norder: 2\nempty:",
			FrontmatterKind::Yaml,
		)
		.unwrap();
		let RootDeclarations(declarations) = fm.declarations("PageMeta");
		declarations.len().xpect_eq(1);
		declarations[0].name.as_str().xpect_eq("PageMeta");
		let NamedFields::Struct(fields) = &declarations[0].fields else {
			panic!("expected named fields")
		};
		// the valueless key declares nothing
		fields.len().xpect_eq(2);
		fields[0].0.as_str().xpect_eq("title");
		fields[1].1.xpect_eq(DataLiteral::Scalar(Value::Uint(2)));
	}

	/// A section declares its own component alongside the default, a TOML
	/// table and a YAML key holding a map alike.
	#[beet_core::test]
	fn declares_sectioned_components() {
		for (content, kind) in [
			(
				"title = \"Hello\"\n[Extra]\nnote = \"hi\"",
				FrontmatterKind::Toml,
			),
			("title: Hello\nExtra:\n  note: hi", FrontmatterKind::Yaml),
		] {
			let fm = Frontmatter::parse(content, kind).unwrap();
			fm.declarations("PageMeta")
				.0
				.iter()
				.map(|named| named.name.to_string())
				.collect::<Vec<_>>()
				.xpect_eq(vec!["PageMeta".to_string(), "Extra".to_string()]);
			// only the default component's keys reach the reflect surface
			fm.get_str("title").unwrap().xpect_eq("Hello");
			fm.get_str("note").is_none().xpect_true();
		}
	}

	/// Stands in for a component declared frontmatter beside `PageMeta`.
	#[derive(
		Debug,
		Default,
		Clone,
		PartialEq,
		Component,
		Reflect,
		serde::Serialize,
		serde::Deserialize,
	)]
	#[reflect(Component, Default, Frontmatter)]
	struct Course {
		unit: Option<SmolStr>,
		weeks: u32,
	}

	/// The components a root carries write as the block declaring them
	/// again, the default unsectioned and in field order, the rest as their
	/// tables, every default field unwritten.
	#[beet_core::test]
	fn writes_what_it_reads() {
		let mut app = App::new();
		app.add_plugins(ParsePlugin).register_type::<Course>();
		let world = app.world_mut();
		let meta = PageMeta {
			title: Some("Plan: one, two".into()),
			created: Some(Date::parse("2025-07-09").unwrap()),
			authors: vec!["Mel Richards".into(), "Packwood, Geoff".into()],
			order: Some(2),
			..default()
		};
		let course = Course {
			unit: Some("BSBOPS404".into()),
			weeks: 0,
		};
		let entity = world.spawn((meta.clone(), course.clone())).id();
		let block = Frontmatter::write(world, entity, "PageMeta")
			.unwrap()
			.unwrap();
		block.xpect_eq(
			"+++\ntitle = \"Plan: one, two\"\ncreated = \"2025-07-09\"\n\
			 authors = [\"Mel Richards\", \"Packwood, Geoff\"]\norder = 2\n\n\
			 [Course]\nunit = \"BSBOPS404\"\n+++\n",
		);
		let registry = world.resource::<AppTypeRegistry>().read();
		let declarations = Frontmatter::extract(&block)
			.unwrap()
			.unwrap()
			.declarations("PageMeta");
		declarations
			.get::<PageMeta>(&registry)
			.unwrap()
			.xpect_eq(Some(meta));
		declarations
			.get::<Course>(&registry)
			.unwrap()
			.xpect_eq(Some(course));
		drop(registry);
		// nothing to declare writes no block
		let bare = world.spawn(PageMeta::default()).id();
		Frontmatter::write(world, bare, "PageMeta")
			.unwrap()
			.xpect_none();
	}

	// -- extraction from raw source --

	#[beet_core::test]
	fn extract_toml_fence() {
		let fm = Frontmatter::extract(
			"+++\ntitle = \"Hello\"\norder = 2\n+++\n\n# Body",
		)
		.unwrap()
		.unwrap();
		fm.kind.xpect_eq(FrontmatterKind::Toml);
		fm.get_str("title").unwrap().xpect_eq("Hello");
		fm.get_uint("order").unwrap().xpect_eq(2);
	}

	#[beet_core::test]
	fn extract_yaml_fence() {
		let fm = Frontmatter::extract("---\ntitle: Hello\n---\n# Body")
			.unwrap()
			.unwrap();
		fm.kind.xpect_eq(FrontmatterKind::Yaml);
		fm.get_str("title").unwrap().xpect_eq("Hello");
	}

	#[beet_core::test]
	fn extract_none() {
		// no fence at all
		Frontmatter::extract("# Body")
			.unwrap()
			.is_none()
			.xpect_true();
		// a thematic break is not a fence
		Frontmatter::extract("--- not a fence")
			.unwrap()
			.is_none()
			.xpect_true();
		// an unclosed fence is not frontmatter
		Frontmatter::extract("---\ntitle: Hello")
			.unwrap()
			.is_none()
			.xpect_true();
	}

	// -- Frontmatter component --

	#[beet_core::test]
	fn frontmatter_empty() {
		let fm = Frontmatter::parse("", FrontmatterKind::Yaml).unwrap();
		fm.value.field_len().xpect_eq(0);
	}

	#[beet_core::test]
	fn dynamic_struct_fields_accessible() {
		let fm = Frontmatter::parse(
			"title: Hello\ncount: 42\ndraft: true",
			FrontmatterKind::Yaml,
		)
		.unwrap();
		fm.value.field_len().xpect_eq(3);
		fm.get_str("title").unwrap().xpect_eq("Hello");
		fm.get_str("count").is_none().xpect_true();
		fm.get_uint("count").unwrap().xpect_eq(42);
		fm.get_bool("draft").unwrap().xpect_true();
		fm.get_str("missing").is_none().xpect_true();
	}
}
