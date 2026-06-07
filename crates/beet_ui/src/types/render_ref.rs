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
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component, Default)]
pub struct RenderRef(pub Entity);

impl RenderRef {
	pub fn new(target: Entity) -> Self { Self(target) }
}

/// A placeholder target, required so [`RenderRef`] satisfies the scene template
/// machinery (`template_value`). The real target is cloned into the patch, so
/// this default is never the value actually inserted.
impl Default for RenderRef {
	fn default() -> Self { Self(Entity::PLACEHOLDER) }
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
