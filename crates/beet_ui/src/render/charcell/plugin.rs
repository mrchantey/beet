use super::*;
use crate::parse::PostParseTree;
#[cfg(feature = "tui")]
use crate::parse::RealtimeParsePlugin;
use crate::prelude::MediaViewport;
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
			// bind every terminal surface to the app-wide color scheme so a TUI app
			// is themed without each host opting in (the terminal analogue of the
			// router's `page_classes`).
			.add_systems(PreUpdate, sync_terminal_color_scheme)
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
			.add_observer(toggle_details_on_click)
			// clicking a control carrying `aria-controls` toggles `aria-hidden`
			// on its target (the ARIA disclosure pattern, eg the menu button
			// collapsing the sidebar rail).
			.add_observer(toggle_aria_controls_on_click);
		// the sidebar rail's breakpoint seeding — the native twin of
		// `sidebar.js`'s init/resize wiring (the menu-button click rides the
		// `aria-controls` observer above). After the viewport sync so a resize
		// seeds the same frame the cascade re-evaluates the width rules.
		#[cfg(feature = "template")]
		app.add_systems(
			PostParseTree,
			crate::widgets::sync_sidebar_breakpoint
				.after(sync_media_viewport::<DoubleBuffer>)
				.before(ResolveStylesSet),
		);
	}
}

/// Bind every terminal surface to the app-wide colour scheme
/// ([`Theme::scheme`](crate::style::material::Theme), dark by default), so a TUI
/// scene loaded with no router (eg `--main`) is themed like a routed site. This is
/// the terminal analogue of the router's `page_classes`: it applies the session
/// scheme to the render root via the same [`ColorScheme`](crate::style::ColorScheme)
/// handle (`sync_color_scheme` mirrors it onto the `.dark-scheme`/`.light-scheme`
/// class), rather than re-implementing the cascade.
///
/// It *tracks* the live `Theme`, so seeding a `Theme`, a `<Theme scheme=..>`, or a
/// `--color-scheme` argument re-themes the running app rather than being snapshot
/// at spawn.
///
///
/// TODO This is a hack because TuiThreadChat bypasses TuiServer,
/// remove once we implement .agents/plans/agnostic-thread-ui.md
#[cfg(feature = "tui")]
fn sync_terminal_color_scheme(
	theme: Option<Res<crate::style::material::Theme>>,
	mut commands: Commands,
	surfaces: Query<
		(Entity, Option<&crate::style::ColorScheme>),
		With<Terminal>,
	>,
) {
	let scheme = theme.map(|theme| theme.scheme).unwrap_or_default();
	for (entity, current) in surfaces.iter() {
		if current != Some(&scheme) {
			commands.entity(entity).insert(scheme);
		}
	}
}

#[derive(Default)]
pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<StylePlugin>()
			.register_type::<crate::prelude::ScrollPosition>()
			.register_type::<MediaViewport>()
			.add_plugins((
				// layout + paint pipeline per buffer type; each only acts on entities
				// carrying its own buffer component, so registering both is harmless.
				buffer_plugin::<DoubleBuffer>,
				buffer_plugin::<FlexBuffer>,
			))
			// surface size → `MediaViewport`, before the cascade reads it (and
			// before `resolve_styles`'s `Changed` trigger scans it this frame)
			.add_systems(
				PostParseTree,
				(
					sync_media_viewport::<DoubleBuffer>,
					sync_media_viewport::<FlexBuffer>,
				)
					.before(ResolveStylesSet),
			)
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
			.add_message::<crate::prelude::CopyToClipboard>()
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
					// write any pending OSC-52 clipboard copies before the flush.
					flush_clipboard,
					flush_terminals,
					restore_terminals
						.run_if(common_conditions::on_message::<AppExit>),
				)
					.chain()
					.after(paint_nodes::<DoubleBuffer>)
					.in_set(CharcellRenderSet),
			);

		// transient toasts (the clipboard-copy confirmation `flush_clipboard`
		// pops) plus their self-despawn timer, co-located with `flush_clipboard`
		// so any router/live app that can copy can also expire the toast.
		#[cfg(all(feature = "tui", feature = "template"))]
		app.init_plugin::<crate::prelude::ToastPlugin>();
	}
}

/// Px a terminal cell is worth to a width-gated media breakpoint
/// ([`MediaQuery::MaxWidth`](crate::prelude::MediaQuery::MaxWidth)).
///
/// A cell is one character, while 16px of proportional web text averages
/// roughly two, so mapping a cell to a full 16px rem makes the terminal look
/// twice as spacious to a breakpoint as it reads — narrow layouts would
/// persist into genuinely cramped column counts (the sidebar's 1024px
/// collapse landed at 64 columns). Tuned denser so that collapse lands at 90
/// columns; every px breakpoint shifts consistently.
const MEDIA_PX_PER_CELL: f32 = 1024.0 / 90.0;

/// Mirror each surface buffer's size onto its required [`MediaViewport`]
/// ([`MEDIA_PX_PER_CELL`] px per cell), the context width-gated media rules
/// resolve against. `set_if_neq`, so paint's per-frame buffer writes never
/// dirty the cascade — only a real resize fires `Changed<MediaViewport>`,
/// which `resolve_styles` picks up to re-cascade the surface's tree, ahead of
/// which this is ordered.
fn sync_media_viewport<B: Component + AsBuffer>(
	mut buffers: Query<(&B, &mut MediaViewport)>,
) {
	for (buffer, mut viewport) in buffers.iter_mut() {
		viewport.set_if_neq(MediaViewport::new(
			buffer.size().x as f32 * MEDIA_PX_PER_CELL,
			buffer.size().y as f32 * MEDIA_PX_PER_CELL,
		));
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
