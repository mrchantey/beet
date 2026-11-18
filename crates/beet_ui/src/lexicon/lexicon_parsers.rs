use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;

/// Parses one lexicon object onto an entity, registered per def in
/// [`LexiconParsers`] by installing it, its [`Installer`] impl forwarding to
/// [`LexiconParsers::register_on`]. The registry clones the registered
/// instance for every parse.
pub trait LexiconParser: 'static + Send + Sync + Clone + Installer {
	/// The def this parser reads, ie `pub.leaflet.blocks.text`.
	fn nsid(&self) -> Nsid;
	/// Parse `cx.member` onto `cx.entity`, each nested member through
	/// [`LexiconParseContext::parse_child`].
	fn parse(&mut self, cx: LexiconParseContext) -> Result;
}

/// Every [`LexiconParser`] this world reads, keyed by NSID, the last
/// registered for a def replacing any before it.
///
/// Initialized empty by [`RenderPlugin`], each parser registered by
/// installing it. One parser type serving several defs registers a
/// configured instance per def.
#[derive(Default, Clone, Resource)]
pub struct LexiconParsers {
	parsers: HashMap<Nsid, Arc<dyn ErasedLexiconParser>>,
}

impl LexiconParsers {
	/// Register `parser` for its def, replacing any parser registered for it
	/// before.
	pub fn register(&mut self, parser: impl LexiconParser) -> &mut Self {
		self.parsers.insert(parser.nsid(), Arc::new(parser));
		self
	}

	/// [`register`](Self::register) `parser` in `app`'s registry, the body of
	/// every [`LexiconParser`]'s [`Installer`] impl.
	pub fn register_on(app: &mut App, parser: impl LexiconParser) {
		app.world_mut()
			.get_resource_or_init::<Self>()
			.register(parser);
	}

	/// Every registered def, sorted.
	pub fn available(&self) -> Vec<Nsid> {
		let mut available: Vec<Nsid> = self.parsers.keys().cloned().collect();
		available.sort();
		available
	}

	/// Parse `member` onto `entity` through the parser its `$type` names; a
	/// def nothing registered lands whole as [`UnknownLexicon`], with a
	/// `debug!`, never an error.
	pub fn parse(
		entity: &mut EntityWorldMut,
		member: &OpenUnion,
		source: Option<&LexiconSource>,
	) -> Result {
		let parser = entity
			.world()
			.get_resource::<Self>()
			.ok_or_else(|| {
				bevyhow!(
					"no `LexiconParsers` in this world: add the `RenderPlugin`"
				)
			})?
			.parser(member.r#type());
		match parser {
			Some(parser) => parser.parse(LexiconParseContext {
				entity,
				member,
				source,
			}),
			None => {
				debug!(
					"no lexicon parser for `{}`, kept as an `UnknownLexicon`",
					member.r#type()
				);
				entity.insert(UnknownLexicon(member.clone()));
				Ok(())
			}
		}
	}

	/// The parser reading `nsid`.
	fn parser(&self, nsid: &Nsid) -> Option<Arc<dyn ErasedLexiconParser>> {
		self.parsers.get(nsid).cloned()
	}
}

/// Context passed to [`LexiconParser::parse`].
pub struct LexiconParseContext<'a, 'w> {
	/// The entity the member parses onto.
	pub entity: &'a mut EntityWorldMut<'w>,
	/// The member, already decoded (`AtprotoValue::decode`), so no parser
	/// sees an `org.beet.core#float` or a `$bytes` object.
	pub member: &'a OpenUnion,
	/// The repo the record was read from, when the caller knows it.
	pub source: Option<&'a LexiconSource>,
}

impl LexiconParseContext<'_, '_> {
	/// Spawn the next child of [`entity`](Self::entity), in child order, and
	/// parse `member` onto it through the registry, answering the child.
	pub fn parse_child(&mut self, member: &OpenUnion) -> Result<Entity> {
		let (parent, source) = (self.entity.id(), self.source);
		self.entity.world_scope(|world| {
			let mut child = world.spawn(ChildOf(parent));
			LexiconParsers::parse(&mut child, member, source)?;
			child.id().xok()
		})
	}
}

/// The repo a parsed record was read from, so a blob it references has a url.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconSource {
	/// The account whose repo holds the record.
	pub did: Did,
	/// The PDS hosting the repo, resolved by the caller (`DidDocument::pds_endpoint`).
	pub pds: Url,
}

impl LexiconSource {
	/// `<pds>/xrpc/com.atproto.sync.getBlob?did=<did>&cid=<cid>`.
	pub fn blob_url(&self, cid: &Cid) -> Url {
		let mut path = self.pds.path().clone();
		path.extend(["xrpc".into(), "com.atproto.sync.getBlob".into()]);
		self.pds
			.clone()
			.with_rooted(true)
			.with_path(path)
			.with_param("did", self.did.as_str())
			.with_param("cid", cid.as_str())
	}
}

/// The object-safe face of a [`LexiconParser`], which a registry can hold.
trait ErasedLexiconParser: 'static + Send + Sync {
	fn parse(&self, cx: LexiconParseContext) -> Result;
}

