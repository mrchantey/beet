//! The [`TemplatePlugin`], registering the template substrate's types.

use crate::prelude::*;

/// Registers the template lifecycle events, slot and pending markers, the
/// [`TemplateError`], the [`Unregistered`] marker, the
/// [`ReflectTemplate`] registry bridge, and the markup node types
/// ([`Element`], [`Attribute`], ..) so a built page dumps and reloads whole.
///
/// A minimal world built from this plugin can `spawn_template`. Mirrors
/// [`DocumentPlugin`] in style; the build walker and slot resolution are
/// synchronous over [`EntityWorldMut`]. The one system is the
/// [`sweep_dropped_pending`] backstop, resolving any [`PendingGuard`] dropped
/// unresolved so a lost dependency never hangs a load.
#[derive(Default)]
pub struct TemplatePlugin;

impl Plugin for TemplatePlugin {
	fn build(&self, app: &mut App) {
		app
			// ensure the type registry exists for a template's `install_types`.
			.init_resource::<AppTypeRegistry>()
			// the dropped-guard side channel, shared with every `PendingGuard`.
			.init_resource::<PendingDropQueue>()
			// slot markers, the error path, and the pending-dependency set.
			.register_type::<SlotTarget>()
			.register_type::<SlotChild>()
			.register_type::<TemplatePending>()
			// the marker holding what a loader could not resolve on an entity.
			.register_type::<Unregistered>()
			// a held template subtree, and `Disabled`, plain content an
			// author writes and a save keeps
			.register_type::<Prefab>()
			.register_type::<Disabled>()
			// another document instantiated here, and what a document takes
			.register_type::<Instance>()
			.register_type::<Prop>()
			.register_type::<HeldTemplate>()
			// the markup nodes every front-end authors: registered here rather
			// than by a renderer, so a page forked into a scene keeps its
			// elements and attributes whichever binary dumps it
			.register_type::<Element>()
			// a text node, and the whole of a plain text or foreign data
			// document
			.register_type::<Value>()
			// a script, and the event a `bx:on<event>` handler answers
			.register_type::<RunScript>()
			.register_type::<OnEvent>()
			.register_type::<Comment>()
			.register_type::<Doctype>()
			.register_type::<CData>()
			.register_type::<ProcessingInstruction>()
			.register_type::<Attribute>()
			.register_type::<AttributeOf>()
			.register_type::<Attributes>()
			// a prefab's subtree is held out of every query that does not
			// name it
			.install_types::<Held>()
			.add_systems(Update, sweep_dropped_pending);
		// what a package declares and its records are made of
		#[cfg(feature = "serde")]
		app.register_type::<LoadRoot>()
			.register_type::<Preload>()
			.register_type::<Provenance>()
			.register_type::<BlobRef>()
			.register_type::<Package>()
			.register_type::<Export>()
			.register_type::<Exports>()
			.register_type::<Dependency>()
			.register_type::<Dependencies>()
			.register_type::<Import>()
			.register_type::<Runtime>()
			.register_type::<ExportRuntimes>()
			.register_type::<Branch>()
			.register_type::<SourceBranch>()
			.register_type::<Files>()
			.register_type::<Imports>()
			.register_type::<Props>()
			.register_type::<Scenes>()
			.register_type::<LoadSets>();
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	fn world_can_spawn_template() {
		use bevy::ecs::template::Template;
		use bevy::ecs::template::TemplateContext;

		#[derive(Clone)]
		struct Noop;
		subtree_template!(Noop);
		impl Template for Noop {
			type Output = ();
			fn build_template(&self, _: &mut TemplateContext) -> Result<()> {
				OK
			}
			fn clone_template(&self) -> Self { Self }
		}

		let mut world = TemplatePlugin::world();
		// the minimal world spawns a template without panicking.
		world.spawn_template(Noop).unwrap();
	}
}
