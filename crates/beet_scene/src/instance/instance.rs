//! Activating an [`Instance`], and what an [`Activation`] records of it.
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;

/// One use of a document, on the use site it was built under.
///
/// Activating builds the document as the use site's child, its references and
/// state its own, and consumes the [`Instance`], a builder a save writes
/// nothing of. The use site stays the props document the clone's `@prop:`
/// bindings read, a prop the use did not pass taking the default its document
/// declares. The children authored under the instance are its slot children,
/// moved into the clone's `<slot>`s once everything it holds has arrived.
///
/// A use whose document is loaded activates where it is built, so templates in
/// it expand under whatever is building (a request's render); any other loads
/// it first and holds its root's [`Ready`] until it has.
#[derive(Debug, Component)]
pub struct Activation {
	/// The document's path in its package.
	pub path: RelPath,
	/// What the use site asked for.
	pub instance: Instance,
	/// The root of what was built.
	pub root: Entity,
	/// The document's entities by key.
	pub keys: TemplateEntityMap,
	/// The use site of the document this one was authored in, `None` for a
	/// use the document at the root of its tree authored.
	pub author: Option<Entity>,
}

/// The use site whose document is being built, so a use within it knows where
/// it was authored: a slot child sits under another document's use and is none
/// of its own.
#[derive(Clone, Copy, Resource)]
pub(crate) struct Activating(Entity);

impl Activation {
	/// Build `prefab` under the use site `site`, consuming its [`Instance`].
	///
	/// ## Errors
	/// Errors when `site` is used by the document it instantiates, however
	/// indirectly, or when the document does not build.
	pub fn activate(
		world: &mut World,
		site: Entity,
		prefab: &InstancePrefab,
		author: Option<Entity>,
	) -> Result<Entity> {
		Self::refuse_cycle(world, prefab, author)?;
		let template = prefab.reflect(world)?;
		let instance = world
			.entity_mut(site)
			.take::<Instance>()
			.unwrap_or_else(|| Instance::new(prefab.path()));
		// a prop the use does not pass reads the document's default
		let mut document = world
			.entity_mut(site)
			.take::<Document>()
			.unwrap_or_else(|| Document::new(Value::map()));
		if let Ok(props) = document.0.as_map_mut() {
			for prop in prefab.props() {
				if let (false, Some(default)) =
					(props.contains(&prop.name), &prop.default)
				{
					props.insert(prop.name.clone(), default.clone());
				}
			}
		}
		world.entity_mut(site).insert((document, PropsDocument));
		// the clone's own uses are authored here
		let root = world.spawn(ChildOf(site)).id();
		let outer = world.remove_resource::<Activating>();
		world.insert_resource(Activating(site));
		// recorded before it builds, so a use within answers its author
		world.entity_mut(site).insert(Activation {
			path: prefab.path().clone(),
			instance,
			root,
			keys: default(),
			author,
		});
		let keys = template.build_keyed(&mut world.entity_mut(root));
		match outer {
			Some(outer) => world.insert_resource(outer),
			None => {
				world.remove_resource::<Activating>();
			}
		}
		let keys =
			keys.map_err(|err| bevyhow!("in `{}`: {err}", prefab.path()))?;
		if let Some(mut activation) = world.get_mut::<Activation>(site) {
			activation.keys = keys;
		}
		root.xok()
	}

	/// Refuse a document authored within a use of itself, which would never
	/// finish building: recursion over data is a list's.
	fn refuse_cycle(
		world: &World,
		prefab: &InstancePrefab,
		author: Option<Entity>,
	) -> Result {
		let mut next = author;
		while let Some(site) = next {
			let Some(activation) = world.get::<Activation>(site) else {
				break;
			};
			if activation.path == *prefab.path() {
				bevybail!("`{}` instantiates itself", prefab.path());
			}
			next = activation.author;
		}
		Ok(())
	}
}

/// A document activated as a build of its own, the entity built into its use
/// site: `world.spawn_template(InstanceTemplate::new(prefab))`, a layout
/// around a page.
#[derive(Clone)]
pub struct InstanceTemplate(Arc<InstancePrefab>);

impl InstanceTemplate {
	/// Activate `prefab`.
	pub fn new(prefab: Arc<InstancePrefab>) -> Self { Self(prefab) }
}

impl Template for InstanceTemplate {
	type Output = ();

	fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
		let site = cx.entity.id();
		cx.entity.world_scope(|world| {
			Activation::activate(world, site, &self.0, None).map(|_| ())
		})
	}

	fn clone_template(&self) -> Self { self.clone() }
}

