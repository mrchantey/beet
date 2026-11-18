use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Renders a tree as one lexicon object, a member of the def it names,
/// registered in [`LexiconRenderers`] by installing it. Its [`Installer`]
/// impl, a supertrait so no renderer skips it, forwards to
/// [`LexiconRenderers::register_on`]. The registry clones the registered
/// instance for every render, as [`DocRenderers`] does.
pub trait LexiconRenderer: 'static + Send + Sync + Clone + Installer {
	/// The def this renderer writes, ie `org.beet.scene#body`.
	fn nsid(&self) -> Nsid;
	/// Render the tree at `cx.entity` as a member of [`nsid`](Self::nsid).
	fn render(&mut self, cx: &mut RenderContext) -> Result<OpenUnion>;
}

/// Every [`LexiconRenderer`] this world writes, keyed by NSID, the last
/// registered for a def replacing any before it.
///
/// Initialized empty by [`RenderPlugin`], each renderer registered by
/// installing it, `app.install(LeafletRenderer)`. One renderer type serving
/// several defs registers a configured instance per def.
#[derive(Default, Clone, Resource)]
pub struct LexiconRenderers {
	renderers: HashMap<Nsid, Arc<dyn ErasedLexiconRenderer>>,
}

impl LexiconRenderers {
	/// Register `renderer` for its def, replacing any renderer registered
	/// for it before.
	pub fn register(&mut self, renderer: impl LexiconRenderer) -> &mut Self {
		self.renderers.insert(renderer.nsid(), Arc::new(renderer));
		self
	}

	/// [`register`](Self::register) `renderer` in `app`'s registry, the body
	/// of every [`LexiconRenderer`]'s [`Installer`] impl.
	pub fn register_on(app: &mut App, renderer: impl LexiconRenderer) {
		app.world_mut()
			.get_resource_or_init::<Self>()
			.register(renderer);
	}

	/// Every registered def, sorted.
	pub fn available(&self) -> Vec<Nsid> {
		let mut available: Vec<Nsid> = self.renderers.keys().cloned().collect();
		available.sort();
		available
	}

	/// Render the tree at `entity`, answering `request`, as a member of
	/// `nsid`, settling the tree first as [`DocRenderers::render`] does. An
	/// entity holding an [`UnknownLexicon`] of `nsid` answers it verbatim
	/// before the registry is consulted; otherwise an unregistered `nsid`
	/// errors listing [`available`](Self::available), and a member naming
	/// another `$type` errors.
	pub fn render(
		world: &mut World,
		entity: Entity,
		request: &RequestParts,
		nsid: &Nsid,
	) -> Result<OpenUnion> {
		if let Some(unknown) = world
			.get::<UnknownLexicon>(entity)
			.filter(|unknown| unknown.r#type() == nsid)
		{
			return unknown.0.clone().xok();
		}
		let renderer = world
			.get_resource::<Self>()
			.ok_or_else(|| {
				bevyhow!(
					"no `LexiconRenderers` in this world: add the `RenderPlugin`"
				)
			})?
			.choose(nsid)?;
		DocRenderers::settle(world);
		let member = renderer.render(world, entity, request)?;
		if member.r#type() != nsid {
			bevybail!(
				"the `{nsid}` renderer answered a `{}` member",
				member.r#type()
			);
		}
		member.xok()
	}

	/// The renderer writing `nsid`, else an error listing the registered
	/// defs.
	fn choose(&self, nsid: &Nsid) -> Result<Arc<dyn ErasedLexiconRenderer>> {
		self.renderers.get(nsid).cloned().ok_or_else(|| {
			bevyhow!(
				"no lexicon renderer for `{nsid}`: the registered defs are {:?}",
				self.available()
					.iter()
					.map(Nsid::as_str)
					.collect::<Vec<_>>()
			)
		})
	}
}

/// The object-safe face of a [`LexiconRenderer`], which a registry can hold.
trait ErasedLexiconRenderer: 'static + Send + Sync {
	fn render(
		&self,
		world: &mut World,
		entity: Entity,
		request: &RequestParts,
	) -> Result<OpenUnion>;
}

