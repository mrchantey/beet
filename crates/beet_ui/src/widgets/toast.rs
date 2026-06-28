//! A transient [`Toast`] overlay: a styled box that floats above all page
//! content for a moment then despawns itself.
//!
//! Authored as `<Toast>"Saved"</Toast>` markup, or popped imperatively with
//! [`Toast::show`] (eg "Copied to clipboard" after a clipboard write). The
//! overlay paints on top via the `.toast` rule's [`Position::Fixed`] +
//! high z-index (see [`classes::toast`](crate::style::material::classes::toast)),
//! and the [`DespawnAfter`] timer removes it after [`Toast::DURATION`].
use crate::prelude::*;
use beet_core::prelude::*;

/// Despawns its entity once a [`Timer`] elapses, ticked by [`despawn_after`]
/// from [`Res<Time>`]. Reusable for any transient entity, not toast-specific.
#[derive(Debug, Clone, Component)]
pub struct DespawnAfter(Timer);

impl DespawnAfter {
	/// Despawn the entity `duration` from now (a one-shot timer).
	pub fn new(duration: Duration) -> Self {
		Self(Timer::new(duration, TimerMode::Once))
	}
}

/// Tick every [`DespawnAfter`] and despawn the entities whose timer finished
/// this frame. Driven by [`Res<Time>`]; mirrors the timer-tick pattern in
/// `animate_visual_transitions`.
pub fn despawn_after(
	time: Res<Time>,
	mut commands: Commands,
	mut query: Query<(Entity, &mut DespawnAfter)>,
) {
	for (entity, mut despawn) in query.iter_mut() {
		despawn.0.tick(time.delta());
		if despawn.0.is_finished() {
			commands.entity(entity).despawn();
		}
	}
}

/// A transient overlay box showing a short message, painted above all page
/// content and self-despawning after [`Toast::DURATION`].
///
/// A marker on a `<div class="toast">`: authorable directly as
/// `<Toast>"msg"</Toast>` (its children are the message) and queryable, so
/// [`show`](Self::show) can keep at most one toast per surface. The `.toast`
/// rule fixes it above the page; pair it with a [`DespawnAfter`] to auto-remove.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Element = Element::new("div"), Classes = Classes::new([classes::TOAST]))]
pub struct Toast;

impl Toast {
	/// How long a toast lingers before [`despawn_after`] removes it.
	pub const DURATION: Duration = Duration::from_secs(2);

	/// Pop a toast under `surface` (the buffer-root host carrying the
	/// `DoubleBuffer`), replacing any toast already there so only one shows at a
	/// time. The toast spawns as a child of `surface`, so its overlay paints over
	/// the surface's other children, and despawns after [`DURATION`].
	pub fn show(
		commands: &mut Commands,
		surface: Entity,
		message: impl Into<String>,
	) {
		// clear the surface's existing toast first, so the next show replaces it
		// rather than stacking a second.
		commands.run_system_cached_with(despawn_toasts, surface);
		commands.spawn((
			ChildOf(surface),
			Toast,
			DespawnAfter::new(Self::DURATION),
			OnSpawn::insert_child(Value::str(message.into())),
		));
	}
}

/// Despawn every existing [`Toast`] child of `surface`, so a fresh toast is the
/// only one on that surface.
fn despawn_toasts(
	surface: In<Entity>,
	mut commands: Commands,
	children: Query<&Children>,
	toasts: Query<(), With<Toast>>,
) {
	for child in children
		.get(*surface)
		.into_iter()
		.flat_map(Children::iter)
		.filter(|child| toasts.contains(*child))
	{
		commands.entity(child).despawn();
	}
}

/// Registers the [`Toast`] component as a name-resolved tag and the
/// [`despawn_after`] lifecycle system.
#[derive(Default)]
pub struct ToastPlugin;

impl Plugin for ToastPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Toast>()
			.add_systems(Update, despawn_after);
	}
}

#[cfg(all(test, feature = "tui"))]
mod test {
	use super::*;
	use crate::style::material::MaterialStylePlugin;
	use bevy::math::UVec2;

	/// Render `content` into a `size` buffer with the charcell pipeline and the
	/// material rules active (so the `.toast` rule resolves and paints),
	/// returning the painted [`Buffer`] for cell inspection. Mirrors the
	/// `non_visual_tags_skipped_with_material` render-test setup.
	fn toast_buffer(size: UVec2, content: impl Bundle) -> Buffer {
		let mut world =
			(CharcellPlugin, MaterialStylePlugin::default()).into_world();
		let root = world
			.spawn((Buffer::new(size).into_double_buffer(), content))
			.id();
		world.run_schedule(PostParseTree);
		world
			.entity_mut(root)
			.take::<DoubleBuffer>()
			.unwrap()
			.into_buffer()
	}

	/// The (col, row) of the first cell of `needle` in a plain frame.
	fn cell_of(frame: &str, needle: char) -> (usize, usize) {
		for (row, line) in frame.lines().enumerate() {
			if let Some(col) = line.find(needle) {
				return (col, row);
			}
		}
		panic!("'{needle}' not found in frame:\n{frame}");
	}

	/// The render-path twin of [`Toast::show`]: a [`Toast`] marker carrying
	/// `message` as its child text, ready to spawn under a buffer root.
	fn toast(message: &str) -> impl Bundle {
		(Toast, OnSpawn::insert_child(Value::str(message)))
	}

	/// A toast paints its message text.
	#[beet_core::test]
	fn renders_message() {
		toast_buffer(UVec2::new(30, 6), toast("Saved"))
			.render_plain()
			.xpect_contains("Saved");
	}