subtree_template!(InstanceTemplate);

/// Activate an [`Instance`] as it lands, unless a prefab holds it. One whose
/// document is loaded activates at once, under whatever is building; any other
/// parks content on the build root and activates when it has loaded.
pub(crate) fn activate_on_insert(
	ev: On<Insert<Instance>>,
	authors: Option<Res<Activating>>,
	mut commands: Commands,
) {
	let site = ev.entity;
	let author = authors.map(|author| author.0);
	commands.queue(move |world: &mut World| -> Result {
		let Some(instance) = world
			.get_entity(site)
			.ok()
			.and_then(|entity| entity.get::<Instance>().cloned())
		else {
			return Ok(());
		};
		if is_held(world, site) {
			return Ok(());
		}
		let document = instance.document;
		// resolved here where the tree is whole, else in the task, where it
		// will be
		let loaded = world
			.with_state::<PackageQuery, _>(|packages| {
				packages.resolve(site, &document).ok()
			})
			.and_then(|blob| world.resource::<PrefabCache>().get(&blob));
		if let Some(prefab) = loaded {
			// a build reports its own failure; the document's is the tree's
			if let Err(err) = Activation::activate(world, site, &prefab, author)
			{
				fail(world, site, err);
			}
			return Ok(());
		}
		let guard = TemplatePending::park_content(
			world,
			site,
			format!("<Instance document=\"{document}\">"),
		);
		world.entity_mut(site).run_async_local(
			async move |entity: AsyncEntity| {
				activate_when_loaded(entity, document, author, guard).await
			},
		);
		Ok(())
	});
}

/// Whether `entity` is content of a prefab, which clones it rather than
/// building it.
fn is_held(world: &World, entity: Entity) -> bool {
	let mut next = Some(entity);
	while let Some(entity) = next {
		if world.get::<Prefab>(entity).is_some() {
			return true;
		}
		next = world.get::<ChildOf>(entity).map(ChildOf::parent);
	}
	false
}

/// Load the document a use names and activate it, then resolve `guard`.
async fn activate_when_loaded(
	entity: AsyncEntity,
	document: DocPath,
	author: Option<Entity>,
	guard: PendingGuard,
) {
	let site = entity.id();
	let root = guard.root();
	let loaded = async {
		let resolved = document.clone();
		let blob = entity
			.with_state::<PackageQuery, Result<Blob>>(
				move |entity, packages| packages.resolve(entity, &resolved),
			)
			.await??;
		PrefabCache::load(entity.world(), &blob).await
	}
	.await;
	entity
		.world()
		.with(move |world: &mut World| {
			// torn down while its document was read
			if world.get_entity(site).is_err() {
				return guard.resolve(world);
			}
			let activated = loaded.and_then(|prefab| {
				// what it nests parks on the root the use itself waits for
				TemplateBuildRoot::scoped(world, root, |world| {
					Activation::activate(world, site, &prefab, author)
				})
			});
			if let Err(err) = activated {
				fail(
					world,
					root,
					bevyhow!("<Instance document=\"{document}\">: {err}"),
				);
			}
			guard.resolve(world);
		})
		.await;
}

