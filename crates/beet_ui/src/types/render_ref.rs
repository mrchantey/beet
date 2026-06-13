use beet_core::prelude::*;

/// Renders another entity in place, by reference, without reparenting it.
///
/// When a [`NodeWalker`] visits an entity carrying this component it recurses
/// into the referenced entity instead of the holder's own components and
/// [`Children`]. The referenced entity is neither owned nor moved, so it can be
/// a persistent, separately-managed subtree (eg fixed-route content shared
/// across requests) transcluded into an ephemeral layout.
///
/// This is distinct from author-facing `<slot>` composition (which lowers to
/// [`SceneProp`] props at macro time): the layout middleware needs to inject
/// already-spawned route content into a freshly-spawned document layout without
/// despawning it, by reference rather than by value.
///
/// The source half of the one-to-many [`RenderRefOf`] relationship: the holder
/// points at one content entity, and the content tracks every holder that
/// transcludes it (eg a layout root and a live page-host slot rendering the same
/// fixed route). The reverse edge is what lets a binding cross the transclusion
/// boundary (the layout-head `@comp$RenderRoot:` walk into the route content) and
/// the cascade inherit through it (the holder is the visual parent of the
/// content). Absence is the unresolved state, eg a page-host slot before any
/// page is set, so a placeholder entity is never exposed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = RenderRefOf)]
pub struct RenderRef(#[entities] pub Entity);

impl RenderRef {
	/// Render `target` in place.
	pub fn new(target: Entity) -> Self { Self(target) }

	/// The referenced entity.
	pub fn target(&self) -> Entity { self.0 }
}

/// The holders that render this entity in place by reference, the target half of
/// the [`RenderRef`] relationship.
///
/// The reverse edge of a transclusion: content transcluded into a layout (or a
/// live page host) has no [`ChildOf`] link to its holder, so this is how a walk
/// crosses from content up into the holder. A binding in a layout head resolving
/// `@comp$RenderRoot:` follows it from the layout root to the route content, and
/// the style cascade inherits from the holder through it.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = RenderRef)]
pub struct RenderRefOf(Vec<Entity>);

impl RenderRefOf {
	/// The holders rendering this entity in place.
	pub fn holders(&self) -> &[Entity] { &self.0 }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn walker_renders_referenced_entity() {
		let mut world = World::new();

		// content entity: <em>transcluded</em>
		let content = world.spawn(Element::new("em")).id();
		world.spawn((Value::Str("transcluded".into()), ChildOf(content)));

		// holder: a transparent entity that points at the content
		let root = world.spawn(RenderRef::new(content)).id();

		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
			.xpect_contains("<em>transcluded</em>");
	}

	#[beet_core::test]
	fn reverse_edge_tracks_holders() {
		let mut world = World::new();
		let content = world.spawn_empty().id();
		let holder = world.spawn(RenderRef::new(content)).id();
		// the relationship hook mirrors the holder onto the content's reverse edge.
		world
			.entity(content)
			.get::<RenderRefOf>()
			.unwrap()
			.holders()
			.xpect_eq(&[holder]);
	}
}
