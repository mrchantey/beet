use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct PropagateSignals;

pub struct SignalsPlugin;
impl Plugin for SignalsPlugin {
	// #[rustfmt::skip]
	fn build(&self, app: &mut App) {
		app.init_plugin(ApplySnippetsPlugin)
			.init_plugin(schedule_order_plugin)
			.init_resource::<DirtySignals>()
			.add_systems(
				PropagateSignals,
				(
					flush_signals,
					#[cfg(feature = "bevy_default")]
					propagate_text_signals,
					#[cfg(target_arch = "wasm32")]
					(
						update_dom_nodes,
						update_fragments,
						update_attribute_values,
					)
						.chain()
						.run_if(document_exists),
				)
					.chain(),
			);

		app.world_mut()
			.register_component_hooks::<SignalEffect>()
			.on_add(propagate_signal_effect);
	}
}

/// In bevy_default pass changed TextNode values to TextSpan
#[cfg(feature = "bevy_default")]
fn propagate_text_signals(
	mut query: Populated<(&mut TextSpan, &TextNode), Changed<TextNode>>,
) {
	for (mut span, text) in query.iter_mut() {
		**span = text.0.clone();
	}
}
