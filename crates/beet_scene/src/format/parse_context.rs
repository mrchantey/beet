//! [`ParseContext`]: a document parsed into a world.
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;

/// A parse into a world: the bytes, the entity they land on and the package
/// path they were read from, the one pipeline every format shares: choose the
/// parser, parse, build the body onto the entity, then one [`PostParseTree`]
/// run. A [`DocParser`] itself takes bytes alone.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_scene::prelude::*;
/// let mut world = ScenePlugin::world();
/// let bytes = MediaBytes::new_bsx("<main><p>hi</p></main>");
/// let mut entity = world.spawn_empty();
/// ParseContext::new(&mut entity, &bytes).parse().unwrap();
/// entity.get::<Element>().unwrap().tag().xpect_eq("main");
/// ```
///
/// Two lifetimes: `'w` for the world borrow inside [`EntityWorldMut`], and
/// `'a` for every other borrow, which must outlive the parse.
pub struct ParseContext<'a, 'w> {
	/// The entity to parse onto.
	pub entity: &'a mut EntityWorldMut<'w>,
	/// The typed bytes to parse.
	pub bytes: &'a MediaBytes,
	/// The document's path in its package, which a [`DocPath`] it holds
	/// resolves from.
	pub path: Option<RelPath>,
	/// The component the document's unsectioned metadata declares, the
	/// registry's own by default ([`DocParsers::metadata_type`]).
	pub metadata_type: Option<TypeName>,
}

impl<'a, 'w> ParseContext<'a, 'w> {
	/// Parse `bytes` onto `entity`.
	pub fn new(
		entity: &'a mut EntityWorldMut<'w>,
		bytes: &'a MediaBytes,
	) -> Self {
		Self {
			entity,
			bytes,
			path: None,
			metadata_type: None,
		}
	}

	/// Parse as the document at `path` of its package.
	pub fn with_path(mut self, path: RelPath) -> Self {
		self.path = Some(path);
		self
	}

	/// Declare the document's unsectioned metadata as the component
	/// `metadata_type` names.
	pub fn with_metadata_type(
		mut self,
		metadata_type: impl Into<TypeName>,
	) -> Self {
		self.metadata_type = Some(metadata_type.into());
		self
	}

	/// Parse the bytes onto the entity, its root the entity itself, through
	/// the parser their media type selects, then extend the parsed tree in one
	/// [`PostParseTree`] run.
	pub fn parse(self) -> Result<(), ParseError> {
		let Self {
			entity,
			bytes,
			path,
			metadata_type,
		} = self;
		let mut parsers = DocParsers::of(entity.world());
		if let Some(metadata_type) = metadata_type {
			parsers = parsers.with_metadata_type(metadata_type);
		}
		let source = parsers.read(bytes, path.as_ref())?;
		DynamicTemplate::read(entity.world(), Arc::new(source))?
			.build_document(entity)?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A world reading the base formats.
	fn world() -> World { ScenePlugin::world() }

	/// Parse `bytes` onto a fresh entity of `world`.
	fn parse(
		world: &mut World,
		bytes: &MediaBytes,
	) -> Result<Entity, ParseError> {
		let entity = world.spawn_empty().id();
		ParseContext::new(&mut world.entity_mut(entity), bytes).parse()?;
		Ok(entity)
	}

	/// A body of one root holding `value`.
	fn holding(value: Value) -> SceneBody {
		SceneBody::root(EntityBody::default().with_typed(&value).unwrap())
	}

	#[beet_core::test]
	fn plain_text_is_one_entity_holding_the_text() {
		DocParsers::of(&world())
			.body(&MediaBytes::new_text("hello"))
			.unwrap()
			.xpect_eq(holding(Value::Str("hello".into())));
	}

	/// CSS has no parser of its own but is a text type, so it reads as plain
	/// text, landing on the entity it parses onto.
	#[beet_core::test]
	fn text_falls_back_to_plain_text() {
		let mut world = world();
		let entity =
			parse(&mut world, &MediaBytes::new_css("body { color: red; }"))
				.unwrap();
		world
			.get::<Value>(entity)
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("body { color: red; }".into()));
	}