impl<T: LexiconRenderer> ErasedLexiconRenderer for T {
	fn render(
		&self,
		world: &mut World,
		entity: Entity,
		request: &RequestParts,
	) -> Result<OpenUnion> {
		LexiconRenderer::render(
			&mut self.clone(),
			&mut RenderContext::new(world, entity, request),
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A renderer of the tests' own, writing the tree's plain text under
	/// `nsid`, tagged so a test can tell two registrations apart.
	#[derive(Clone)]
	struct Plain {
		nsid: &'static str,
		tag: &'static str,
	}

	impl LexiconRenderer for Plain {
		fn nsid(&self) -> Nsid { Nsid::new_static(self.nsid) }

		fn render(&mut self, cx: &mut RenderContext) -> Result<OpenUnion> {
			let mut text = PlainTextRenderer::default();
			cx.walk(&mut text);
			OpenUnion::new(
				self.nsid(),
				value!({ "text": (text.into_string()), "tag": (self.tag) }),
			)
		}
	}

	impl Installer for Plain {
		fn install(self, app: &mut App) {
			LexiconRenderers::register_on(app, self);
		}
	}

	/// A renderer answering a member of another def than it names.
	#[derive(Clone)]
	struct Liar;

	impl LexiconRenderer for Liar {
		fn nsid(&self) -> Nsid { Nsid::new_static("com.example.liar") }

		fn render(&mut self, _cx: &mut RenderContext) -> Result<OpenUnion> {
			OpenUnion::new(Nsid::new_static("com.example.other"), value!({}))
		}
	}

	impl Installer for Liar {
		fn install(self, app: &mut App) {
			LexiconRenderers::register_on(app, self);
		}
	}

	/// A world rendering `<p>hi</p>` through `installer`, and its root.
	fn page<M>(installer: impl Installer<M>) -> (World, Entity) {
		let mut app = world_ext::ui_app();
		app.install(installer);
		let mut world = core::mem::take(app.world_mut());
		let entity = world.spawn(rsx! { <p>"hi"</p> }).id();
		(world, entity)
	}

	fn render(world: &mut World, entity: Entity, nsid: &str) -> Result<Value> {
		LexiconRenderers::render(
			world,
			entity,
			&RequestParts::default(),
			&Nsid::parse(nsid)?,
		)
		.map(Value::from)
	}

	/// Each def answers through its own renderer.
	#[beet_core::test]
	fn dispatches_by_nsid() {
		let (mut world, entity) = page((
			Plain {
				nsid: "com.example.a",
				tag: "a",
			},
			Plain {
				nsid: "com.example.b",
				tag: "b",
			},
		));
		render(&mut world, entity, "com.example.a")
			.unwrap()
			.xpect_eq(value!({
				"$type": "com.example.a",
				"text": "hi\n",
				"tag": "a"
			}));
		render(&mut world, entity, "com.example.b")
			.unwrap()
			.as_map()
			.unwrap()
			.get("tag")
			.unwrap()
			.clone()
			.xpect_eq(Value::from("b"));
	}

	/// An unregistered def names the registered ones.
	#[beet_core::test]
	fn unregistered_lists_the_registered() {
		let (mut world, entity) = page(Plain {
			nsid: "com.example.a",
			tag: "a",
		});
		render(&mut world, entity, "com.example.unknown")
			.unwrap_err()
			.to_string()
			.xpect_contains("com.example.unknown")
			.xpect_contains("com.example.a");
	}

	/// The renderer registered last for a def answers it.
	#[beet_core::test]
	fn last_registration_wins() {
		let (mut world, entity) = page((
			Plain {
				nsid: "com.example.a",
				tag: "first",
			},
			Plain {
				nsid: "com.example.a",
				tag: "last",
			},
		));
		render(&mut world, entity, "com.example.a")
			.unwrap()
			.as_map()
			.unwrap()
			.get("tag")
			.unwrap()
			.clone()
			.xpect_eq(Value::from("last"));
		// replaced, not registered beside it, and sorted with the built-ins
		world
			.resource::<LexiconRenderers>()
			.available()
			.xpect_eq(vec![Nsid::new_static("com.example.a"), SceneBody::NSID]);
	}

	/// A member naming another def than the one asked for is refused.
	#[beet_core::test]
	fn refuses_a_member_of_another_def() {
		let (mut world, entity) = page(Liar);
		render(&mut world, entity, "com.example.liar")
			.unwrap_err()
			.to_string()
			.xpect_contains("com.example.other");
	}
}
