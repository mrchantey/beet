//! Frontmatter: a document's leading metadata block, YAML between `---` fences
//! or TOML between `+++`, read and written through serde
//! ([`MediaType::Yaml`], [`MediaType::Toml`]).
//!
//! The block is the markdown surface for a document's root component
//! declarations exactly as a root spread is the BSX surface, so both resolve
//! through one reflect path rather than a hand-written key mapping. Its
//! unsectioned keys declare the document's [`FrontmatterType`], and each table (`[Section]` in TOML, a key holding a map in
//! YAML) the component it names.
//!
//! What is frontmatter's own is small: the fences, the unsectioned keys
//! declaring the document's [`FrontmatterType`], a section naming the
//! component its keys declare, and default fields left out when writing. A
//! component is written into a block when its type data says it is
//! frontmatter, `#[reflect(Frontmatter)]` ([`ReflectFrontmatter`]), and a
//! block is written in the format asked for. A block's format is its
//! [`MediaType`], one of the formats [`Frontmatter::fence`] knows, so another
//! format is a row there rather than a type of its own.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::CreateTypeData;
use bevy::reflect::TypeRegistry;

/// Parsed frontmatter metadata from a YAML or TOML block, the root components
/// of the document it leads ([`components`](Self::components)).
///
/// ## Example
/// ```rust
/// # use beet_core::prelude::*;
/// # use beet_scene::prelude::*;
/// let fm = Frontmatter::parse("title: Hello\nauthor: World", MediaType::Yaml).unwrap();
/// fm.media_type.xpect_eq(MediaType::Yaml);
/// ```
#[derive(Debug)]
pub struct Frontmatter {
	/// The format the block was written in, one [`Frontmatter::fence`] knows.
	pub media_type: MediaType,
	/// The parsed pairs grouped by the component their section names, in source
	/// order. Unsectioned pairs carry `None` and target the document's
	/// [`FrontmatterType`].
	sections: Vec<FrontmatterSection>,
	/// Where the block's content starts in its document, what an error in it
	/// counts from.
	offset: usize,
}

/// The component a document's unsectioned frontmatter keys declare, resolved by
/// self-or-ancestor from whichever entity discovered the document.
///
/// With none above it a document declares the registry's own
/// ([`DocParsers::metadata_type`], `beet_ui`'s `DocMeta`). A site
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
#[reflect(Component)]
pub struct FrontmatterType {
	/// Short type path of the component, resolved against the type registry
	/// exactly as a BSX tag is.
	pub component: SmolStr,
}

/// The component the documents an entity discovers or is declare in their
/// unsectioned metadata.
#[derive(SystemParam)]
pub struct FrontmatterTypeQuery<'w, 's> {
	types: AncestorQuery<'w, 's, &'static FrontmatterType>,
	parsers: Option<Res<'w, DocParsers>>,
}

impl FrontmatterTypeQuery<'_, '_> {
	/// The component's short name: that of the nearest [`FrontmatterType`] at
	/// or above `entity`, else the registry's own.
	pub fn get(&self, entity: Entity) -> SmolStr {
		match self.types.get(entity) {
			Ok(declared) => declared.component.clone(),
			Err(_) => self
				.parsers
				.as_ref()
				.map(|parsers| parsers.metadata_type())
				.unwrap_or(DocParsers::METADATA_TYPE)
				.into(),
		}
	}
}

/// Type data marking a component as frontmatter, see [`FrontmatterType`]:
/// derived by `#[reflect(Frontmatter)]` on a `Component + Default + Serialize`
/// type, it reads the fields a written block declares, those the component's
/// default does not hold, in field order.
#[derive(Clone)]
pub struct ReflectFrontmatter {
	/// The fields of the component on an entity its default does not hold,
	/// `None` when the entity lacks it.
	fields: fn(EntityRef) -> Option<Result<Vec<(SmolStr, Value)>>>,
}

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

impl Frontmatter {
	/// The key the `toml` crate carries a datetime under, which a block's
	/// `created = 2025-07-11` reads as.
	const TOML_DATETIME: &str = "$__toml_private_datetime";

