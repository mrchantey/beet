//! The instantiation entrypoints and the build walker.
//!
//! [`WorldTemplateExt::spawn_template`] and
//! [`EntityWorldMutTemplateExt::insert_template`]
//! are the only ways to instantiate a template. Each runs the synchronous build
//! walker over an [`EntityWorldMut`]: build the template into its entity, resolve
//! slots across the built subtree, then fire the lifecycle events
//! ([`SpawnTemplate`], then [`LoadTemplate`] once dependencies drain).
//!
//! Errors never escape as an `Err` and never panic: a failed build, slot
//! resolution, or load rides [`TemplateError`] on the root and surfaces through
//! [`LoadTemplate`] with `is_error: true`.

use crate::prelude::*;
use bevy::ecs::template::Template;

/// Opts a build-subtree template type out of [`Unpin`], so it escapes Bevy's
/// blanket `Template for T: Default + Clone + Unpin` impl and can supply its own
/// subtree-building [`Template`] impl while still deriving `Default`/`Clone`
/// (needed by the loader's `from_reflect`).
///
/// Uses Bevy's `SpecializeFromTemplate` specialization trick: the `Unpin` impl
/// is gated on a bound nothing satisfies, so the type is `!Unpin` for coherence
/// yet the compiler still accepts the impl. The `#[template]` derive emits this
/// for every function component.
///
/// ```
/// # use beet_core::prelude::*;
/// # use bevy::ecs::template::{Template, TemplateContext};
/// #[derive(Default, Clone)]
/// struct MyTemplate;
/// subtree_template!(MyTemplate);
/// impl Template for MyTemplate {
///     type Output = ();
///     fn build_template(&self, _: &mut TemplateContext) -> Result<()> { OK }
///     fn clone_template(&self) -> Self { self.clone() }
/// }
/// ```
#[macro_export]
macro_rules! subtree_template {
	($ty:ty) => {
		impl ::core::marker::Unpin for $ty where
			for<'a> [()]:
				$crate::exports::bevy::ecs::template::SpecializeFromTemplate
		{
		}
	};
}

/// Instantiation entrypoint on [`World`].
#[extend::ext(name=WorldTemplateExt)]
pub impl World {
	/// Spawns a root entity and builds `template` into it.
	///
	/// Runs the build walker: build into the root, resolve slots, fire
	/// [`SpawnTemplate`] then [`LoadTemplate`]. On success returns the root, like
	/// [`World::spawn`]. On failure the error rides [`TemplateError`] and
	/// [`LoadTemplate`] (`is_error: true`) *and* is returned here, its inner a
	/// [`CloneError`] shared across all three.
	fn spawn_template(
		&mut self,
		template: impl Template<Output = ()>,
	) -> Result<EntityWorldMut<'_>> {
		let root = self.spawn_empty().id();
		build_root(self, root, template)?;
		Ok(self.entity_mut(root))
	}
}

/// Instantiation entrypoint on [`EntityWorldMut`], building onto an existing
/// entity which becomes the root for lifecycle purposes.
#[extend::ext(name=EntityWorldMutTemplateExt)]
pub impl EntityWorldMut<'_> {
	/// Builds `template` into this entity, which becomes the template root.
	///
	/// Same lifecycle as [`WorldTemplateExt::spawn_template`], including the
	/// returned [`CloneError`] on failure.
	fn insert_template(
		&mut self,
		template: impl Template<Output = ()>,
	) -> Result<&mut Self> {
		let root = self.id();
		self.world_scope(|world| build_root(world, root, template))?;
		Ok(self)
	}
}

/// Deferred instantiation entrypoint on [`Commands`].
#[extend::ext(name=CommandsTemplateExt)]
pub impl Commands<'_, '_> {
	/// Queues a command that spawns a root and builds `template` into it,
	/// returning the [`EntityCommands`] for the root.
	///
	/// Naturally infallible: the build runs later, when the command flushes, so a
	/// failure rides [`TemplateError`] and [`LoadTemplate`] only (there is no
	/// return value to carry it). Reach for the async `spawn_template` when the
	/// failure must be awaited.
	fn spawn_template(
		&mut self,
		template: impl Template<Output = ()> + Send + 'static,
	) -> EntityCommands<'_> {
		let mut entity = self.spawn_empty();
		entity.queue(move |mut entity: EntityWorldMut| {
			let root = entity.id();
			entity.world_scope(move |world| {
				let _ = build_root(world, root, template);
			});
		});
		entity
	}
}