	/// The toast box carries its overlay fill: every painted glyph cell sits on
	/// the `.toast` background (the inverse-surface token), not a bare cell.
	#[beet_core::test]
	fn box_has_overlay_background() {
		let buffer = toast_buffer(UVec2::new(30, 6), toast("Hi"));
		// the message cells all carry a background fill from the rule
		buffer
			.iter_cells()
			.filter(|(_, cell)| cell.symbol_str() != " ")
			.all(|(_, cell)| cell.style.background.is_some())
			.xpect_true();
	}

	/// The toast floats low and to the right, not in normal flow at the top:
	/// the page text keeps the first row while the fixed toast sits in the
	/// bottom-right region of the viewport.
	#[beet_core::test]
	fn floats_bottom_right_off_flow() {
		let buffer = toast_buffer(UVec2::new(20, 8), children![
			rsx! { <div>"page"</div> },
			toast("T")
		]);
		let frame = buffer.render_plain();
		let (page_col, page_row) = cell_of(&frame, 'p');
		let (toast_col, toast_row) = cell_of(&frame, 'T');
		// page stays at the top-left in flow
		page_row.xpect_eq(0);
		page_col.xpect_eq(0);
		// the toast floats below the page (bottom region) and to the right, ie out
		// of normal flow rather than stacked under it at the top-left
		(toast_row > page_row).xpect_true();
		(toast_row >= 4).xpect_true();
		(toast_col > 10).xpect_true();
	}

	/// Stacking: a toast overlapping the in-flow page wins the cells it covers,
	/// since its `Position::Fixed` lifts it into a high-z stacking context above
	/// the page. Proven by the painted cell carrying the toast's inverse-surface
	/// fill rather than the page's surface fill.
	#[beet_core::test]
	fn paints_on_top_of_page_content() {
		// the `.page` rule fills its box with the surface colour; the fixed toast
		// (inverse surface) lands in the bottom-right corner over it.
		let buffer = toast_buffer(UVec2::new(16, 6), children![
			rsx! { <div class="page">"page"</div> },
			toast("X")
		]);
		// the toast glyph paints over the page ...
		let (toast_pos, toast_cell) = buffer
			.iter_cells()
			.find(|(_, cell)| cell.symbol_str() == "X")
			.expect("toast glyph painted");
		// ... winning its cell with the toast's own background fill
		toast_cell.symbol_str().xpect_eq("X");
		let toast_bg = toast_cell.style.background.expect("toast fill");
		// the page's fill is a different colour, so the cell the toast covers is
		// painted by the toast, not the page beneath (stacking put it on top)
		let page_bg = buffer
			.iter_cells()
			.find(|(pos, cell)| {
				*pos != toast_pos && cell.style.background.is_some()
			})
			.and_then(|(_, cell)| cell.style.background);
		// a page background exists and differs from the toast's, confirming the
		// toast cell is genuinely the overlay's, not a bleed of the page colour
		(page_bg.is_some() && page_bg != Some(toast_bg)).xpect_true();
	}

	/// A charcell world with a manually driven [`Time`], for ticking
	/// [`despawn_after`].
	fn timed_world() -> World {
		let mut world = CharcellPlugin::world();
		world.init_resource::<Time>();
		world
	}

	/// Advance `world`'s [`Time`] then run [`despawn_after`].
	fn advance(world: &mut World, delta: Duration) {
		world.resource_mut::<Time>().advance_by(delta);
		world.run_system_cached(despawn_after).unwrap();
	}

	/// [`DespawnAfter`] keeps its entity until the duration elapses, then despawns
	/// it.
	#[beet_core::test]
	fn despawns_after_duration() {
		let mut world = timed_world();
		let entity =
			world.spawn(DespawnAfter::new(Duration::from_secs(2))).id();
		// before the duration: still alive
		advance(&mut world, Duration::from_millis(1900));
		world.get_entity(entity).is_ok().xpect_true();
		// past the duration: gone
		advance(&mut world, Duration::from_millis(200));
		world.get_entity(entity).is_ok().xpect_false();
	}

	/// How many `Toast` children `surface` currently has.
	fn toast_count(world: &mut World, surface: Entity) -> usize {
		world
			.query_filtered::<&ChildOf, With<Toast>>()
			.iter(world)
			.filter(|child_of| child_of.parent() == surface)
			.count()
	}

	/// Queue a `Toast::show` onto `world` and apply it.
	fn show(world: &mut World, surface: Entity, message: &'static str) {
		world.commands().queue(move |world: &mut World| {
			Toast::show(&mut world.commands(), surface, message)
		});
		world.flush();
	}

	/// `Toast::show` spawns exactly one toast under a surface, and a second
	/// `show` replaces it, keeping a single toast per surface.
	#[beet_core::test]
	fn show_keeps_one_toast_per_surface() {
		let mut world = timed_world();
		let surface = world.spawn_empty().id();

		show(&mut world, surface, "first");
		toast_count(&mut world, surface).xpect_eq(1);

		// a second show replaces the first: still exactly one
		show(&mut world, surface, "second");
		toast_count(&mut world, surface).xpect_eq(1);
	}

	/// A shown toast despawns once its [`Toast::DURATION`] elapses.
	#[beet_core::test]
	fn shown_toast_self_despawns() {
		let mut world = timed_world();
		let surface = world.spawn_empty().id();
		show(&mut world, surface, "bye");
		// alive just before the duration
		advance(&mut world, Toast::DURATION - Duration::from_millis(1));
		toast_count(&mut world, surface).xpect_eq(1);
		// despawned once it elapses
		advance(&mut world, Duration::from_millis(2));
		toast_count(&mut world, surface).xpect_eq(0);
	}
}
