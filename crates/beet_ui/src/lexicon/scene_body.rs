use crate::prelude::*;
use beet_core::prelude::*;
use beet_scene::prelude::*;

/// `org.beet.scene#body` ([`SceneBody::NSID`]): a tree as a scene body, and
/// one read back onto an entity.
///
/// Rendering saves the tree at the entity through [`TemplateSaver`] with
/// nothing filtered: every component the saver writes is in the member, keyed
/// by type path, its keys minted afresh in tree order, and a value the data
/// model cannot hold (an integer past `i64`) is an error here rather than at
/// the record's write. Parsing spawns the
/// member through [`TemplateLoader`], its first entity the one parsed onto,
/// under the body's unregistered rule: a component this binary never
/// registered is left off and held ([`Unregistered`]).
///
/// A record sorts the keys of every map, so the member's entities are read in
/// [record order](SceneBody::in_record_order), which is the tree order the
/// renderer minted its keys in.
///
/// Installed by [`RenderPlugin`], so every build carrying the render pipeline
/// reads and writes the def.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SceneBodyLexicon;

impl LexiconRenderer for SceneBodyLexicon {
	fn nsid(&self) -> Nsid { SceneBody::NSID }

	fn render(&mut self, cx: &mut RenderContext) -> Result<OpenUnion> {
		let (body, _) = TemplateSaver::new().roots_body(
			cx.world,
			&[cx.entity],
			default(),
		)?;
		let body = body.to_value()?;
		AtprotoValue::try_from(body.clone())?;
		OpenUnion::new(SceneBody::NSID, body)
	}
}

impl LexiconParser for SceneBodyLexicon {
	fn nsid(&self) -> Nsid { SceneBody::NSID }

	fn parse(&mut self, cx: LexiconParseContext) -> Result {
		let body =
			SceneBody::from_value(Value::Map(cx.member.fields().clone()))?
				.in_record_order();
		let entity = cx.entity.id();
		cx.entity.world_scope(|world| {
			TemplateLoader::new(world)
				.with_entity(entity)
				.load_onto(&body)
				.map(|_| ())
		})
	}
}

impl Installer for SceneBodyLexicon {
	fn install(self, app: &mut App) {
		LexiconRenderers::register_on(app, self);
		LexiconParsers::register_on(app, self);
	}
}

// the fixture post is markdown
#[cfg(all(test, feature = "markdown"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;

	/// A post exercising the blocks a page is made of, long enough that its
	/// keys run past `"9"`, where a sorted map and file order part ways.
	const POST: &str = r#"# Full Stack Bevy

