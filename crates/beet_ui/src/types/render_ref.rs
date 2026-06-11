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
/// The target is `None` when the holder is unresolved (eg a page-host slot
/// before any page is set), so a placeholder entity is never exposed; the
/// derived [`Default`] is that unresolved state, satisfying the scene template
/// machinery without a sentinel.
#[derive(
	Debug,
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component, Default)]
pub struct RenderRef(pub Option<Entity>);

impl RenderRef {
	/// Render `target` in place.
	pub fn new(target: Entity) -> Self { Self(Some(target)) }

	/// The referenced entity, or `None` when the holder is unresolved.
	pub fn target(&self) -> Option<Entity> { self.0 }
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
}
