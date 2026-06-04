//! Measure phase: compute [`IntrinsicSize`] bottom-up (post-order).
//!
//! Each node answers: *"If I had infinite space, how big would I want to be?"*
use super::*;
use crate::style::Display;
use beet_core::prelude::*;
use bevy::math::UVec2;

/// The node's preferred size before parent constraints apply.
///
/// Written by the measure phase, read by the layout phase.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct IntrinsicSize(pub UVec2);

/// ECS system: compute [`IntrinsicSize`] for all nodes bottom-up.
pub fn measure_nodes<B: Component + AsBuffer>(
	mut params: ParamSet<(CharcellQuery, Query<&mut IntrinsicSize>)>,
	tree: CharcellTree,
	roots: Populated<(Entity, &B)>,
) -> Result {
	for (root, buffer) in roots {
		let viewport_size = buffer.size();
		let ordered = tree.post_order(root);
		let mut sizes = HashMap::<Entity, UVec2>::new();

		// Read phase: use CharcellQuery to measure each node bottom-up
		{
			let charcell = params.p0();
			for &entity in &ordered {
				let Ok(node) = charcell.unresolved_node(entity) else {
					continue;
				};
				let size =
					measure_node(&node, &charcell, viewport_size, &sizes)?;
				sizes.insert(entity, size);
			}
		}

		// Write phase: flush computed sizes to ECS components
		for (entity, size) in sizes {
			if let Ok(mut intrinsic) = params.p1().get_mut(entity) {
				intrinsic.set_if_neq(IntrinsicSize(size));
			}
		}
	}
	Ok(())
}

/// Compute a single node's intrinsic size.
///
/// Uses viewport dimensions as the unconstrained available space.
pub(super) fn measure_node(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	viewport: UVec2,
	sizes: &HashMap<Entity, UVec2>,
) -> Result<UVec2> {
	let box_model = BoxModel::from_node(node, viewport);
	let overhead = box_model.overhead();
	let content_available = UVec2::new(
		viewport.x.saturating_sub(overhead.x),
		viewport.y.saturating_sub(overhead.y),
	);
	let content_size = match node.layout_style().display {
		Display::Flex => {
			measure_flex(node, query, sizes, content_available, viewport)?
		}
		// text leaf (eg a paragraph's text node)
		_ if node.value().is_some() => measure_text(node, content_available.x),
		// container of inline content: flow descendants as wrapped text
		_ if establishes_inline_flow(node, query) => {
			measure_inline_flow(node, query, content_available.x)
		}
		// block leaf whose content is generated (eg the `<hr>` rule, `<img>` alt)
		_ if let Some(marker) =
			node.marker().filter(|_| !node.has_child_nodes(query)) =>
		{
			measure_str(marker, content_available.x)
		}
		// block container: stack children vertically
		_ => measure_block(node, query, content_available, sizes),
	};
	(content_size + overhead).xok()
}

/// Measure block container: stack children top-to-bottom.
///
/// Height is the sum of child heights (each at least one row, mirroring
/// [`block_layout_rects`]), width the widest child clamped to the content box.
///
/// Children must already be measured (post-order traversal ensures this).
fn measure_block(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	available: UVec2,
	sizes: &HashMap<Entity, UVec2>,
) -> UVec2 {
	// a list item holding a nested list reserves a left gutter for its marker
	let gutter = marker_gutter(node, query);
	let mut max_w = 0u32;
	let mut total_h = 0u32;
	for child in node.child_nodes(query) {
		let size = sizes
			.get(&child.entity)
			.copied()
			.unwrap_or_else(|| child.intrinsic_size());
		max_w = max_w.max(size.x);
		total_h += size.y.max(1);
	}
	UVec2::new((gutter + max_w).min(available.x), total_h)
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::math::UVec2;

	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_plain_sized(UVec2::new(20, 10), bundle)
			.trim_lines()
			.lines()
			.map(|line| line.trim_end())
			.collect::<Vec<_>>()
			.join("\n")
	}

	#[beet_core::test]
	fn block_stacks_children_vertically() {
		// a heading and a paragraph are both block: each on its own line(s),
		// stacked top-to-bottom and separated by the block gap (the heading's
		// bottom margin), with trailing blank rows trimmed away.
		render(rsx_direct! {
			<div>
				<h1>"Title"</h1>
				<p>"Body"</p>
			</div>
		})
		.xpect_eq("Title\n\nBody");
	}

	#[beet_core::test]
	fn block_child_text_wraps_and_reserves_height() {
		// a paragraph wider than the viewport wraps to two rows; the block
		// container must reserve both (plus the block gap) so the following
		// block is not clipped: two wrapped rows, a blank gap, then the heading.
		render(rsx_direct! {
			<div>
				<p>"one two three four five six"</p>
				<h2>"End"</h2>
			</div>
		})
		.lines()
		.count()
		.xpect_eq(4);
	}
}
