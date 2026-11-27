//! [`SceneSource`]: where what a document built was written.
use alloc::sync::Arc;
use beet_core::prelude::*;

/// The document an entity is the root of, so any entity it built answers
/// where it was written ([`SourceQuery`]).
///
/// What was read rather than what is, so it is neither registered nor cloned:
/// no scene holds one.
#[derive(Debug, Component)]
#[component(clone_behavior = Ignore)]
pub struct SceneSource {
	source: Arc<DocumentSource>,
	keys: TemplateEntityMap,
}

impl SceneSource {
	/// The document.
	pub fn source(&self) -> &DocumentSource { &self.source }

	/// Land `source` on `root`, what it built under `keys`.
	pub(super) fn land(
		world: &mut World,
		root: Entity,
		source: Arc<DocumentSource>,
		keys: impl IntoIterator<Item = (EntityKey, Entity)>,
	) {
		// a format with no positions answers nothing
		if source.spans().spans.is_empty() {
			return;
		}
		let keys = TemplateEntityMap::from_pairs(keys);
		// a build tearing its own root down has nowhere to answer from
		if let Ok(mut root) = world.get_entity_mut(root) {
			root.insert(Self { source, keys });
		}
	}

	/// Where `entity` was written, if this document wrote it and its format
	/// says.
	pub fn position(&self, entity: Entity) -> Option<SourcePosition> {
		self.keys
			.get_file_key(entity)
			.and_then(|key| self.source.position(key))
	}
}

/// Where an entity was written: in the nearest document above it that wrote
/// it, content slotted into another's answering in its own.
#[derive(SystemParam)]
pub struct SourceQuery<'w, 's> {
	sources: Query<'w, 's, &'static SceneSource>,
	parents: Query<'w, 's, &'static ChildOf>,
	owners: Query<'w, 's, &'static AttributeOf>,
}

impl SourceQuery<'_, '_> {
	/// Where `entity` was written, `None` for one no document did, or one whose
	/// format has no positions.
	pub fn position(&self, entity: Entity) -> Option<SourcePosition> {
		let mut next = Some(entity);
		while let Some(above) = next {
			if let Some(position) = self
				.sources
				.get(above)
				.ok()
				.and_then(|source| source.position(entity))
			{
				return Some(position);
			}
			next = self
				.parents
				.get(above)
				.map(ChildOf::parent)
				.or_else(|_| self.owners.get(above).map(|owner| owner.get()))
				.ok();
		}
		None
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use alloc::sync::Arc;
	use beet_core::prelude::*;

	/// The positions of every element `source`, the document at `path`, built,
	/// by its tag.
	fn positions(path: &str, bytes: MediaBytes) -> Vec<(String, String)> {
		let mut world = ScenePlugin::world();
		let root = world.spawn_empty().id();
		ParseContext::new(&mut world.entity_mut(root), &bytes)
			.with_path(FwdPath::new(path).unwrap())
			.parse()
			.unwrap();
		let elements = world
			.query::<(Entity, &Element)>()
			.iter(&world)
			.map(|(entity, element)| (entity, element.tag().to_string()))
			.collect::<Vec<_>>();
		let mut positions = world.with_state::<SourceQuery, _>(|sources| {
			elements
				.into_iter()
				.map(|(entity, tag)| {
					let position = sources
						.position(entity)
						.map(|position| position.to_string())
						.unwrap_or_default();
					(tag, position)
				})
				.collect::<Vec<_>>()
		});
		positions.sort();
		positions
	}

	#[beet_core::test]
	fn a_built_entity_answers_where_it_was_written() {
		positions(
			"a.bsx",
			MediaBytes::new_bsx("<main>\n\t<h1>hi</h1>\n\t<p/>\n</main>"),
		)
		.xpect_eq(vec![
			("h1".into(), "a.bsx:2:2".into()),
			("main".into(), "a.bsx:1:1".into()),
			("p".into(), "a.bsx:3:2".into()),
		]);
	}

	/// What a directive wrote stands where its host does.
	#[beet_core::test]
	fn what_a_directive_wrote_answers_for_its_host() {
		positions(
			"a.bsx",
			MediaBytes::new_bsx("<main>\n<button bx:onclick=\"go()\"/></main>"),
		)
		.xpect_eq(vec![
			("button".into(), "a.bsx:2:1".into()),
			("main".into(), "a.bsx:1:1".into()),
			("script".into(), "a.bsx:2:1".into()),
		]);
	}

	/// A save holds nothing of where it was read.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn a_save_holds_no_source() {
		let mut world = ScenePlugin::world();
		let root = world.spawn_empty().id();
		ParseContext::new(
			&mut world.entity_mut(root),
			&MediaBytes::new_bsx("<main/>"),
		)
		.parse()
		.unwrap();
		TemplateSaver::new()
			.save_roots(&mut world, MediaType::Json, [root])
			.unwrap()
			.as_utf8()
			.unwrap()
			.xnot()
			.xpect_contains("SceneSource");
	}

