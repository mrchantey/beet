//! The [`TemplatePlugin`], registering the template substrate's types.

use crate::prelude::*;

/// Registers the template lifecycle events, slot and pending markers, the
/// [`TemplateError`], and the [`ReflectTemplate`] registry bridge.
///
/// A minimal world built from this plugin can `spawn_template`. Mirrors
/// [`DocumentPlugin`] in style; the build walker and slot resolution are
/// synchronous over [`EntityWorldMut`], so this plugin registers types and the
/// [`AppTypeRegistry`] (via Bevy's reflect plumbing) but installs no
/// load-bearing systems.
#[derive(Default)]
pub struct TemplatePlugin;

impl Plugin for TemplatePlugin {
	fn build(&self, app: &mut App) {
		app
			// ensure the type registry exists for `register_template`.
			.init_resource::<AppTypeRegistry>()
			// slot markers, the error path, and the pending-dependency set.
			.register_type::<SlotTarget>()
			.register_type::<SlotChild>()
			.register_type::<TemplatePending>();
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
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
		world.spawn_template(Noop);
	}
}