/// Deferred instantiation entrypoint on [`EntityCommands`].
#[extend::ext(name=EntityCommandsTemplateExt)]
pub impl EntityCommands<'_> {
	/// Queues a command building `template` into this entity, which becomes the
	/// template root. Naturally infallible, like
	/// [`CommandsTemplateExt::spawn_template`].
	fn insert_template(
		&mut self,
		template: impl Template<Output = ()> + Send + 'static,
	) -> &mut Self {
		self.queue(move |mut entity: EntityWorldMut| {
			let root = entity.id();
			entity.world_scope(move |world| {
				let _ = build_root(world, root, template);
			});
		});
		self
	}
}

/// The build walker, run synchronously on a freshly designated `root`.
///
/// 1. Build the template into the root, capturing any failure.
/// 2. Resolve slots across the built subtree (skipped on a build failure).
/// 3. Fire [`SpawnTemplate`] on the root (the post-build boundary).
/// 4. Drain the pending-dependency set, firing [`LoadTemplate`] when empty.
///
/// Structured so nested template nodes (the `DynamicTemplate` IR) and future
/// post-build passes attach at the slot/`SpawnTemplate` boundary without
/// rewriting this function.
fn build_root(
	world: &mut World,
	root: Entity,
	template: impl Template<Output = ()>,
) -> Result<(), CloneError> {
	// expose the root so a deferred dependency (asset, remote schema, remote
	// template) parks its pending id on it; restore any outer root afterwards so a
	// nested `insert_template` build does not leak its root to the parent.
	let previous_root = world.remove_resource::<TemplateBuildRoot>();
	world.insert_resource(TemplateBuildRoot(root));

	// caller content routed to the root before the build (eg the layout
	// middleware's transcluded body, a `SlotChild` portal), to anchor onto the
	// built layout once it exists.
	let pre_slot_children = root_slot_children(world, root);

	// step 1: build into the root, recording a failure rather than escaping it.
	let build_result = world
		.entity_mut(root)
		.build_template(&template)
		// step 1b: anchor the pre-build slot children onto the built layout's content
		// root, so the layout's `<Slot>`s receive them regardless of how the root
		// template built (clobbering the root's `Children`, or nesting the content
		// under a tag-less wrapper when the document is multi-root).
		.map(|()| anchor_pre_slot_children(world, root, &pre_slot_children))
		// step 2: slots, only when the build itself succeeded.
		.and_then(|()| resolve_slots(world, root));
	// a failure rides `TemplateError` + `LoadTemplate` *and* is returned, all
	// sharing one `CloneError`.
	let outcome = match build_result {
		Ok(()) => Ok(()),
		Err(error) => {
			let error = CloneError::new(error);
			world
				.entity_mut(root)
				.insert(TemplateError::new(error.clone()));
			Err(error)
		}
	};

	match previous_root {
		Some(previous) => world.insert_resource(previous),
		None => {
			world.remove_resource::<TemplateBuildRoot>();
		}
	}

	let mut entity = world.entity_mut(root);
	// step 3: the built signal / post-build phase boundary.
	entity.trigger(|entity| SpawnTemplate { entity });
	// step 4: fire LoadTemplate when nothing is pending.
	drain_pending_dependencies(&mut entity);

	outcome
}

/// The root's direct [`SlotChild`] children present before the build, ie caller
/// content routed to the root by the spawner (the layout middleware's portal).
fn root_slot_children(world: &World, root: Entity) -> Vec<Entity> {
	world
		.entity(root)
		.get::<Children>()
		.into_iter()
		.flat_map(Children::iter)
		.filter(|child| world.entity(*child).contains::<SlotChild>())
		.collect()
}

