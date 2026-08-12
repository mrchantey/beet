//! The `<Template src>` include: pull another entry in at the include site.
//!
//! Generalizes the old remote-template front-end. A no-code entry composes from
//! other files, so no single giant `main.bsx`: `src` names another entry of any
//! format (bsx, json, ron), read and parsed through the unified
//! [`EntryTemplate`], then built where the `<Template>` tag sits. Installed
//! through the [`BsxTagResolvers`] seam, so it overrides the core stub when a
//! router app is present.
//!
//! The read is an async *pending* dependency, not a blocking call: the handler
//! parks a [`PendingId`] on the build root and spawns a task that resolves the
//! nearest ancestor [`BlobStore`] (the site store on the loaded root), reads `src`
//! through it, and builds the included entry at the include site, then resolves the
//! dependency so `LoadTemplate` proceeds. So an include never blocks the runtime
//! (single-threaded on wasm) and an S3-backed site composes the same way as a local
//! one. This reuses the same wiring [`TemplatePending::register_fetch`] gives the remote
//! front-ends in `beet_core`'s `remote.rs`.

use beet_core::prelude::*;
use beet_net::prelude::*;

/// Register the `<Template src="..">` include handler into the [`BsxTagResolvers`]
/// seam: a local `src` is read through the nearest ancestor [`BlobStore`] (the site
/// store composed on the loaded root) as an async pending dependency, and its
/// parsed entry built at the include site.
pub(crate) fn register_template_include(world: &mut World) {
	world.get_resource_or_init::<BsxTagResolvers>().insert(
		"Template",
		|el, entity| {
			let Some(src) = template_src(el) else {
				// `<Template>` with no `src` is a directives-only no-op.
				return Ok(());
			};
			// remote (`http(s)://`, `s3://`) includes resolve through the async
			// pending path too, but the transport is not yet wired (TODO).
			if is_remote(&src) {
				bevybail!(
					"remote `<Template src=\"{src}\">` includes are not yet \
					supported; use a local path"
				);
			}
			let target = entity.id();
			// park a structural dependency on the build root and spawn the async
			// read + build, so slot resolution and `LoadTemplate` wait for the
			// include and the runtime is never blocked. The ancestor store is
			// resolved inside the task, where the whole tree is built, so it is
			// reachable by ancestry.
			entity.world_scope(|world| -> Result {
				let (async_world, spawner, guard) = TemplatePending::register_fetch(
					world,
					target,
					PendingKind::Structural,
					format!("<Template src=\"{src}\">"),
				)?;
				spawner
					.spawn(resolve_include(async_world, src, target, guard));
				Ok(())
			})
		},
	);
}

/// Read + build a local `<Template src>` include, then resolve its pending
/// dependency so slot resolution and `LoadTemplate` proceed. Logs (rather than
/// panics) on failure, leaving the include site empty, mirroring the
/// remote-template resolver.
async fn resolve_include(
	async_world: AsyncWorld,
	src: SmolStr,
	target: Entity,
	guard: PendingGuard,
) {
	let root = guard.root();
	if let Err(err) = read_and_build(&async_world, &src, target, root).await {
		error!("`<Template src=\"{src}\">` include failed: {err}");
	}
	// resolve the dependency and drain the set: once the last structural
	// dependency lands, the deferred slot resolution runs over the settled tree.
	async_world
		.with(move |world: &mut World| guard.resolve(world))
		.await;
}

/// Resolve the include base (the nearest ancestor [`BlobStore`], the site store on
/// the loaded root), read `src` through it, then parse and build the entry at the
/// include site. A store-less tree is an error: every platform resolves includes
/// through the store, never the filesystem directly (there is none on wasm).
///
/// The build runs under [`TemplateBuildRoot::scoped`] with the *original* build
/// root, so a nested `<Template src>` inside the included entry parks its own
/// dependency on the same root: the root settles (and its slots resolve) only
/// once every level has built.
async fn read_and_build(
	async_world: &AsyncWorld,
	src: &str,
	target: Entity,
	root: Entity,
) -> Result {
	let store = async_world
		.entity(target)
		.with_state::<AncestorQuery<&BlobStore>, Result<BlobStore>>(
			|entity, stores| stores.get(entity).cloned(),
		)
		.await??;
	let media = store.get_media(&SmolPath::from(src)).await?;
	async_world
		.with(move |world: &mut World| -> Result {
			TemplateBuildRoot::scoped(world, root, |world| {
				let entry = EntryTemplate::from_bytes(world, &media)?;
				world.entity_mut(target).build_template(&entry)?;
				Ok(())
			})
		})
		.await
}

