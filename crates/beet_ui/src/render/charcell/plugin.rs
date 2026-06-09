use super::*;
use crate::parse::PostParseTree;
use crate::style::ResolveStylesSet;
use crate::style::StylePlugin;
use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
#[allow(unused)]
use bevy::ecs::schedule::common_conditions;

#[derive(Default)]
pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<StylePlugin>()
			.add_plugins((
				// layout + paint pipeline per buffer type; each only acts on entities
				// carrying its own buffer component, so registering both is harmless.
				buffer_plugin::<DoubleBuffer>,
				buffer_plugin::<FlexBuffer>,
			))
			// decorations run after styles resolve, the paint pipeline after them
			.configure_sets(
				PostParseTree,
				(
					DecorateSet.after(ResolveStylesSet),
					CharcellRenderSet.after(DecorateSet),
				),
			)
			// post-resolve decorations consumed by the charcell paint pipeline
			.add_systems(
				PostParseTree,
				(
					apply_hyperlinks,
					apply_markers,
					apply_table_vertical_borders,
					apply_disclosure,
				)
					.in_set(DecorateSet),
			);


		// Terminal-specific systems: input, render, flush.
		#[cfg(feature = "terminal")]
		app.add_observer(exit_ctrl_c)
			.add_systems(PreUpdate, terminal_events)
			.add_systems(
				PostParseTree,
				(
					render_terminal,
					flush_terminals,
					restore_terminals
						.run_if(common_conditions::on_message::<AppExit>),
				)
					.chain()
					.after(paint_nodes::<DoubleBuffer>)
					.in_set(CharcellRenderSet),
			);
	}
}

/// Register the charcell `prepare → measure → layout → paint` pipeline for
/// buffer type `B` in the [`PostParseTree`] schedule's [`CharcellRenderSet`].
fn buffer_plugin<B: Component<Mutability = Mutable> + AsBuffer>(app: &mut App) {
	app.add_systems(
		PostParseTree,
		(
			prepare_charcell_tree::<B>,
			measure_nodes::<B>,
			layout_nodes::<B>,
			paint_nodes::<B>,
		)
			.chain()
			.in_set(CharcellRenderSet),
	);
}

/// [`PostParseTree`] set for post-resolve structural decorations: list/quote
/// markers and hyperlinks. A natural extension point — add your own
/// [`Marker`]-inserting systems here (see [`heading_hash_markers`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct DecorateSet;

/// [`PostParseTree`] set containing node layout, terminal render, and flush.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct CharcellRenderSet;