/// Anchor the layout's pre-build [`SlotChild`] content (the transcluded portal)
/// onto the built layout's *content root*, so the layout's `<Slot>`s receive it.
///
/// The root template may build in two ways that strand pre-added slot children:
/// - a template-invocation root (`<SiteLayout/>`) whose body lowers to
///   `children!`/`Children::spawn` *sets* the root's [`Children`], detaching them
///   (Bevy drops the [`ChildOf`] rather than despawning);
/// - a multi-root document (eg a leading `<!-- comment -->` before `<SiteLayout>`)
///   builds the content under a tag-less wrapper one level below the root, so a
///   child of the root sits in a *different composition scope* than the content's
///   `<Slot>`s and never matches.
///
/// Both are fixed by re-homing each pre-added slot child onto the content root:
/// the root itself if it built an [`Element`], else its sole element child (the
/// wrapper's content). This keeps the portal in the same scope as the layout's
/// slot targets, matching the additive single-root element-root case.
fn anchor_pre_slot_children(
	world: &mut World,
	root: Entity,
	pre_slot_children: &[Entity],
) {
	let content_root = content_root(world, root);
	for &child in pre_slot_children {
		// skip a child the build consumed (despawned) or already re-homed under the
		// content root; otherwise (detached, or stranded above the content) re-home it.
		let needs_anchor = world.get_entity(child).is_ok()
			&& world.entity(child).get::<ChildOf>().map(ChildOf::parent)
				!= Some(content_root);
		if needs_anchor {
			world.entity_mut(child).insert(ChildOf(content_root));
		}
	}
}

