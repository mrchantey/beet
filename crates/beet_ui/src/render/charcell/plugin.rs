use super::*;
use crate::parse::PostParseTree;
#[cfg(feature = "tui")]
use crate::parse::RealtimeParsePlugin;
use crate::style::ResolveStylesSet;
use crate::style::StylePlugin;
use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
#[allow(unused)]
use bevy::ecs::schedule::common_conditions;

/// The single plugin a live, interactive terminal app adds.
///
/// Composes the charcell render pipeline ([`CharcellPlugin`]) with per-frame
/// repaint ([`RealtimeParsePlugin`]) and the reactive document chain
/// ([`DocumentUiPlugin`]). Each surface owns its own [`Pointer`] (required by
/// [`Terminal`]), so input routes per surface and many can coexist (one per SSH
/// session). The terminal lifecycle (input read, render, flush, restore) ships
/// with [`CharcellPlugin`] under the `tui` feature, so this is purely the
/// live-app composition.
///
/// Later tasks layer the input bridge (terminal bytes to bevy input) and the
/// hit-test (cursor to [`Pointer`] events) onto this plugin.
#[cfg(feature = "tui")]
#[derive(Default)]
pub struct CharcellTuiPlugin;

#[cfg(feature = "tui")]
impl Plugin for CharcellTuiPlugin {
	fn build(&self, app: &mut App) {
		// InputPlugin maintains ButtonInput<KeyCode>/<MouseButton> and registers the
		// input message types from the bridge's emissions; it works headless (no
		// winit), which is exactly what the terminal needs.
		if !app.is_plugin_added::<bevy::input::InputPlugin>() {
			app.add_plugins(bevy::input::InputPlugin);
		}
		// CursorMoved is a window message; with no WindowPlugin (no winit) the
		// terminal host registers it itself so the bridge can emit it.
		app.add_message::<bevy::window::CursorMoved>();
		// browser-like form behavior: editable controls and submit, so a live
		// TUI's `<form>`/`<select>` work out of the box.
		#[cfg(feature = "template")]
		app.init_plugin::<crate::prelude::FormPlugin>();
		app.init_plugin::<CharcellPlugin>()
			.init_plugin::<RealtimeParsePlugin>()
			.init_plugin::<crate::prelude::DocumentUiPlugin>()
			.init_plugin::<crate::prelude::FocusPlugin>()
			// pointer-driven `:hover` element state for the style cascade
			.init_plugin::<crate::prelude::PointerStatePlugin>()
			// the native `<select>` dropdown interaction
			.init_plugin::<SelectPlugin>()
			// terminal bytes to bevy input, before InputPlugin consumes the messages
			// this frame so `ButtonInput` reflects them immediately.
			.add_systems(
				PreUpdate,
				terminal_input_bridge.before(bevy::input::InputSystems),
			)
			// SIGWINCH-equivalent: poll the real tty size and resize stdio buffers.
			.add_systems(PreUpdate, resize_stdio_buffers)
			// hit-test + scroll input ride the bridged bevy mouse/key messages.
			// pointer_input runs first so a wheel's own hover is current when
			// scroll_input reads it; scrollbar_mouse claims gutter presses, others
			// fall through to pointer_input.
			.add_systems(
				Update,
				(
					(pointer_input, scroll_input).chain(),
					scrollbar_mouse,
					exit_on_ctrl_c,
				),
			)
			// clicking a `<summary>` toggles its `<details>` (the terminal stand-in
			// for the web's native disclosure).
			.add_observer(toggle_details_on_click);
	}
}

#[derive(Default)]
pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<StylePlugin>()
			.register_type::<crate::prelude::ScrollPosition>()
			.add_plugins((
				// layout + paint pipeline per buffer type; each only acts on entities
				// carrying its own buffer component, so registering both is harmless.
				buffer_plugin::<DoubleBuffer>,
				buffer_plugin::<FlexBuffer>,
			))
			// decorations run after styles resolve, the paint pipeline after both
			// decorations and the style transitions it reads displayed values from
			.configure_sets(
				PostParseTree,
				(
					DecorateSet.after(ResolveStylesSet),
					CharcellRenderSet
						.after(DecorateSet)
						.after(crate::style::AnimateStylesSet),
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

		// Terminal output: render the diffed buffer, overlay kitty rasters,
		// flush, restore on exit. Input is bridged to bevy by
		// `CharcellTuiPlugin` (the live app), not here.
		// `KittyGraphicsSupport` is a per-surface component (each terminal detects
		// its own client's capability), not a global resource; the terminal hosts
		// insert it (a `StdioTerminal` from the env, an SSH session from its pty).
		#[cfg(feature = "tui")]
		app.init_resource::<KittyPlacements>()
			// before the cascade so the `graphics` state (and the block box it
			// selects) resolves the same frame the raster attaches — no
			// one-frame alt-text flash.
			.add_systems(
				PostParseTree,
				(attach_kitty_images, render_image_errors)
					.chain()
					.before(ResolveStylesSet),
			)
			.register_type::<crate::prelude::Title>()
			.add_systems(
				PostParseTree,
				(
					render_terminal,
					place_kitty_images,
					// reflect each surface's `<title>` onto its terminal title bar
					// before the flush carries its bytes to stdout: collect the
					// per-surface title, then write a changed one.
					collect_terminal_titles,
					flush_terminal_titles,
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
			// re-clamp scroll offsets against the freshly laid-out geometry before
			// paint reads them to translate descendants.
			clamp_scroll_positions::<B>,
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