/// Whether `src` names a remote endpoint rather than a local path.
fn is_remote(src: &str) -> bool {
	src.starts_with("http://")
		|| src.starts_with("https://")
		|| src.starts_with("s3://")
}

/// The `src` string attribute of a `<Template>` element, if present.
fn template_src(el: &BsxElement) -> Option<SmolStr> {
	el.attributes.iter().find_map(|attr| {
		if attr.key != "src" {
			return None;
		}
		match &attr.value {
			AttrValue::Str(src) => Some(SmolStr::from(src.as_str())),
			_ => None,
		}
	})
}

#[cfg(test)]
mod test {
	use super::*;

	/// An entry that includes two `.bsx` files builds both at the include sites:
	/// each `<Template src>` becomes the included entry's root. The includes resolve
	/// through the ancestor store (an in-memory store seeded with the two entries, so
	/// the test is storage agnostic and runs on wasm), asynchronously (the pending
	/// path), so the build settles the async runtime before the children are asserted.
	#[beet_core::test]
	async fn includes_local_files() {
		// the include path is an async pending dependency, so the world needs the
		// async runtime alongside the template machinery.
		let mut world = (AsyncPlugin, TemplatePlugin).into_world();
		register_template_include(&mut world);

		// the include base: an in-memory store seeded with the two entries.
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("first.bsx"), "<section class=\"card\"/>")
			.await
			.unwrap();
		store
			.insert(&SmolPath::from("second.bsx"), "<article/>")
			.await
			.unwrap();

		let root = BsxTemplate::parse_entry(
			&world,
			"<main><Template src=\"first.bsx\"/><Template src=\"second.bsx\"/></main>",
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();
		// compose the store on the root so the includes resolve it by ancestry.
		world.entity_mut(root).insert(store);
		// the includes resolve as async pending dependencies; settle before asserting.
		AsyncRunner::settle_async_tasks(&mut world).await;

		// the two includes built at their sites, in order: a `section` and an `article`.
		let children: Vec<Entity> = world
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.collect();
		world
			.entity(children[0])
			.get::<Element>()
			.unwrap()
			.tag()
			.xpect_eq("section");
		world
			.entity(children[1])
			.get::<Element>()
			.unwrap()
			.tag()
			.xpect_eq("article");
	}

