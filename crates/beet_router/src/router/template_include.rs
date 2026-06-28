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
//! one. This reuses the same wiring [`register_pending_fetch`] gives the remote
//! front-ends in `beet_core`'s `remote.rs`.

use beet_core::prelude::*;
use beet_net::prelude::*;

/// Register the `<Template src="..">` include handler into the [`BsxTagResolvers`]
/// seam: a local `src` is read through the nearest ancestor [`BlobStore`] (the site
/// store composed on the loaded root) as an async pending dependency, and its
/// parsed entry built at the include site.
pub fn register_template_include(world: &mut World) {
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
			// park a pending dependency on the build root and spawn the async read +
			// build, so `LoadTemplate` waits for the include and the runtime is never
			// blocked. The ancestor store is resolved inside the task, where the whole
			// tree is built, so it is reachable by ancestry.
			entity.world_scope(|world| -> Result {
				let (async_world, spawner, root, pending_id) =
					register_pending_fetch(world, target)?;
				spawner.spawn(resolve_include(
					async_world,
					src,
					target,
					root,
					pending_id,
				));
				Ok(())
			})
		},
	);
}

/// Read + build a local `<Template src>` include, then resolve its pending
/// dependency so `LoadTemplate` proceeds. Logs (rather than panics) on failure,
/// leaving the include site empty, mirroring the remote-template resolver.
async fn resolve_include(
	async_world: AsyncWorld,
	src: SmolStr,
	target: Entity,
	root: Entity,
	pending_id: PendingId,
) {
	if let Err(err) = read_and_build(&async_world, &src, target).await {
		error!("`<Template src=\"{src}\">` include failed: {err}");
	}
	// resolve the dependency and drain the set, firing `LoadTemplate` once settled.
	async_world
		.with(move |world: &mut World| {
			let mut root_entity = world.entity_mut(root);
			if let Some(mut pending) = root_entity.get_mut::<TemplatePending>()
			{
				pending.resolve(pending_id);
			}
			drain_pending_dependencies(&mut root_entity);
		})
		.await;
}

/// Resolve the include base (the nearest ancestor [`BlobStore`], the site store on
/// the loaded root), read `src` through it, then parse and build the entry at the
/// include site. A store-less tree is an error: every platform resolves includes
/// through the store, never the filesystem directly (there is none on wasm).
async fn read_and_build(
	async_world: &AsyncWorld,
	src: &str,
	target: Entity,
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
			let entry = EntryTemplate::from_bytes(world, &media)?;
			world.entity_mut(target).build_template(&entry)?;
			Ok(())
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
