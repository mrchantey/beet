//! The [`TemplatePlugin`], registering the template substrate's types.

use crate::prelude::*;
#[cfg(feature = "template_serde")]
use bevy::ecs::query::Allow;

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
			// ensure the type registry exists for a template's `install_type`.
			.init_resource::<AppTypeRegistry>()
			// the dropped-guard side channel, shared with every `PendingGuard`.
			.init_resource::<PendingDropQueue>()
			// slot markers, the error path, and the pending-dependency set.
			.register_type::<SlotTarget>()
			.register_type::<SlotChild>()
			.register_type::<TemplatePending>()
			.register_type::<TemplatesLoaded>()
			// the marker holding what a loader could not resolve on an entity.
			.register_type::<Unregistered>()
			// a held template subtree, whose `Disabled` is never content
			.register_type::<Prefab>()
			.register_type::<Disabled>()
			.register_type_data::<Disabled, ReflectDerived>()
			// the markup nodes every front-end authors: registered here rather
			// than by a renderer, so a page forked into a scene keeps its
			// elements and attributes whichever binary dumps it
			.register_type::<Element>()
			// a text node, and the whole of a plain text or foreign data
			// document
			.register_type::<Value>()
			// a script, and the event a `bx:<event>` handler answers
			.register_type::<RunScript>()
			.register_type::<OnEvent>()
			.register_type::<Comment>()
			.register_type::<Doctype>()
			.register_type::<CData>()
			.register_type::<ProcessingInstruction>()
			.register_type::<Attribute>()
			.register_type::<AttributeOf>()
			.register_type::<Attributes>()
			.add_systems(Update, sweep_dropped_pending);
		#[cfg(feature = "template_serde")]
		app.add_observer(hold_loaded_prefabs);
	}
}

/// Hold every prefab a body load spawned, whose `Disabled` no body carries.
#[cfg(feature = "template_serde")]
fn hold_loaded_prefabs(
	ev: On<LoadTemplateSerde>,
	prefabs: Query<Entity, (With<Prefab>, Allow<Disabled>)>,
	mut commands: Commands,
) {
	for prefab in ev
		.entities
		.iter()
		.filter(|entity| prefabs.contains(**entity))
	{
		let prefab = *prefab;
		commands.queue(move |world: &mut World| Prefab::hold(world, prefab));
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