impl<T: LexiconParser> ErasedLexiconParser for T {
	fn parse(&self, cx: LexiconParseContext) -> Result {
		LexiconParser::parse(&mut self.clone(), cx)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// `com.example.list`: an object whose `items` are members, each parsed
	/// onto a child of the list's entity.
	#[derive(Clone)]
	struct List;

	impl LexiconParser for List {
		fn nsid(&self) -> Nsid { Nsid::new_static("com.example.list") }

		fn parse(&mut self, mut cx: LexiconParseContext) -> Result {
			cx.entity.insert(Element::new("ul"));
			for item in cx.member.fields().get("items")?.as_list()? {
				cx.parse_child(&OpenUnion::try_from(item.clone())?)?;
			}
			Ok(())
		}
	}

	impl Installer for List {
		fn install(self, app: &mut App) {
			LexiconParsers::register_on(app, self);
		}
	}

	/// `com.example.item`: its `text` as an `li`, tagged so a test can tell
	/// two registrations apart.
	#[derive(Clone)]
	struct Item(&'static str);

	impl LexiconParser for Item {
		fn nsid(&self) -> Nsid { Nsid::new_static("com.example.item") }

		fn parse(&mut self, cx: LexiconParseContext) -> Result {
			let text = cx.member.fields().get("text")?.as_str()?.to_string();
			cx.entity.insert((Element::new("li"), children![Value::from(
				format!("{}{text}", self.0)
			)]));
			Ok(())
		}
	}

	impl Installer for Item {
		fn install(self, app: &mut App) {
			LexiconParsers::register_on(app, self);
		}
	}

	/// A world reading through `installer`.
	fn world<M>(installer: impl Installer<M>) -> World {
		let mut app = world_ext::ui_app();
		app.install(installer);
		core::mem::take(app.world_mut())
	}

	/// `member` parsed onto a fresh entity of `world`, answering it.
	fn parse(world: &mut World, member: Value) -> Entity {
		let mut entity = world.spawn_empty();
		LexiconParsers::parse(
			&mut entity,
			&OpenUnion::try_from(member).unwrap(),
			None,
		)
		.unwrap();
		entity.id()
	}

	/// The list fixture: one known item, one unknown.
	fn list() -> Value {
		value!({
			"$type": "com.example.list",
			"items": [
				{ "$type": "com.example.item", "text": "known" },
				{ "$type": "com.example.poll", "options": ["a", "b"] }
			]
		})
	}

	/// Each nested member dispatches back through the registry onto its own
	/// child, in order, and a def nothing reads lands whole.
	#[beet_core::test]
	fn dispatches_nested_members() {
		let mut world = world((List, Item("")));
		let entity = parse(&mut world, list());
		HtmlRenderer::default()
			.render(&mut RenderContext::new(
				&mut world,
				entity,
				&RequestParts::default(),
			))
			.unwrap()
			.to_string()
			.xpect_eq("<ul><li>known</li></ul>");
		let unknown = world.entity(entity).get::<Children>().unwrap()[1];
		world
			.entity(unknown)
			.get::<UnknownLexicon>()
			.unwrap()
			.r#type()
			.as_str()
			.xpect_eq("com.example.poll");
	}

	/// The parser registered last for a def reads it.
	#[beet_core::test]
	fn last_registration_wins() {
		let mut world = world((Item("first "), Item("last ")));
		let entity = parse(
			&mut world,
			value!({ "$type": "com.example.item", "text": "x" }),
		);
		PlainTextRenderer::default()
			.render(&mut RenderContext::new(
				&mut world,
				entity,
				&RequestParts::default(),
			))
			.unwrap()
			.to_string()
			.xpect_eq("last x\n");
		// replaced, not registered beside it, and sorted with the built-ins
		world
			.resource::<LexiconParsers>()
			.available()
			.xpect_eq(vec![
				Nsid::new_static("com.example.item"),
				SceneBody::NSID,
			]);
	}

	/// An unknown member parses to an [`UnknownLexicon`] and renders back as
	/// the same json bytes, though no renderer knows its def.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn unknown_member_round_trips() {
		let json = r#"{"$type":"com.example.poll","options":["a","b"],"question":{"text":"which?","weight":2}}"#;
		let mut world = world_ext::ui_world();
		let member: Value = serde_json::from_str(json).unwrap();
		let entity = parse(&mut world, member);
		LexiconRenderers::render(
			&mut world,
			entity,
			&RequestParts::default(),
			&Nsid::new_static("com.example.poll"),
		)
		.unwrap()
		.xmap(Value::from)
		.xmap(|value| serde_json::to_string(&value).unwrap())
		.xpect_eq(json);
	}

	/// A blob's url is the repo's `getBlob` on its PDS.
	#[beet_core::test]
	fn addresses_a_blob() {
		LexiconSource {
			did: Did::parse("did:plc:vgxy56shhs3t3b2mu2avizjf").unwrap(),
			pds: Url::parse("https://pds.example.com").unwrap(),
		}
		.blob_url(&Cid::raw(b"hello"))
		.to_string()
		.xpect_eq(format!(
			"https://pds.example.com/xrpc/com.atproto.sync.getBlob?cid={}&did=did:plc:vgxy56shhs3t3b2mu2avizjf",
			Cid::raw(b"hello").as_str()
		));
	}
}