	/// The text each entity of `bytes` was written as, by its key.
	fn written(bytes: &MediaBytes) -> Vec<(String, String)> {
		let document = ScenePlugin::parsers().parse(bytes).unwrap();
		document
			.body
			.entities
			.keys()
			.filter_map(|key| {
				document.spans.get(key).map(|span| {
					(
						key.to_string(),
						bytes.as_utf8().unwrap()
							[span.start as usize..span.end as usize]
							.to_string(),
					)
				})
			})
			.collect()
	}

	fn pairs<'a>(
		expected: impl IntoIterator<Item = (&'a str, &'a str)>,
	) -> Vec<(String, String)> {
		expected
			.into_iter()
			.map(|(key, text)| (key.to_string(), text.to_string()))
			.collect()
	}

	/// Markup, as written from its opening to its close.
	#[beet_core::test]
	fn markup_spans() {
		for media_type in [MediaType::Bsx, MediaType::Html] {
			written(&MediaBytes::new_str(
				media_type,
				"\u{FEFF}<main>\n\t<!-- a -->\n\t<p class=\"b\">é &amp; c</p><br/>\n</main>",
			))
			.xpect_eq(pairs([
				(
					"0",
					"<main>\n\t<!-- a -->\n\t<p class=\"b\">é &amp; c</p><br/>\n</main>",
				),
				("0/#comment.0", "<!-- a -->"),
				("0/p.0", "<p class=\"b\">é &amp; c</p>"),
				("0/p.0/@class", "class=\"b\""),
				("0/p.0/#text.0", "é &amp; c"),
				("0/br.0", "<br/>"),
			]));
		}
		// several roots are held by one reaching over them
		written(&MediaBytes::new_bsx("<a/>\n<b/>\n")).xpect_eq(pairs([
			("0", "<a/>\n<b/>"),
			("0/a.0", "<a/>"),
			("0/b.0", "<b/>"),
		]));
		// what is written by key alone keeps its place
		written(&MediaBytes::new_bsx("<a><b bx:ref=\"it\"/>{@doc:c}</a>"))
			.xpect_eq(pairs([
				("0", "<a><b bx:ref=\"it\"/>{@doc:c}</a>"),
				("it", "<b bx:ref=\"it\"/>"),
				("0/#text.0", "{@doc:c}"),
			]));
	}

	/// A markdown page's ranges are the file's, in prose, in markup embedded
	/// in it and in a fence that is a document.
	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn markdown_spans() {
		let source = "+++\ntitle = \"a\"\n+++\n\n# Hi\n\n<div class=\"b\">\n\n*em* {x}\n\n</div>\n\n- one\n- `two`\n\n```text/html render\n<b>c</b>\n```\n";
		written(&MediaBytes::new_markdown(source)).xpect_eq(pairs([
			("0", source),
			("0/h1.0", "# Hi\n"),
			("0/h1.0/#text.0", "Hi"),
			("0/div.0", "<div class=\"b\">\n\n*em* {x}\n\n</div>"),
			("0/div.0/@class", "class=\"b\""),
			("0/div.0/p.0", "*em* {x}\n"),
			("0/div.0/p.0/em.0", "*em*"),
			("0/div.0/p.0/em.0/#text.0", "em"),
			("0/div.0/p.0/#text.0", " {x}"),
			("0/ul.0", "- one\n- `two`\n\n"),
			("0/ul.0/li.0", "- one\n"),
			("0/ul.0/li.0/#text.0", "one"),
			("0/ul.0/li.1", "- `two`\n\n"),
			("0/ul.0/li.1/code.0", "`two`"),
			("0/ul.0/li.1/code.0/#text.0", "two"),
			("0/b.0", "<b>c</b>"),
			("0/b.0/#text.0", "c"),
		]));
	}

	/// TOML's reader says where each entity's table is; json's says nothing.
	#[cfg(all(feature = "toml", feature = "json"))]
	#[beet_core::test]
	fn structured_spans() {
		written(&MediaBytes::new_str(
			MediaType::Toml,
			"\"$type\" = \"org.beet.scene#body\"\n[entities.0]\nHealth = 3\n[entities.1]\nHealth = 4\n",
		))
		.into_iter()
		.map(|(key, text)| (key, text.lines().next().unwrap().to_string()))
		.collect::<Vec<_>>()
		.xpect_eq(pairs([
			("0", "[entities.0]"),
			("1", "[entities.1]"),
		]));
		written(&MediaBytes::new_json(
			r#"{"$type":"org.beet.scene#body","entities":{"0":{"Health":3}}}"#,
		))
		.xpect_empty();
		written(&MediaBytes::new_text("hi")).xpect_empty();
	}

	/// A body is what its document says, wherever it says it.
	#[cfg(feature = "postcard")]
	#[beet_core::test]
	fn whitespace_changes_no_body() {
		let read = |markup: &str| {
			let document = ScenePlugin::parsers()
				.parse(&MediaBytes::new_bsx(markup))
				.unwrap();
			(
				document.body.to_bytes(MediaType::Postcard).unwrap(),
				document.spans,
			)
		};
		let (body, spans) = read("<main><p>hi</p><br/></main>");
		let (spaced, elsewhere) =
			read("\n<main>\n\t<p>hi</p>\n\n\t<br />\n</main>\n");
		body.xpect_eq(spaced);
		spans.xpect_not_eq(elsewhere);
	}

	/// The message of the error `bytes`, the document at `path`, is.
	fn refused(path: &str, bytes: MediaBytes) -> String {
		ScenePlugin::parsers()
			.read(&bytes, Some(&FwdPath::new(path).unwrap()))
			.unwrap_err()
			.to_string()
	}

	/// A document that does not parse or lower says where, in every format
	/// that knows.
	#[beet_core::test]
	fn an_error_names_its_place() {
		// a value the grammar refuses, at the character it stopped at
		refused("a.bsx", MediaBytes::new_bsx("<main>\n\t{@nope:x}\n</main>"))
			.xpect_starts_with("a.bsx:2:8: unknown binding source `@nope`");
		// an element its lowering refuses
		refused("a.bsx", MediaBytes::new_bsx("<main>\n\t<Slot/></main>"))
			.xpect_starts_with("a.bsx:2:2: `<Slot>`");
		// a directive nothing matches
		refused(
			"a.bsx",
			MediaBytes::new_bsx("<main>\n<p>\n<b bx:nope=\"x\"/></p></main>"),
		)
		.xpect_starts_with("a.bsx:3:1: `bx:nope`");
		// a block that is never closed, where it opened
		refused("a.bsx", MediaBytes::new_bsx("<main>\n{ a"))
			.xpect_starts_with("a.bsx:2:1: unterminated");
		refused(
			"a.xml",
			MediaBytes::new_str(MediaType::Xml, "<a>\n  <b>\n</a>"),
		)
		.xpect_starts_with("a.xml:3:");
		// a document read with no path names the place alone
		BsxParser::body(&ScenePlugin::world(), "<main>\n<Slot/></main>")
			.unwrap_err()
			.to_string()
			.xpect_starts_with("2:1: `<Slot>`");
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn a_markdown_error_names_its_place_in_the_file() {
		refused(
			"a.mdx",
			MediaBytes::new_str(MediaType::Mdx, "# Hi\n\nsome {@nope:x} prose"),
		)
		.xpect_starts_with("a.mdx:3:12: in `{@nope:x}`");
		// in embedded markup, its directives included
		refused(
			"a.md",
			MediaBytes::new_markdown(
				"# Hi\n\n<div>\n<b bx:nope=\"x\"></b>\n</div>",
			),
		)
		.xpect_starts_with("a.md:4:1: `bx:nope`");
		// in a fence that is a document
		refused(
			"a.md",
			MediaBytes::new_markdown(
				"# Hi\n\n```application/vnd.beet.bsx render\n<a>\n{@nope:x}</a>\n```",
			),
		)
		.xpect_starts_with("a.md:5:7: unknown binding source");
		// in the frontmatter
		refused(
			"a.md",
			MediaBytes::new_markdown("+++\na = 1\nb = nope\n+++"),
		)
		.xpect_starts_with("a.md:3:5: ");
	}

	#[cfg(all(feature = "toml", feature = "json"))]
	#[beet_core::test]
	fn a_structured_error_names_its_place() {
		refused("a.json", MediaBytes::new_json("{\n  \"a\": nope\n}"))
			.xpect_starts_with("a.json:2:");
		refused(
			"a.toml",
			MediaBytes::new_str(MediaType::Toml, "a = 1\nb = nope"),
		)
		.xpect_starts_with("a.toml:2:5: ")
		// the place is said once, by the one formatter
		.xnot()
		.xpect_contains("line 2");
	}

	/// A value its type refuses names the entity it was written on.
	#[beet_core::test]
	fn a_reflection_error_names_its_place() {
		let world = ScenePlugin::world();
		let source = DocParsers::of(&world)
			.read(
				&MediaBytes::new_bsx("<main>\n<p {Unknowable}/></main>"),
				Some(&fwd_path!("a.bsx")),
			)
			.unwrap();
		let mut document = ParsedDocument::from(source.body().clone());
		document.spans = source.spans().clone();
		document.body.entities[&EntityKey::parse("0/p.0").unwrap()]
			.insert("ReactiveChildren", Value::str("nope"));
		DynamicTemplate::read(
			&world,
			Arc::new(DocumentSource::new(
				Some(fwd_path!("a.bsx")),
				source.bytes().clone(),
				document,
			)),
		)
		.map(|_| ())
		.unwrap_err()
		.to_string()
		.xpect_starts_with("a.bsx:2:1: ");
	}
}
