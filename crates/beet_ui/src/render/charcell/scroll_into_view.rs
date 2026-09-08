//! Keyboard focus scroll-into-view: when [`Focus`] lands on an element, its
//! nearest scrollable ancestor scrolls the minimum needed to reveal it.
//!
//! The browser behaviour Tab traversal depends on. Without it a page taller than
//! the terminal leaves the typist editing a control below the fold, which is the
//! single thing that makes keyboard use of a long form feel broken.

use super::*;
use crate::input::ScrollPosition;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::math::IVec2;

/// Cells of context kept between the focused element and the scrollport edge:
/// flush against the edge reads as clipped, one cell of margin reads as a
/// deliberate stop.
const FOCUS_SCROLL_MARGIN: i32 = 1;

/// ECS system: scroll a newly focused element into its scrollport.
///
/// Gated on `Added<Focus>`, so a settled focus never re-scrolls: someone who has
/// deliberately scrolled away from their focused field keeps their position, and
/// only a focus *move* pulls the view back.
///
/// Runs after [`layout_nodes`](super::layout_nodes), whose fresh
/// [`LayoutRect`](super::LayoutRect)s it reads, and before
/// [`clamp_scroll_positions`](super::clamp_scroll_positions), so the write is
/// settled and painted in the same frame the focus moved.
pub(crate) fn scroll_focus_into_view<B: Component + AsBuffer>(
	focused: Populated<Entity, Added<Focus>>,
	surfaces: SurfaceQuery,
	tree: CharcellTree,
	roots: Query<&B>,
	// `CharcellQuery` (p0) reads `ScrollPosition`, so it can't coexist with the
	// `&mut ScrollPosition` writer (p1) outside a `ParamSet`.
	mut params: ParamSet<(CharcellQuery, Query<&mut ScrollPosition>)>,
) {
	// resolve each newly focused element against its own surface's geometry,
	// snapshotting the writes (the read and write borrows can't overlap).
	let mut targets = Vec::new();
	for entity in focused.iter() {
		let Some(viewport) = surfaces
			.surface_of(entity)
			.and_then(|surface| roots.get(surface).ok())
			.map(|buffer| buffer.size())
		else {
			continue;
		};
		let charcell = params.p0();
		// the nearest scrollable ancestor, excluding the element itself: scrolling
		// a focused box's own content cannot bring that box into view.
		let container =
			tree.visual_ancestors(entity).skip(1).find(|ancestor| {
				scrollable_extent(*ancestor, &charcell, viewport)
					.is_some_and(|max| max != IVec2::ZERO)
			});
		let (Some(container), Ok(focus_node)) =
			(container, charcell.unresolved_node(entity))
		else {
			continue;
		};
		let Ok(node) = charcell.unresolved_node(container) else {
			continue;
		};
		// the scrollport is the window the container currently shows, in the same
		// unscrolled layout space the focused rect is in.
		let port = scrollport_rect(&node, &charcell, viewport);
		let rect = focus_node.layout_rect();
		let offset = node.scroll_offset();
		let delta = IVec2::new(
			axis_delta(
				rect.min.x, rect.max.x, port.min.x, port.max.x, offset.x,
			),
			axis_delta(
				rect.min.y, rect.max.y, port.min.y, port.max.y, offset.y,
			),
		);
		if delta != IVec2::ZERO {
			targets.push((
				container,
				delta,
				scroll_state(&node, &charcell, viewport),
			));
		}
	}
	for (container, delta, state) in targets {
		if let Ok(mut scroll) = params.p1().get_mut(container) {
			scroll.scroll_by(delta, &state);
		}
	}
}

/// The minimum scroll delta on one axis that reveals `[min, max)`, plus a
/// [`FOCUS_SCROLL_MARGIN`] of context; zero when it is already visible.
///
/// Layout is unscrolled and paint translates descendants by `-offset`, so the
/// window a container currently shows is its scrollport shifted by that offset.
/// An element before the window aligns its near edge, one past it aligns its far
/// edge, and one larger than the window aligns its near edge (the first branch
/// wins). Never centred: a minimal scroll keeps the surrounding form put.
fn axis_delta(
	min: i32,
	max: i32,
	port_min: i32,
	port_max: i32,
	offset: i32,
) -> i32 {
	let (visible_min, visible_max) = (port_min + offset, port_max + offset);
	if min - FOCUS_SCROLL_MARGIN < visible_min {
		min - FOCUS_SCROLL_MARGIN - visible_min
	} else if max + FOCUS_SCROLL_MARGIN > visible_max {
		max + FOCUS_SCROLL_MARGIN - visible_max
	} else {
		0
	}
}

#[cfg(all(test, feature = "tui"))]
mod test {
	use super::*;
	use crate::render::charcell::test_host::TestHost;
	use crate::style::*;
	use bevy::math::UVec2;

	/// A viewport-height scrollport holding `count` one-row fields, each
	/// painting its own `field{i}` label, so the column overflows once `count`
	/// exceeds the eight rows on screen.
	fn field_column(count: usize) -> TestHost {
		let mut host = TestHost::sized(UVec2::new(20, 8));
		host.app
			.world_mut()
			.get_resource_or_init::<RuleSet>()
			.extend_rules(vec![
				Rule::class("form")
					.with_value(common_props::OverflowYProp, Overflow::Scroll),
			]);
		host.spawn_content(rsx! {
			<div class="form">
				{(0..count)
					.map(|i| rsx! { <input {Value::str(format!("field{i}"))}/> })
					.collect::<Vec<_>>()}
			</div>
		});
		host.step();
		host
	}

	/// The vertical offset of the (single) scroll container.
	fn offset_y(host: &mut TestHost) -> i32 {
		host.app
			.world_mut()
			.query::<&ScrollPosition>()
			.iter(host.app.world())
			.map(|scroll| scroll.offset.y)
			.max()
			.unwrap()
	}

	/// Tab scrolls the focused control into view: a column of fields taller than
	/// its scrollport keeps the focused one on screen, instead of leaving the
	/// typist editing a field below the fold.
	#[beet_core::test]
	fn tab_scrolls_focus_into_view() {
		let mut host = field_column(20);
		// premise: the last field starts below the fold.
		host.frame_plain().xnot().xpect_contains("field19");

		// Tab to it; each press reveals the newly focused field.
		for _ in 0..20 {
			host.send_input(b"\t");
			host.step();
		}
		host.frame_plain().xpect_contains("field19");
		// the view moved the minimum needed, so the field sits at the bottom of
		// the scrollport rather than centred or pinned to the top.
		host.frame_plain().xnot().xpect_contains("field0");
	}

	/// A focus move within the visible window scrolls nothing: the minimum is
	/// zero, so stepping back to a neighbour already on screen leaves the form
	/// exactly where it is rather than re-aligning or centring it.
	#[beet_core::test]
	fn visible_focus_does_not_scroll() {
		let mut host = field_column(20);
		// tab into the middle of the column, leaving the container scrolled well
		// short of its end (where a clamp would mask an over-scroll).
		for _ in 0..11 {
			host.send_input(b"\t");
			host.step();
		}
		let scrolled = offset_y(&mut host);
		(scrolled > 0).xpect_true();

		// Shift+Tab to the previous field, which is already on screen.
		host.send_input(b"\x1b[Z");
		host.step();
		offset_y(&mut host).xpect_eq(scrolled);
	}
}