A paragraph with *emphasis*, `code` and [a link](https://beet.org/docs).

```rust
fn main() { println!("hello"); }
```

- one
- two

> A quote.

![a diagram](/assets/diagram.png)
"#;

	/// A component holding a float, which the data model cannot.
	#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
	#[reflect(Component, Default)]
	struct Opacity(f32);

	fn world() -> World {
		let mut app = world_ext::ui_app();
		app.register_type::<Opacity>();
		core::mem::take(app.world_mut())
	}

	/// The fixture post parsed under a parent, as a page's content sits under
	/// its layout: the parent is no part of what is published.
	fn post(world: &mut World) -> Entity {
		let layout = world.spawn(Element::new("main")).id();
		let post = world.spawn(ChildOf(layout)).id();
		ParseContext::new(
			&mut world.entity_mut(post),
			&MediaBytes::new_markdown(POST),
		)
		.parse()
		.unwrap();
		post
	}

	fn html(world: &mut World, entity: Entity) -> String {
		DocRenderers::render(
			world,
			entity,
			&RequestParts::default().with_accept(MediaType::Html),
		)
		.unwrap()
		.to_string()
	}

	fn render(world: &mut World, entity: Entity) -> OpenUnion {
		LexiconRenderers::render(
			world,
			entity,
			&RequestParts::default(),
			&SceneBody::NSID,
		)
		.unwrap()
	}

	/// `member` parsed onto a fresh entity of a fresh world.
	fn parse(member: &OpenUnion) -> (World, Entity) {
		let mut world = world();
		let mut entity = world.spawn_empty();
		LexiconParsers::parse(&mut entity, member, None).unwrap();
		let entity = entity.id();
		(world, entity)
	}

	/// The member as a record carries it and a reader receives it: encoded
	/// into the data model, written as json text with its object keys sorted
	/// as strings (`"10"` before `"2"`), as a canonical writer orders them,
	/// read back and decoded.
	#[cfg(feature = "json")]
	fn through_a_record(member: OpenUnion) -> OpenUnion {
		let record = AtprotoValue::try_from(Value::from(member)).unwrap();
		let mut sorted = Value::clone(&record);
		sorted.sort_maps();
		AtprotoValue::parse_json(&sorted.to_json_string())
			.unwrap()
			.decode()
			.xmap(OpenUnion::try_from)
			.unwrap()
	}

	/// A post rendered as a scene body and read back into a fresh world
	/// renders the html the original does, byte for byte, and carries no
	/// reference to the parent it sat under.
	#[beet_core::test]
	fn round_trips_to_identical_html() {
		let mut source = world();
		let post = post(&mut source);
		let expected = html(&mut source, post);
		expected.xref().xpect_contains("<h1>Full Stack Bevy</h1>");

		let member = render(&mut source, post);
		member.r#type().clone().xpect_eq(SceneBody::NSID);
		// the root is the post itself, with no parent
		let body =
			SceneBody::from_value(Value::Map(member.fields().clone())).unwrap();
		body.entities[&EntityKey::from(0)]
			.contains_typed::<ChildOf>()
			.xpect_false();
		// the detach left the live tree as it was
		source.get::<ChildOf>(post).xpect_some();

		let (mut read, entity) = parse(&member);
		html(&mut read, entity).xpect_eq(expected);
	}

	/// The same through a record, whose maps do not keep the order they were
	/// written in.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn round_trips_through_a_record() {
		let mut source = world();
		let post = post(&mut source);
		let expected = html(&mut source, post);
		let member = through_a_record(render(&mut source, post));
		let (mut read, entity) = parse(&member);
		html(&mut read, entity).xpect_eq(expected);
	}

	/// The same tree renders the same member twice, so a converge's second
	/// run finds nothing to write.
	#[beet_core::test]
	fn renders_identically_twice() {
		let mut source = world();
		let post = post(&mut source);
		let first = render(&mut source, post);
		render(&mut source, post).xpect_eq(first);
	}

	/// A float crosses the data model as `org.beet.core#float` and reads back
	/// the number it was.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn a_float_survives_the_data_model() {
		let mut source = world();
		let post = post(&mut source);
		source.entity_mut(post).insert(Opacity(0.3));
		let member = render(&mut source, post);
		AtprotoValue::try_from(Value::from(member.clone()))
			.unwrap()
			.to_json_string()
			.xpect_contains(AtprotoValue::FLOAT.as_str());

		let (read, entity) = parse(&through_a_record(member));
		read.get::<Opacity>(entity).unwrap().xpect_eq(Opacity(0.3));
	}

	/// A member naming a component this binary never registered spawns its
	/// entity without it and holds it, where a strict reader refuses it.
	#[beet_core::test]
	fn an_unregistered_component_is_held() {
		let member = OpenUnion::new(
			SceneBody::NSID,
			value!({
				"entities": {
					"0": {
						"beet_core::types::element::element::Element": "p",
						"some_plugin::Sparkle": { "amount": 3 }
					}
				}
			}),
		)
		.unwrap();
		let (mut read, entity) = parse(&member);
		read.get::<Element>(entity).unwrap().tag().xpect_eq("p");
		read.get::<Unregistered>(entity)
			.unwrap()
			.get("some_plugin::Sparkle")
			.unwrap()
			.clone()
			.xpect_eq(value!({ "amount": 3 }));
		read.with_state::<UnregisteredQuery, _>(|query| query.refuse(entity))
			.unwrap_err()
			.to_string()
			.xpect_contains("some_plugin::Sparkle");
	}

	/// The published shape, so a change to it is reviewed.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn snapshots_the_member() {
		let mut source = world();
		let post = post(&mut source);
		let body = SceneBody::from_value(Value::Map(
			render(&mut source, post).fields().clone(),
		))
		.unwrap();
		body.to_bytes_with_options(MediaType::Json, SerializeOptions {
			pretty: true,
		})
		.unwrap()
		.to_string()
		.xpect_snapshot();
	}
}