	/// Slot content *inside* an included entry resolves once the include settles:
	/// the included `<Fragment slot="x">` collapses into the included `<Slot
	/// name="x"/>`, leaving no routing markers behind. The resolution cannot run
	/// mid-`build_root` (the include has not built yet), so it must run when the
	/// pending set drains, just before the deferred [`LoadTemplate`].
	#[beet_core::test]
	async fn resolves_slots_in_included_content() {
		let mut world =
			(AsyncPlugin, TemplatePlugin, DocumentPlugin).into_world();
		register_template_include(&mut world);

		let store = BlobStore::temp();
		store
			.insert(
				&SmolPath::from("card.bsx"),
				"<main><Slot name=\"x\"/><Fragment slot=\"x\"><b/></Fragment></main>",
			)
			.await
			.unwrap();

		let root = BsxTemplate::parse_entry(
			&world,
			"<article><Template src=\"card.bsx\"/></article>",
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();
		world.entity_mut(root).insert(store);

		// record the slot state the deferred LoadTemplate observes on the root.
		let loaded_with_unresolved_slots = Store::new(None);
		let observed = loaded_with_unresolved_slots.clone();
		world.entity_mut(root).observe(
			move |ev: On<LoadTemplate>, slots: Query<&SlotChild>| {
				if ev.entity == root {
					observed.set(Some(slots.iter().count()));
				}
			},
		);
		AsyncRunner::settle_async_tasks(&mut world).await;

		// the include's slot content resolved: no routing markers survive anywhere.
		world
			.query::<&SlotChild>()
			.iter(&world)
			.count()
			.xpect_eq(0);
		world
			.query::<&SlotTarget>()
			.iter(&world)
			.count()
			.xpect_eq(0);
		// the `<b/>` collapsed into `<main>` at the slot's position.
		let tags = collect_tags(&mut world, root);
		tags.contains(&"b".to_string()).xpect_true();
		// and LoadTemplate fired with the slots already resolved.
		loaded_with_unresolved_slots.get().xpect_eq(Some(0));
	}

	/// A nested include (`first.bsx` including `second.bsx`) parks its pending
	/// dependency on the *original* build root, so the root's [`LoadTemplate`]
	/// fires exactly once, after the whole tree (both levels) has built.
	#[beet_core::test]
	async fn nested_include_defers_root_load() {
		let mut world =
			(AsyncPlugin, TemplatePlugin, DocumentPlugin).into_world();
		register_template_include(&mut world);

		let store = BlobStore::temp();
		store
			.insert(
				&SmolPath::from("first.bsx"),
				"<div><Template src=\"second.bsx\"/></div>",
			)
			.await
			.unwrap();
		store
			.insert(&SmolPath::from("second.bsx"), "<span/>")
			.await
			.unwrap();

		let root = BsxTemplate::parse_entry(
			&world,
			"<article><Template src=\"first.bsx\"/></article>",
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();
		world.entity_mut(root).insert(store);

		// record, per root LoadTemplate fire, whether the nested content existed.
		let fires = Store::new(Vec::<bool>::new());
		let recorded = fires.clone();
		world.entity_mut(root).observe(
			move |ev: On<LoadTemplate>, elements: Query<&Element>| {
				if ev.entity == root {
					let has_span =
						elements.iter().any(|element| element.tag() == "span");
					let mut all = recorded.get();
					all.push(has_span);
					recorded.set(all);
				}
			},
		);
		AsyncRunner::settle_async_tasks(&mut world).await;

		// the nested content built through both include levels.
		collect_tags(&mut world, root)
			.contains(&"span".to_string())
			.xpect_true();
		// the root loaded exactly once, and only after the nested include built.
		fires.get().xpect_eq(vec![true]);
	}

	/// Every element tag reachable under `root`.
	fn collect_tags(world: &mut World, root: Entity) -> Vec<String> {
		world.with_state::<Query<(Option<&Element>, Option<&Children>)>, _>(
			|query| {
				let mut tags = Vec::new();
				let mut stack = vec![root];
				while let Some(entity) = stack.pop() {
					let Ok((element, children)) = query.get(entity) else {
						continue;
					};
					if let Some(element) = element {
						tags.push(element.tag().to_string());
					}
					if let Some(children) = children {
						stack.extend(children.iter());
					}
				}
				tags
			},
		)
	}

	/// A `<Fragment slot="x">` forwards every child into the named slot even with
	/// the include resolver registered (it intercepts only `<Template>`), the
	/// `<HtmlDocument>` `slot="head"` shape in concrete form: each grouped child
	/// lands in the target slot, the transparent fragment leaving no wrapper.
	#[beet_core::test]
	fn fragment_forwards_children_into_slot() {
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		register_template_include(&mut world);

		let root = BsxTemplate::parse_entry(
			&world,
			"<main><Slot name=\"x\"/><Fragment slot=\"x\"><b/><i/></Fragment></main>",
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();

		// the slot collapsed and the fragment forwarded both children into `<main>`,
		// the transparent fragment leaving no wrapper element of its own.
		let tags = world
			.with_state::<Query<(Option<&Element>, Option<&Children>)>, _>(
				|query| {
					let mut tags = Vec::new();
					let mut stack = vec![root];
					while let Some(entity) = stack.pop() {
						let Ok((element, children)) = query.get(entity) else {
							continue;
						};
						if let Some(element) = element {
							tags.push(element.tag().to_string());
						}
						if let Some(children) = children {
							stack.extend(children.iter());
						}
					}
					tags
				},
			);
		tags.contains(&"b".to_string()).xpect_true();
		tags.contains(&"i".to_string()).xpect_true();
	}
}
