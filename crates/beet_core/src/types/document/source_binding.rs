//! One-way document-source binding: mirrors a field of another document into
//! this entity's [`Value`], the source half of a props binding chain.
//!
//! A binding-valued prop (`<Card title=@doc:field/>`) spawns a binding entity
//! carrying `(Value, SourceFieldRef, FieldRef)`: this component pulls the
//! caller's document field into the [`Value`], and the co-located [`FieldRef`]
//! syncs the `Value` with the props store, so the full chain is
//! `source field -> Value <-> props.title -> body bindings`. The document sync
//! handles the fan-out from the store to the body.
//!
//! Unlike [`FieldRef`] this binding never writes back to the source document,
//! and an entity carries at most one [`FieldRef`], which is what makes the
//! co-location possible: the `FieldRef` slot stays free for the sink.

use crate::prelude::*;
use bevy::ecs::component::ComponentId;

/// The entity a binding resolves against, mirroring [`DocumentPath`] naming.
///
/// Shared by [`SourceFieldRef`] (the document resolution subject) and
/// [`ReflectFieldRef`](crate::prelude::ReflectFieldRef) (the bound component's
/// entity).
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect, MapEntities)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BindingTarget {
	/// The binding entity itself.
	#[default]
	This,
	/// A specific entity, eg the element an attribute `Value` binds to.
	Entity(#[entities] Entity),
	/// A lazily resolved target: the nearest self-or-ancestor entity carrying
	/// the component named by this short type path, re-resolved each sync pass.
	///
	/// Produced by the lazy reserved BSX selectors (`@entity:PageRoot::`,
	/// `@entity:Router::`), whose marker entities may not exist or be attached at
	/// build time. Unresolvable targets are silently skipped until the marker
	/// appears in the ancestry.
	Reserved(SmolStr),
}

impl From<Entity> for BindingTarget {
	fn from(entity: Entity) -> Self { Self::Entity(entity) }
}

impl BindingTarget {
	/// The fixed target entity, `binding_entity` for [`Self::This`]. `None` for
	/// a [`Self::Reserved`] target, which needs [`Self::resolve`].
	pub fn fixed(&self, binding_entity: Entity) -> Option<Entity> {
		match self {
			Self::This => Some(binding_entity),
			Self::Entity(entity) => Some(*entity),
			Self::Reserved(_) => None,
		}
	}

	/// Fully resolve the target: fixed targets directly, a [`Self::Reserved`]
	/// target to the nearest self-or-ancestor entity carrying the named
	/// component, `None` when no such ancestor exists (yet). The walk follows
	/// [`ChildOf`], hopping an [`AttributeOf`] relation when no parent exists,
	/// so an attribute or props binding entity (outside the `ChildOf` tree)
	/// resolves through its owning element/store.
	///
	/// A matched marker carrying a [`LayoutContent`] (a layout root linked to its
	/// transcluded route content, installed by the router's layout wrap) resolves
	/// to that content instead, so a layout-head `@entity:PageRoot::` binding reads
	/// the route's `ArticleMeta` across the transclusion boundary. A
	/// self-referential render root has no such link and resolves to itself, as
	/// before.
	pub fn resolve(
		&self,
		world: &World,
		binding_entity: Entity,
	) -> Option<Entity> {
		use bevy::ecs::relationship::Relationship;
		match self {
			Self::Reserved(name) => {
				let component_id = component_id_by_short_path(world, name)?;
				// walk self-or-ancestors over the element + attribute tree (the
				// shared `ChildOf`-then-`AttributeOf` step) to the nearest holder of
				// the named component.
				ElementTraverseQuery::world_ancestors_inclusive(
					world,
					binding_entity,
				)
				.find_map(|entity| {
					let entity_ref = world.get_entity(entity).ok()?;
					entity_ref.contains_id(component_id).then(|| {
						// a render root linked to detached content (the layout case)
						// resolves into the content; a self-referential one stays put.
						entity_ref
							.get::<LayoutContent>()
							.map(|content| content.get())
							.unwrap_or(entity)
					})
				})
			}
			_ => self.fixed(binding_entity),
		}
	}
}

/// On a render-root entity: a one-directional link to the per-request route
/// content it transcludes, the seam a layout-head `@entity:PageRoot::` binding
/// follows to reach the route's metadata.
///
/// This is the **beet_core-side** transclusion edge, and the only one the
/// binding resolver here can follow: the structural `Portal` is a beet_ui type
/// sitting on a descendant slot child (the wrong direction), and the router's
/// `RequestContext` is a beet_router type invisible to this crate, so neither can
/// serve the head binding's content hop. The edge stays because the binding
/// re-resolves each sync pass to keep the `<title>` live and per-route across
/// client-side navigation; it would only be removable if the title never needed
/// to update live, which contradicts the live-page feature.
///
/// A layout builds detached and transcludes the route content by reference (a
/// `Portal` slot child), so the content carries no [`ChildOf`] edge to the
/// layout. The layout root's own render-root handle is self-referential (it
/// drives serialization of the layout tree) and cannot double as the content
/// pointer, so this is a distinct edge installed alongside it (in the router's
/// layout wrap). The reverse [`LayoutContentOf`] gives content -> render-root
/// traversal.
///
/// A self-referential render root (a fixed or per-request route that is its own
/// content) carries no [`LayoutContent`]: the reserved walk resolves to the
/// marker entity itself, the pre-transclusion behavior.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = LayoutContentOf)]
pub struct LayoutContent(#[entities] pub Entity);

impl LayoutContent {
	/// Link a render root to the `content` entity its reserved bindings read.
	pub fn new(content: Entity) -> Self { Self(content) }
}

/// On route content: the render roots transcluding it, the reverse edge of
/// [`LayoutContent`]. Gives content -> render-root traversal for free.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = LayoutContent)]
pub struct LayoutContentOf(Vec<Entity>);

impl LayoutContentOf {
	/// The render roots transcluding this content.
	pub fn holders(&self) -> &[Entity] { &self.0 }
}

/// The [`ComponentId`] of the component with this short type path: the type
/// registry first ([`AppTypeRegistry`]), falling back to a registered
/// component-info scan for unreflected types. `None` until the component has
/// been registered with the world (eg inserted at least once).
pub fn component_id_by_short_path(
	world: &World,
	name: &str,
) -> Option<ComponentId> {
	world
		.get_resource::<AppTypeRegistry>()
		.and_then(|registry| {
			registry
				.read()
				.get_with_short_type_path(name)
				.and_then(|registration| {
					world.components().get_id(registration.type_id())
				})
		})
		.or_else(|| {
			world
				.components()
				.iter_registered()
				.find(|info| info.name().shortname().to_string() == name)
				.map(|info| info.id())
		})
}

/// One-way source binding: mirrors a document field into this entity's
/// [`Value`], never writing back.
///
/// The [`document`](Self::document) resolves from the [`subject`](Self::subject)
/// entity rather than the binding entity, since a props binding entity lives
/// outside the `ChildOf` hierarchy (related to its template via `AttributeOf`):
/// the subject is the props store at the tag site, so [`DocumentPath::Ancestor`]
/// finds the caller's document (props stores are skipped, including the subject
/// itself).
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect, MapEntities)]
#[reflect(Component, MapEntities)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourceFieldRef {
	/// The path to the source document, resolved from [`subject`](Self::subject).
	pub document: DocumentPath,
	/// The path to the field within the source document.
	pub field_path: FieldPath,
	/// The entity the document resolves from, the binding entity by default.
	#[entities]
	pub subject: BindingTarget,
}

