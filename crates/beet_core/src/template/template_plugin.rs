//! The [`TemplatePlugin`], registering the template substrate's types.

use crate::prelude::*;

/// Registers the template lifecycle events, slot and pending markers, the
/// [`TemplateError`], the [`UnregisteredTag`] marker, and the
/// [`ReflectTemplate`] registry bridge.
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
			// ensure the type registry exists for `register_template`.
			.init_resource::<AppTypeRegistry>()
			// the dropped-guard side channel, shared with every `PendingGuard`.
			.init_resource::<PendingDropQueue>()
			// slot markers, the error path, and the pending-dependency set.
			.register_type::<SlotTarget>()
			.register_type::<SlotChild>()
			.register_type::<TemplatePending>()
			.register_type::<TemplatesLoaded>()
			// the inert marker an unresolvable tag leaves behind.
			.register_type::<UnregisteredTag>()
			.add_systems(Update, sweep_dropped_pending);
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