	/// Each format a block is written in, with the fence it sits between.
	const FORMATS: [(MediaType, &'static str); 2] =
		[(MediaType::Yaml, "---"), (MediaType::Toml, "+++")];

	/// The fence a block in `media_type` sits between, refusing a format no
	/// block is written in.
	pub fn fence(media_type: &MediaType) -> Result<&'static str> {
		Self::FORMATS
			.iter()
			.find(|(format, _)| format == media_type)
			.map(|(_, fence)| *fence)
			.ok_or_else(|| {
				bevyhow!(
					"`{media_type}` is not a frontmatter format, a block is \
					YAML or TOML"
				)
			})
	}

	/// Extract and parse a leading frontmatter block from a raw source: a
	/// `---` fence parses as YAML, a `+++` fence as TOML. Returns `None` when
	/// the source has no leading fence.
	///
	/// The full markdown parser extracts frontmatter as part of its pass; this
	/// standalone form serves scan-time metadata readers (eg route discovery)
	/// that must not pay for a content parse.
	pub fn extract(source: &str) -> Result<Option<Self>> {
		let trimmed = source.trim_start();
		let Some((media_type, fence)) = Self::FORMATS
			.iter()
			.find(|(_, fence)| trimmed.starts_with(fence))
		else {
			return Ok(None);
		};
		// the fence must be a whole line, ie not a thematic break `--- foo`
		let body = &trimmed[fence.len()..];
		if !body.starts_with('\n') && !body.starts_with("\r\n") {
			return Ok(None);
		}
		let Some(end) = body.find(&format!("\n{fence}")) else {
			return Ok(None);
		};
		// the content is the lines between the fences, less the first's break
		let start = body.find('\n').map_or(0, |newline| newline + 1);
		Self::parse_at(
			body.get(start..end).unwrap_or_default(),
			media_type.clone(),
			source.len() - body.len() + start,
		)
		.map(Some)
	}

	/// The components a markdown document's leading block declares, and none
	/// for any other media type or a document with no block.
	///
	/// A format property rather than a content parser's: the plain text
	/// fallback reads it too, so a scan knows a page's metadata whether or not
	/// the markdown content parser is built in.
	pub fn root_components(
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<EntityBody> {
		if !matches!(bytes.media_type(), MediaType::Markdown | MediaType::Mdx) {
			return EntityBody::default().xok();
		}
		match Self::extract(core::str::from_utf8(bytes)?)? {
			Some(frontmatter) => frontmatter.components(parsers),
			None => EntityBody::default().xok(),
		}
	}

	/// Parse a raw frontmatter block, the text between its fences, through
	/// serde in `media_type`, refusing a format no block is written in: its
	/// keys holding a map are sections, the rest unsectioned.
	pub fn parse(content: &str, media_type: MediaType) -> Result<Self> {
		Self::fence(&media_type)?;
		Self::parse_at(content, media_type, 0)
	}

	/// [`parse`](Self::parse) the block whose content starts `offset` bytes
	/// into its document, which an error in it is placed by.
	pub(super) fn parse_at(
		content: &str,
		media_type: MediaType,
		offset: usize,
	) -> Result<Self> {
		let value = match content.trim().is_empty() {
			true => Value::Map(default()),
			false => media_type
				.deserialize_document::<Value>(content.as_bytes())
				.map_err(|error| SourceError::within(error, offset))?,
		};
		let Value::Map(map) = Self::normalize(value) else {
			return Err(SourceError::at(
				bevyhow!("a frontmatter block is a map of keys"),
				offset,
			));
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
		Ok(Self {
			media_type,
			sections,
			offset,
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

	/// This block as its document's root components: each section names its
	/// own by short type path, and every unsectioned key declares the
	/// registry's [metadata type](DocParsers::metadata_type), the document's
	/// [`FrontmatterType`].
	///
	/// The one lowering a root spread takes, so a frontmatter key coerces to
	/// its field type as a markup literal does, `created = "2025-07-11"` a
	/// [`Date`]. A key with no value declares nothing.
	pub fn components(&self, parsers: &DocParsers) -> Result<EntityBody> {
		let declarations = self.sections.iter().filter_map(|section| {
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
					.unwrap_or_else(|| parsers.metadata_type().into()),
				fields: NamedFields::Struct(fields),
			})
		});
		BsxParser::lower_declarations(declarations, parsers)
			.map_err(|error| SourceError::at(error, self.offset))
	}

	/// The block declaring every frontmatter component `entity` carries, in
	/// `media_type` between its fences, the inverse of
	/// [`extract`](Self::extract) and [`components`](Self::components):
	/// `default_component`, the document's [`FrontmatterType`], unsectioned,
	/// every other as its own section in name order, each with only the
	/// fields its default does not hold, in field order. `None` when no field
	/// is written, as in a world registering no frontmatter type; a format no
	/// block is written in, or a value the format cannot hold, is refused.
	pub fn write(
		world: &World,
		entity: Entity,
		default_component: &str,
		media_type: &MediaType,
	) -> Result<Option<String>> {
		let fence = Self::fence(media_type)?;
		let Some(registry) = world.get_resource::<AppTypeRegistry>() else {
			return Ok(None);
		};
		let mut block =
			Self::block(world, entity, &registry.read(), default_component)?;
		block.sections.retain(|(_, fields)| !fields.is_empty());
		if block.fields.is_empty() && block.sections.is_empty() {
			return Ok(None);
		}
		let text = media_type
			.serialize_with_options(&block, SerializeOptions {
				pretty: false,
			})?;
		format!("{fence}\n{}{fence}\n", String::from_utf8(text)?)
			.xsome()
			.xok()
	}

	/// What [`write`](Self::write) writes, read off `entity`'s frontmatter
	/// components.
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
}

/// A block as [`Frontmatter::write`] writes it, its entries in their own
/// order where a [`Value`] map sorts its keys: the unsectioned fields, then
/// each section's table.
#[derive(Default)]
struct WrittenBlock {
	fields: Vec<(SmolStr, Value)>,
	sections: Vec<(SmolStr, Vec<(SmolStr, Value)>)>,
}

/// A table's fields in their own order.
struct InOrder<'a>(&'a [(SmolStr, Value)]);

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

#[cfg(test)]
mod test {
	use super::*;

	/// The unsectioned pairs of a block, ie the group targeting the default
	/// component.
	fn pairs(content: &str, media_type: MediaType) -> Vec<(String, Value)> {
		Frontmatter::parse(content, media_type)
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
			MediaType::Yaml,
		)
		.xpect_eq(expected.clone());
		pairs(
			"title = \"Hello, World\"\ncount = 42 # a comment\ndraft = true\ntags = [\"ecs\", \"web, http\",]",
			MediaType::Toml,
		)
		.xpect_eq(expected);
		// a TOML date reads as the text a quoted one does
		pairs("created = 2025-07-11", MediaType::Toml)
			.xpect_eq(vec![("created".into(), Value::str("2025-07-11"))]);
	}

	/// A block beyond its format, or beyond YAML's subset, is refused with
	/// the reason.
	#[beet_core::test]
	fn refuses_what_its_format_refuses() {
		Frontmatter::parse("logo: &shared x", MediaType::Yaml)
			.unwrap_err()
			.to_string()
			.xpect_contains("an anchor or alias");
		Frontmatter::parse("title = Hello", MediaType::Toml).xpect_err();
	}

	/// The components `frontmatter` declares against no types.
	fn components(frontmatter: &Frontmatter) -> EntityBody {
		frontmatter
			.components(&DocParsers::default().with_metadata_type("DocMeta"))
			.unwrap()
	}

	/// Every key declares a field of the default component, so nothing
	/// hand-maps frontmatter to a struct.
	#[beet_core::test]
	fn declares_the_default_component() {
		let fm = Frontmatter::parse(
			"title: Hello\norder: 2\nempty:",
			MediaType::Yaml,
		)
		.unwrap();
		// the valueless key declares nothing
		components(&fm).xpect_eq(
			Map::new([(
				"DocMeta",
				value!({ "title": "Hello", "order": 2u64 }),
			)])
			.into(),
		);
	}

	/// A section declares its own component alongside the default, a TOML
	/// table and a YAML key holding a map alike.
	#[beet_core::test]
	fn declares_sectioned_components() {
		for (content, media_type) in [
			("title = \"Hello\"\n[Extra]\nnote = \"hi\"", MediaType::Toml),
			("title: Hello\nExtra:\n  note: hi", MediaType::Yaml),
		] {
			let fm = Frontmatter::parse(content, media_type).unwrap();
			components(&fm).xpect_eq(
				Map::new([
					("DocMeta", value!({ "title": "Hello" })),
					("Extra", value!({ "note": "hi" })),
				])
				.into(),
			);
		}
	}

	/// Stands in for the component unsectioned keys declare.
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
	struct DocMeta {
		title: Option<SmolStr>,
		created: Option<Date>,
		authors: Vec<SmolStr>,
	}

	/// Stands in for a component declared frontmatter beside `DocMeta`.
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
		app.add_plugins(ScenePlugin)
			.register_type::<DocMeta>()
			.register_type::<Course>();
		let world = app.world_mut();
		let meta = DocMeta {
			title: Some("Plan: one, two".into()),
			created: Some(Date::parse("2025-07-09").unwrap()),
			authors: vec!["Mel Richards".into(), "Packwood, Geoff".into()],
			..default()
		};
		let course = Course {
			unit: Some("BSBOPS404".into()),
			weeks: 0,
		};
		let entity = world.spawn((meta.clone(), course.clone())).id();
		for (media_type, expected) in [
			(
				MediaType::Toml,
				"+++\ntitle = \"Plan: one, two\"\ncreated = \"2025-07-09\"\n\
				 authors = [\"Mel Richards\", \"Packwood, Geoff\"]\n\n\
				 [Course]\nunit = \"BSBOPS404\"\n+++\n",
			),
			(
				MediaType::Yaml,
				"---\ntitle: \"Plan: one, two\"\ncreated: 2025-07-09\n\
				 authors: [Mel Richards, \"Packwood, Geoff\"]\n\
				 Course:\n  unit: BSBOPS404\n---\n",
			),
		] {
			let block =
				Frontmatter::write(world, entity, "DocMeta", &media_type)
					.unwrap()
					.unwrap();
			block.clone().xpect_eq(expected);
			let components = Frontmatter::extract(&block)
				.unwrap()
				.unwrap()
				.components(
					&DocParsers::of(world).with_metadata_type("DocMeta"),
				)
				.unwrap();
			let registry = world.resource::<AppTypeRegistry>().read();
			BodyComponents::get::<DocMeta>(&components, &registry)
				.unwrap()
				.xpect_eq(Some(meta.clone()));
			BodyComponents::get::<Course>(&components, &registry)
				.unwrap()
				.xpect_eq(Some(course.clone()));
		}
		// nothing to declare writes no block, and no format but these two is
		// a block's
		let bare = world.spawn(DocMeta::default()).id();
		Frontmatter::write(world, bare, "DocMeta", &MediaType::Toml)
			.unwrap()
			.xpect_none();
		Frontmatter::write(world, entity, "DocMeta", &MediaType::Json)
			.unwrap_err()
			.to_string()
			.xpect_contains("is not a frontmatter format");
		Frontmatter::parse("{}", MediaType::Json).xpect_err();
	}

	// -- extraction from raw source --

	#[beet_core::test]
	fn extract_toml_fence() {
		let fm = Frontmatter::extract(
			"+++\ntitle = \"Hello\"\norder = 2\n+++\n\n# Body",
		)
		.unwrap()
		.unwrap();
		fm.media_type.xpect_eq(MediaType::Toml);
		components(&fm).xpect_eq(
			Map::new([(
				"DocMeta",
				value!({ "title": "Hello", "order": 2u64 }),
			)])
			.into(),
		);
	}

	#[beet_core::test]
	fn extract_yaml_fence() {
		let fm = Frontmatter::extract("---\ntitle: Hello\n---\n# Body")
			.unwrap()
			.unwrap();
		fm.media_type.xpect_eq(MediaType::Yaml);
		components(&fm).xpect_eq(
			Map::new([("DocMeta", value!({ "title": "Hello" }))]).into(),
		);
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

	/// An empty block declares nothing.
	#[beet_core::test]
	fn frontmatter_empty() {
		components(&Frontmatter::parse("", MediaType::Yaml).unwrap())
			.is_empty()
			.xpect_true();
	}
}