impl SourceFieldRef {
	/// Mirror the [`DocumentPath::Ancestor`] document's field into this entity's
	/// [`Value`].
	pub fn new<M>(field_path: impl IntoFieldPath<M>) -> Self {
		Self {
			document: DocumentPath::default(),
			field_path: field_path.into_field_path(),
			subject: BindingTarget::default(),
		}
	}

	/// Resolve the source document from another entity, eg the props store at
	/// the template's tag site.
	pub fn with_subject(mut self, subject: Entity) -> Self {
		self.subject = BindingTarget::Entity(subject);
		self
	}

	/// Resolve against a specific [`DocumentPath`] instead of the default
	/// [`DocumentPath::Ancestor`].
	pub fn with_document(mut self, document: DocumentPath) -> Self {
		self.document = document;
		self
	}
}

/// Mirror each [`SourceFieldRef`]'s source document field into its co-located
/// [`Value`], gated on the source document's change tick (plus the initial
/// sync on insert) and an inequality guard.
///
/// Runs after [`sync_document_to_local`] so a same-pass conflict with the
/// co-located [`FieldRef`]'s read path resolves source-wins, and before
/// [`sync_local_to_document`] so the mirrored value reaches the sink document
/// within the same pass. A missing document or field is silently skipped.
pub(super) fn sync_source_field_refs(
	resolver: DocumentResolver,
	scopes: ScopeQuery,
	docs: Query<Ref<Document>>,
	mut sources: Populated<(Entity, Ref<SourceFieldRef>, &mut Value)>,
) -> Result {
	for (entity, source, mut value) in sources.iter_mut() {
		// a reserved subject never occurs here: the props machinery only
		// produces fixed subjects, and lazy resolution is component-sync only.
		let Some(subject) = source.subject.fixed(entity) else {
			continue;
		};
		// a tag-site `@prop` source reads the *caller's* store: the subject is
		// the template entity carrying its own props store, which must not
		// shadow the one the tag was authored under.
		let doc_entity = match (&source.document, &source.subject) {
			(DocumentPath::Props, BindingTarget::Entity(_)) => {
				resolver.entity_above(subject, &source.document)
			}
			_ => resolver.entity(subject, &source.document),
		};
		let Ok(doc) = docs.get(doc_entity) else {
			continue;
		};
		// only an actually-changed source (or a fresh binding) reads
		if !doc.is_changed() && !source.is_added() {
			continue;
		}
		// scopes at the subject apply, matching the authored tag site
		let field_path =
			scopes.resolved_path(subject, &source.field_path, Some(doc_entity));
		let Ok(field_val) = doc.get_field_ref(&field_path) else {
			continue;
		};
		if *value != *field_val {
			*value = field_val.clone();
		}
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn mirrors_source_field() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "name": "Alice" }))).id();
		let binding = world
			.spawn((ChildOf(doc), Value::default(), SourceFieldRef::new("name")))
			.id();
		world.update_local();

		world
			.entity(binding)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("Alice".into()));

		// a source change re-mirrors
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "name": "Bob" });
		world.update_local();
		world
			.entity(binding)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("Bob".into()));
	}

	#[beet_core::test]
	fn never_writes_back() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "name": "Alice" }))).id();
		let binding = world
			.spawn((ChildOf(doc), Value::default(), SourceFieldRef::new("name")))
			.id();
		world.update_local();

		// a local edit never reaches the source document
		*world.entity_mut(binding).get_mut::<Value>().unwrap() =
			Value::Str("local".into());
		world.update_local();
		world
			.entity(doc)
			.get::<Document>()
			.unwrap()
			.get_field_ref(&[FieldSegment::key("name")])
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("Alice".into()));
	}

	#[beet_core::test]
	fn subject_resolves_from_props_store() {
		let mut world = DocumentPlugin::world();
		// user doc -> props store; the binding entity is outside the hierarchy
		let doc = world.spawn(Document::new(val!({ "name": "Alice" }))).id();
		let store = world
			.spawn((ChildOf(doc), Document::default(), PropsDocument))
			.id();
		let binding = world
			.spawn((
				Value::default(),
				// Ancestor from the store skips the store itself (PropsDocument)
				SourceFieldRef::new("name").with_subject(store),
			))
			.id();
		world.update_local();

		world
			.entity(binding)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("Alice".into()));
	}

	/// The props chain with the body field spawned *before* the binding entity:
	/// write-back iteration order must not matter, ie the body's freshly added
	/// Null `Value` (no signal) never clobbers the binding's same-pass write
	/// (the `sync_local_to_document` added-null guard).
	#[beet_core::test]
	fn source_chain_is_iteration_order_independent() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "name": "Alice" }))).id();
		let store = world
			.spawn((
				ChildOf(doc),
				Document::new(val!({ "title": null })),
				PropsDocument,
			))
			.id();
		// body first: its archetype (and write-back iteration slot) precedes
		// the binding entity's
		let body = world
			.spawn((
				ChildOf(store),
				Value::default(),
				FieldRef::new("title").with_document(DocumentPath::Props),
			))
			.id();
		world.spawn((
			Value::default(),
			SourceFieldRef::new("name").with_subject(store),
			FieldRef::new("title").with_document(DocumentPath::Entity(store)),
		));
		world.update_local();
		world.update_local();

		world
			.entity(body)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("Alice".into()));
	}

	/// The full props chain at the document level:
	/// `user doc -> binding Value <-> props store -> body field`.
	#[beet_core::test]
	fn source_chains_into_props_store() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "name": "Alice" }))).id();
		// the bound key is pre-seeded (as `resolve.rs` does), so a freshly added
		// body Value (Null) never racily seeds it via write-back
		let store = world
			.spawn((
				ChildOf(doc),
				Document::new(val!({ "title": null })),
				PropsDocument,
			))
			.id();
		// the binding entity: source `name` -> Value <-> props `title`
		world.spawn((
			Value::default(),
			SourceFieldRef::new("name").with_subject(store),
			FieldRef::new("title").with_document(DocumentPath::Entity(store)),
		));
		// a body field beneath the store reads props `title`
		let body = world
			.spawn((
				ChildOf(store),
				Value::default(),
				FieldRef::new("title").with_document(DocumentPath::Props),
			))
			.id();
		world.update_local();
		world.update_local();

		world
			.entity(body)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("Alice".into()));

		// reactive: a source document change reaches the body
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "name": "Bob" });
		world.update_local();
		world.update_local();
		world
			.entity(body)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("Bob".into()));
	}
}