/// The entity a layout's `<Slot>`s live under: the root if it built an
/// [`Element`], else its sole [`Element`] child (a multi-root document nests the
/// real content under a tag-less wrapper, eg a leading comment before the layout
/// element), else the root unchanged.
fn content_root(world: &World, root: Entity) -> Entity {
	if world.entity(root).contains::<Element>() {
		return root;
	}
	world
		.entity(root)
		.get::<Children>()
		.into_iter()
		.flat_map(Children::iter)
		.find(|child| world.entity(*child).contains::<Element>())
		.unwrap_or(root)
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::ecs::template::TemplateContext;

	/// A build-into-subtree template: spawns a labelled child under the root.
	#[derive(Clone)]
	struct Child(&'static str);

	impl Template for Child {
		type Output = ();
		fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
			let name = self.0;
			let root = cx.entity.id();
			// SAFETY: only used to spawn an unrelated child entity.
			let world = unsafe { cx.entity.world_mut() };
			world.spawn((Name::new(name), ChildOf(root)));
			OK
		}

		fn clone_template(&self) -> Self { self.clone() }
	}

	#[beet_core::test]
	fn builds_into_root() {
		let mut world = TemplatePlugin::world();
		let root = world.spawn_template(Child("kid")).unwrap().id();
		let kid = world.entity(root).get::<Children>().unwrap()[0];
		world
			.entity(kid)
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("kid");
	}

	/// A nested, slotted template: builds a `<header>`/`<body>` subtree carrying
	/// slot targets, and routes caller content into them. Exercises the full
	/// walker: build, slot resolution, and lifecycle in one pass.
	#[derive(Clone)]
	struct Card;

	impl Template for Card {
		type Output = ();
		fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
			let root = cx.entity.id();
			// SAFETY: building this template's own subtree under the root.
			let world = unsafe { cx.entity.world_mut() };
			// structure: root -> header[Slot "title" (fallback)] , body[Slot]
			let header = world.spawn((Name::new("header"), ChildOf(root))).id();
			world.spawn((
				SlotTarget::named("title"),
				children![Name::new("fallback-title")],
				ChildOf(header),
			));
			let body = world.spawn((Name::new("body"), ChildOf(root))).id();
			world.spawn((SlotTarget::new(), ChildOf(body)));
			// caller content routed as direct slot children of the root.
			world.spawn((
				Name::new("the-body"),
				SlotChild::new(),
				ChildOf(root),
			));
			world.spawn((
				Name::new("the-title"),
				SlotChild::named("title"),
				ChildOf(root),
			));
			OK
		}
		fn clone_template(&self) -> Self { self.clone() }
	}

	#[beet_core::test]
	fn nested_slotted_template_end_to_end() {
		let mut world = TemplatePlugin::world();
		let spawn_count = Store::new(0);
		let load_state = Store::new(None);
		let sc = spawn_count;
		world.add_observer(move |_: On<SpawnTemplate>| sc.set(sc.get() + 1));
		let ls = load_state;
		world.add_observer(move |ev: On<LoadTemplate>| {
			ls.set(Some(ev.is_error))
		});

		let root = world.spawn_template(Card).unwrap().id();

		// helper: names of an entity's children in order.
		let child_names = |world: &World, entity: Entity| -> Vec<String> {
			world
				.entity(entity)
				.get::<Children>()
				.into_iter()
				.flat_map(|children| children.iter())
				.filter_map(|child| {
					world.entity(child).get::<Name>().map(|n| n.to_string())
				})
				.collect()
		};

		// tree: root -> [header, body].
		child_names(&world, root)
			.xpect_eq(vec!["header".to_string(), "body".to_string()]);
		let header = world.entity(root).get::<Children>().unwrap()[0];
		let body = world.entity(root).get::<Children>().unwrap()[1];

		// the title slot resolved to the caller's title, fallback dropped.
		let title_target = world.entity(header).get::<Children>().unwrap()[0];
		child_names(&world, title_target)
			.xpect_eq(vec!["the-title".to_string()]);
		// the default slot resolved to the caller's body.
		let body_target = world.entity(body).get::<Children>().unwrap()[0];
		child_names(&world, body_target).xpect_eq(vec!["the-body".to_string()]);
		// slot markers are stripped after resolution.
		world
			.entity(title_target)
			.contains::<SlotTarget>()
			.xpect_false();

		// lifecycle: SpawnTemplate once, LoadTemplate immediately with no error.
		spawn_count.get().xpect_eq(1);
		load_state.get().xpect_eq(Some(false));
	}

	#[beet_core::test]
	fn fires_spawn_then_load() {
		use crate::prelude::*;
		let mut world = TemplatePlugin::world();
		let spawn_count = Store::new(0);
		let load_state = Store::new(None);

		let sc = spawn_count.clone();
		world.add_observer(move |_: On<SpawnTemplate>| {
			sc.set(sc.get() + 1);
		});
		let ls = load_state.clone();
		world.add_observer(move |ev: On<LoadTemplate>| {
			ls.set(Some(ev.is_error));
		});

		world.spawn_template(Child("kid")).unwrap();

		// SpawnTemplate fires exactly once on the root.
		spawn_count.get().xpect_eq(1);
		// LoadTemplate fires immediately, no error, no pending deps.
		load_state.get().xpect_eq(Some(false));
	}

	#[beet_core::test]
	fn build_failure_rides_template_error() {
		#[derive(Clone)]
		struct Boom;
		impl Template for Boom {
			type Output = ();
			fn build_template(&self, _: &mut TemplateContext) -> Result<()> {
				bevybail!("boom")
			}
			fn clone_template(&self) -> Self { Self }
		}

		let mut world = TemplatePlugin::world();
		let load_error = Store::new(None);
		let le = load_error.clone();
		world.add_observer(move |ev: On<LoadTemplate>| {
			le.set(Some(ev.is_error));
		});

		let result = world.spawn_template(Boom);

		// the error rides the lifecycle and is also returned, never a panic.
		load_error.get().xpect_eq(Some(true));
		// (`EntityWorldMut` is not `Debug`, so take the error without `unwrap_err`)
		result.err().unwrap().to_string().xpect_contains("boom");
		// the root carries the same error via `TemplateError`.
		world
			.query::<&TemplateError>()
			.iter(&world)
			.count()
			.xpect_eq(1);
	}

	#[beet_core::test]
	fn pending_defers_load() {
		// a registered dependency defers LoadTemplate until it resolves.
		let mut world = TemplatePlugin::world();
		let load_fired = Store::new(false);
		let lf = load_fired;
		world.add_observer(move |_: On<LoadTemplate>| lf.set(true));

		// a template that registers one pending dependency on the root, parking
		// its id in a `Store` so the test can resolve it afterwards.
		#[derive(Clone)]
		struct Pending(Store<Option<PendingId>>);
		impl Template for Pending {
			type Output = ();
			fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
				let id = cx
					.entity
					.entry::<TemplatePending>()
					.or_default()
					.get_mut()
					.register();
				self.0.set(Some(id));
				OK
			}
			fn clone_template(&self) -> Self { self.clone() }
		}

		let id_slot = Store::new(None);
		let root = world.spawn_template(Pending(id_slot)).unwrap().id();

		// LoadTemplate is deferred while the dependency is outstanding.
		load_fired.get().xpect_false();

		// resolve it, then drain: now LoadTemplate fires.
		let id = id_slot.get().unwrap();
		let mut entity = world.entity_mut(root);
		entity.get_mut::<TemplatePending>().unwrap().resolve(id);
		drain_pending_dependencies(&mut entity);
		load_fired.get().xpect_true();
	}
}