	#[beet_core::test]
	fn binary_type_errors() {
		parse(
			&mut world(),
			&MediaBytes::new(MediaType::Png, b"\x89PNG".as_slice()),
		)
		.unwrap_err();
	}

	#[cfg(feature = "json")]
	#[beet_core::test]
	fn json_without_a_type_is_one_value() {
		DocParsers::of(&world())
			.body(&MediaBytes::new_json(r#"{"todos":["milk"]}"#))
			.unwrap()
			.xpect_eq(holding(value!({ "todos": ["milk"] })));
	}

	#[cfg(feature = "json")]
	#[beet_core::test]
	fn json_naming_the_def_is_a_body() {
		let body = DocParsers::of(&world())
			.body(&MediaBytes::new_json(
				r#"{"$type":"org.beet.scene#body","entities":{"0":{"Health":3}}}"#,
			))
			.unwrap();
		body.entities[&EntityKey::ROOT]
			.get("Health")
			.unwrap()
			.clone()
			.xpect_eq(Value::Uint(3));
	}

	#[cfg(feature = "toml")]
	#[beet_core::test]
	fn toml_reads_both_ways() {
		let parsers = DocParsers::of(&world());
		parsers
			.body(&MediaBytes::new_str(MediaType::Toml, "title = \"Post\""))
			.unwrap()
			.xpect_eq(holding(value!({ "title": "Post" })));
		parsers
			.body(&MediaBytes::new_str(
				MediaType::Toml,
				"\"$type\" = \"org.beet.scene#body\"\n[entities.0]\nHealth = 3",
			))
			.unwrap()
			.entities
			.len()
			.xpect_eq(1);
	}

	/// Postcard names nothing of itself, so a body is told from a value by
	/// the `$type` it is always written with.
	#[cfg(feature = "postcard")]
	#[beet_core::test]
	fn postcard_reads_both_ways() {
		let parsers = DocParsers::of(&world());
		let body = holding(Value::Str("hi".into()));
		parsers
			.body(&body.to_bytes(MediaType::Postcard).unwrap())
			.unwrap()
			.xpect_eq(body);
		let value = value!({ "a": [1u64, 2u64] });
		parsers
			.body(&MediaBytes::serialize(MediaType::Postcard, &value).unwrap())
			.unwrap()
			.xpect_eq(holding(value));
	}

	#[beet_core::test]
	fn javascript_is_one_script() {
		DocParsers::of(&world())
			.body(&MediaBytes::new_str(
				MediaType::Javascript,
				"console.log(1)",
			))
			.unwrap()
			.xpect_eq(SceneBody::root(
				EntityBody::default()
					.with_typed(&RunScript)
					.unwrap()
					.with_typed(&Value::Str("console.log(1)".into()))
					.unwrap(),
			));
	}

	/// A downstream parser answers its media types ahead of a base format,
	/// installed before or after it.
	#[beet_core::test]
	fn a_downstream_parser_replaces_a_builtin() {
		#[derive(Clone)]
		struct Shout;
		impl DocParser for Shout {
			fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Text] }
			fn parse(
				&mut self,
				bytes: &MediaBytes,
				_parsers: &DocParsers,
			) -> Result<ParsedDocument, ParseError> {
				let text = bytes.as_utf8()?.to_uppercase();
				ParsedDocument::from(SceneBody::root(
					EntityBody::default().with_typed(&Value::new(text))?,
				))
				.xok()
			}
		}
		impl Installer for Shout {
			fn install(self, app: &mut App) {
				DocParsers::register_on(app, self);
			}
		}
		let mut after = App::new();
		after.add_plugins(ScenePlugin).install(Shout);
		let mut before = App::new();
		before.install(Shout).add_plugins(ScenePlugin);
		for mut app in [after, before] {
			let entity =
				parse(app.world_mut(), &MediaBytes::new_text("hi")).unwrap();
			app.world()
				.get::<Value>(entity)
				.cloned()
				.unwrap()
				.xpect_eq(Value::Str("HI".into()));
		}
	}
}