/// Record a use that could not activate on the build root above it, which is
/// where a load reads what failed it.
fn fail(world: &mut World, entity: Entity, err: BevyError) {
	error!("{err}");
	let root = TemplateBuildRoot::resolve(world, entity);
	if let Ok(mut root) = world.get_entity_mut(root)
		&& !root.contains::<TemplateError>()
	{
		root.insert(TemplateError::new(CloneError::new(err)));
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A world activating instances, and a store of `documents`.
	async fn package(documents: &[(&str, &str)]) -> (World, BlobStore) {
		let world =
			(AsyncPlugin, DocumentPlugin, SceneStorePlugin).into_world();
		let store = BlobStore::new(InMemoryBlobStore::new());
		for (path, source) in documents {
			store
				.insert(&RelPath::new(path), source.to_string())
				.await
				.unwrap();
		}
		(world, store)
	}

	/// Spawn `markup`, the document at `path`, over `store` and settle it.
	async fn spawn(
		world: &mut World,
		store: &BlobStore,
		path: &str,
		markup: &str,
	) -> Entity {
		let body = BsxParser::body(world, markup).unwrap();
		let template =
			DynamicTemplate::reflect_at(world, &body, &RelPath::new(path))
				.unwrap();
		let root = world.spawn(PackageStore::new(store.clone())).id();
		world.entity_mut(root).insert_template(template).unwrap();
		settle(world).await;
		root
	}

	async fn settle(world: &mut World) {
		AsyncRunner::settle_async_tasks(world).await;
		world.update_local();
	}

	/// The tree under `entity` as tags and text, a wrapper transparent.
	fn outline(world: &World, entity: Entity) -> String {
		let children = || {
			world
				.get::<Children>(entity)
				.into_iter()
				.flat_map(|children| children.iter())
				.map(|child| outline(world, child))
				.collect::<String>()
		};
		match (world.get::<Element>(entity), world.get::<Value>(entity)) {
			(Some(element), _) => {
				format!("<{0}>{1}</{0}>", element.tag(), children())
			}
			(None, Some(value)) if world.get::<Children>(entity).is_none() => {
				value.to_string()
			}
			_ => children(),
		}
	}

	const CARD: &str = "<!-- a card --><section><h2>{@prop:title}</h2><slot/><footer><slot name=\"foot\">none</slot></footer></section>";

	#[beet_core::test]
	async fn props_and_slots() {
		let (mut world, store) = package(&[("widgets/Card.bsx", CARD)]).await;
		let root = spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<main>
				<Instance document="widgets/Card.bsx" props={{title:"One"}}><p>body</p><b slot="foot">foot</b></Instance>
				<Instance document="widgets/Card" props={{title:"Two"}}/>
			</main>"#,
		)
		.await;
		outline(&world, root).xpect_eq(
			"<main><section><h2>One</h2><p>body</p><footer><b>foot</b></footer></section>\
			 <section><h2>Two</h2><footer>none</footer></section></main>",
		);
		// the builder is consumed, and what it built recorded
		let site = world.get::<Children>(root).unwrap()[0];
		world.entity(site).contains::<Instance>().xpect_false();
		let activation = world.get::<Activation>(site).unwrap();
		activation.path.as_str().xpect_eq("widgets/Card.bsx");
		activation
			.keys
			.world(&EntityKey::ROOT)
			.unwrap()
			.xpect_eq(activation.root);
	}

	/// A bound prop follows its source, and one the use does not pass reads the
	/// default its document declares.
	#[beet_core::test]
	async fn bound_and_defaulted_props() {
		let (mut world, store) = package(&[(
			"Card.bsx",
			r#"<Prop name="kind" kind="string" default="plain"/><h2>{@prop:title}</h2><i>{@prop:kind}</i>"#,
		)])
		.await;
		let root = spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<main {Document({name: "Ada"})}><Instance document="./Card.bsx" props={{title:@doc:name}}/></main>"#,
		)
		.await;
		settle(&mut world).await;
		outline(&world, root).xpect_eq("<main><h2>Ada</h2><i>plain</i></main>");
		world
			.get_mut::<Document>(root)
			.unwrap()
			.0
			.as_map_mut()
			.unwrap()
			.insert("name", "Grace");
		world.update_local();
		world.update_local();
		outline(&world, root)
			.xpect_eq("<main><h2>Grace</h2><i>plain</i></main>");
	}

	/// A use within a document resolves from that document, its props its own,
	/// bound ones from the use it is within; a prop is of the document that
	/// wrote it, in content slotted into another as anywhere else.
	#[beet_core::test]
	async fn nested_instances() {
		let (mut world, store) = package(&[
			("widgets/Card.bsx", CARD),
			(
				"widgets/Pair.bsx",
				r#"<div><Instance document="./Card" props={{title:@prop:left}}><em>{@prop:title}</em><Instance document="./Card" props={{title:@prop:title}}><i>{@prop:left}</i></Instance></Instance><slot/></div>"#,
			),
		])
		.await;
		let root = spawn(
			&mut world,
			&store,
			"routes/page.bsx",
			r#"<main><Instance document="../widgets/Pair" props={{left:"L", title:"T"}}><p>tail</p></Instance></main>"#,
		)
		.await;
		settle(&mut world).await;
		outline(&world, root).xpect_eq(
			"<main><div><section><h2>L</h2><em>T</em><section><h2>T</h2><i>L</i><footer>none</footer></section><footer>none</footer></section><p>tail</p></div></main>",
		);
	}

	/// A document used in its own slot is no cycle; one used in itself is, and
	/// fails its root naming the way round.
	#[beet_core::test]
	async fn a_cycle_is_refused() {
		let (mut world, store) = package(&[
			("Card.bsx", CARD),
			("A.bsx", r#"<a><Instance document="./B"/></a>"#),
			("B.bsx", r#"<b><Instance document="./A"/></b>"#),
		])
		.await;
		let root = spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<main><Instance document="./Card"><Instance document="./Card"/></Instance></main>"#,
		)
		.await;
		world.get::<TemplateError>(root).xpect_none();
		let root = spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<main><Instance document="./A"/></main>"#,
		)
		.await;
		world
			.get::<TemplateError>(root)
			.unwrap()
			.error
			.to_string()
			.xpect_contains(
				"`A.bsx` instantiates `B.bsx` instantiates `A.bsx`",
			);
	}

	/// A document that is missing, or that its name does not tell from
	/// another, fails its root.
	#[beet_core::test]
	async fn names_one_document() {
		let (mut world, store) =
			package(&[("Card.bsx", "<a/>"), ("Card.md", "# a")]).await;
		for (document, message) in [
			("./Missing", "no document at `Missing`"),
			("./Card", "`Card.bsx`, `Card.md`"),
		] {
			let root = spawn(
				&mut world,
				&store,
				"main.bsx",
				&format!(r#"<main><Instance document="{document}"/></main>"#),
			)
			.await;
			world
				.get::<TemplateError>(root)
				.unwrap()
				.error
				.to_string()
				.xpect_contains(message);
		}
	}

	/// A document is read once and held, each use its own; a change to it in
	/// its store has the next use read it again.
	#[beet_core::test]
	async fn loads_once() {
		let (mut world, store) = package(&[("Card.bsx", "<i/>")]).await;
		let markup = r#"<main><Instance document="./Card"/></main>"#;
		spawn(&mut world, &store, "main.bsx", markup).await;
		store
			.insert(&RelPath::new("Card.bsx"), "<b/>")
			.await
			.unwrap();
		let root = spawn(&mut world, &store, "main.bsx", markup).await;
		outline(&world, root).xpect_eq("<main><i></i></main>");
		world.trigger(BlobEvent::new(
			store.clone(),
			RelPath::new("Card.bsx"),
			BlobEventKind::Changed,
		));
		world.flush();
		let root = spawn(&mut world, &store, "main.bsx", markup).await;
		outline(&world, root).xpect_eq("<main><b></b></main>");
	}

	/// A component whose default is minted, as a thread's id is.
	#[derive(Component, Reflect)]
	#[reflect(Component, Default)]
	struct Minted {
		id: u32,
	}

	impl Default for Minted {
		fn default() -> Self {
			static NEXT: core::sync::atomic::AtomicU32 =
				core::sync::atomic::AtomicU32::new(0);
			Self {
				id: NEXT.fetch_add(1, core::sync::atomic::Ordering::Relaxed),
			}
		}
	}

	/// A default is minted per use, never once for the document.
	#[beet_core::test]
	async fn each_use_has_its_own_defaults() {
		let (mut world, store) =
			package(&[("Item.bsx", "<li {Minted}/>")]).await;
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Minted>();
		spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<ul><Instance document="./Item"/><Instance document="./Item"/></ul>"#,
		)
		.await;
		let ids = world.query_once::<&Minted>();
		ids.len().xpect_eq(2);
		ids[0].id.xpect_not_eq(ids[1].id);
	}

	/// An instance under an unmet `bx:cfg` is never built, so its document is
	/// never read: the case that makes exclusion a mechanism of its own.
	#[beet_core::test]
	async fn an_excluded_instance_reads_nothing() {
		let (mut world, store) = package(&[]).await;
		let root = spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<main><div bx:cfg="feature:infra"><Instance document="./gated"/></div></main>"#,
		)
		.await;
		world.get::<TemplateError>(root).xpect_none();
		let tombstone = world.get::<Children>(root).unwrap()[0];
		world
			.entity(tombstone)
			.contains::<CfgExcluded>()
			.xpect_true();
	}

	/// An instance in a list's item is the item's to clone, and activates in
	/// each row.
	#[beet_core::test]
	async fn activates_per_row() {
		let (mut world, store) = package(&[("Item.bsx", "<li/>")]).await;
		let root = spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<ul {Document({items: [1, 2]})} bx:for="items"><Instance document="./Item"/></ul>"#,
		)
		.await;
		settle(&mut world).await;
		settle(&mut world).await;
		outline(&world, root).matches("<li>").count().xpect_eq(2);
	}

	/// A row's slot children fill the document its instance uses.
	#[beet_core::test]
	async fn a_row_fills_slots() {
		for loaded in [false, true] {
			let (mut world, store) = package(&[("Card.bsx", CARD)]).await;
			if loaded {
				spawn(
					&mut world,
					&store,
					"main.bsx",
					r#"<main><Instance document="./Card"/></main>"#,
				)
				.await;
			}
			let root = spawn(
				&mut world,
				&store,
				"main.bsx",
				r#"<ul {Document({items: [1, 2]})} bx:for="items"><Instance document="./Card" props={{title:"card"}}><b>row</b></Instance></ul>"#,
			)
			.await;
			settle(&mut world).await;
			settle(&mut world).await;
			outline(&world, root)
				.matches("<h2>card</h2><b>row</b><footer>")
				.count()
				.xpect_eq(2);
		}
	}

	/// A row's scope is its list's document alone: a binding in the row to the
	/// props reads the prop, and one to the list's document its item.
	#[beet_core::test]
	async fn a_row_scope_leaves_props_alone() {
		let (mut world, store) = package(&[
			("Card.bsx", CARD),
			(
				"List.bsx",
				r#"<ul {Document({items: [{name:"a"}, {name:"b"}]})} bx:for="items"><li>{@prop:name}{@doc:name}<Instance document="./Card" props={{title:@doc:name}}>{@prop:name}</Instance></li></ul>"#,
			),
			(
				"Props.bsx",
				r#"<ol {Document({name:"doc"})} {(ReactiveChildren, @prop:items)}><template><li>{@prop:name}{@doc:name}</li></template></ol>"#,
			),
		])
		.await;
		let root = spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<main><Instance document="./List" props={{name:"prop"}}/><Instance document="./Props" props={{items:[{name:"c"}]}}/></main>"#,
		)
		.await;
		settle(&mut world).await;
		settle(&mut world).await;
		let row = |name: &str| {
			format!(
				"<li>prop{name}<section><h2>{name}</h2>prop<footer>none</footer></section></li>"
			)
		};
		outline(&world, root)
			.xpect_contains(&format!("{}{}</ul><ol>", row("a"), row("b")))
			.xpect_contains("<li>cdoc</li>");
	}

	/// A document no use instantiates reads its own props' defaults, what it
	/// slots into another document included, and never that document's props.
	#[beet_core::test]
	async fn an_unused_document_reads_its_defaults() {
		let (mut world, store) = package(&[("Card.bsx", CARD)]).await;
		let root = spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<main><Prop name="who" default="Ada"/><h1>{@prop:who}</h1><Instance document="./Card" props={{title:@prop:who}}><em>{@prop:who}</em><i>{@prop:title}</i></Instance></main>"#,
		)
		.await;
		settle(&mut world).await;
		outline(&world, root).xpect_contains(
			"<h1>Ada</h1><section><h2>Ada</h2><em>Ada</em><i>null</i><footer>",
		);
	}

	/// A snapshot holds what an activation built and nothing of the reference,
	/// its bindings still reading the use they were built at.
	#[beet_core::test]
	async fn a_snapshot_holds_what_it_built() {
		let documents = [
			("Card.bsx", CARD),
			(
				"Pair.bsx",
				r#"<Instance document="./Card" props={{title:"card"}}><em>{@prop:name}</em></Instance>"#,
			),
		];
		let (mut world, store) = package(&documents).await;
		let root = spawn(
			&mut world,
			&store,
			"main.bsx",
			r#"<main><Instance document="./Pair" props={{name:"Ada"}}/></main>"#,
		)
		.await;
		settle(&mut world).await;
		let expected = "<main><section><h2>card</h2><em>Ada</em><footer>none</footer></section></main>";
		outline(&world, root).xpect_eq(expected);
		let saved = TemplateSaver::new()
			.with_entity_tree(&world, root)
			.save(&world, MediaType::Json)
			.unwrap();
		saved
			.as_utf8()
			.unwrap()
			.xref()
			.xpect_contains("PropsDocument")
			.xpect_contains("\"h2\"")
			.xnot()
			.xpect_contains("instance::Instance");

		let (mut world, _) = package(&[]).await;
		let body = DocParsers::of(&world).body(&saved).unwrap();
		let template = DynamicTemplate::reflect(&world, &body).unwrap();
		let root = world.spawn_template(template).unwrap().id();
		settle(&mut world).await;
		outline(&world, root).xpect_eq(expected);
		// the binding, not only the value it last read
		for mut document in world
			.query_filtered::<&mut Document, With<PropsDocument>>()
			.iter_mut(&mut world)
		{
			let props = document.0.as_map_mut().unwrap();
			if props.contains("name") {
				props.insert("name", "Grace");
			}
		}
		world.update_local();
		outline(&world, root).xpect_contains("<em>Grace</em>");
	}
}
