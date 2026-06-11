use super::*;

use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
use bevy::math::IVec2;
use bevy::math::UVec2;

/// The paint context handed to each node: the clip it draws within and the
/// accumulated scroll translation applied to its rects. Reused by the hit-test
/// so a cursor maps through the same transform the paint applied.
#[derive(Clone, Copy)]
pub(super) struct PaintContext {
	pub clip: Clip,
	/// Accumulated translation from ancestor scroll containers, applied to this
	/// node's own box (its descendants additionally shift by this node's offset).
	pub offset: IVec2,
}

impl Default for PaintContext {
	fn default() -> Self {
		Self {
			clip: Clip::NONE,
			offset: IVec2::ZERO,
		}
	}
}

/// ECS system: paint all nodes in each [`DoubleBuffer`] tree.
///
/// Traverses each tree pre-order. Each node fills its background (if set),
/// draws its border, then paints text. Pre-order ensures parents fill first so
/// children naturally overlay their margin area without any parent lookup.
///
/// Nodes inside an [inline formatting context](inline) are painted by their
/// container, so the whole subtree below an IFC owner is skipped here.
///
/// Each node carries a [`PaintContext`]: a clip (an overflow container narrows
/// its descendants to its scrollport) and a scroll translation (a scroll
/// container shifts its descendants by `-offset`). Both flow top-down.
pub fn paint_nodes<B: Component<Mutability = Mutable> + AsBuffer>(
	mut roots: Populated<(Entity, &mut B)>,
	charcell: CharcellQuery,
	tree: CharcellTree,
) -> Result {
	for (root, mut buffer) in roots.iter_mut() {
		let viewport_size = buffer.size();
		let ordered = tree.pre_order(root);

		// descendants of an IFC owner are painted by the owner, not themselves
		let mut managed = HashSet::<Entity>::default();
		for &entity in &ordered {
			if managed.contains(&entity) {
				continue;
			}
			let Ok(node) = charcell.unresolved_node(entity) else {
				continue;
			};
			if establishes_inline_flow(&node, &charcell) {
				managed.extend(tree.descendants(entity));
			}
		}

		// resolve the clip + scroll translation for every node (parent-aware).
		let contexts =
			resolve_contexts(root, &ordered, &charcell, &tree, viewport_size);

		// paint in CSS stacking order (z-index + positioning + scroll containers),
		// not raw tree order, so positioned/overlapping content layers correctly.
		let painted = stacking_order(root, &charcell, &tree, &managed);

		// full reset may become a problematic pattern if we want to do
		// partial paints
		buffer.reset();

		for &entity in &painted {
			if managed.contains(&entity) {
				continue;
			}
			let Ok(node) = charcell.unresolved_node(entity) else {
				continue;
			};
			let cx = contexts.get(&entity).copied().unwrap_or_default();
			paint_node(&node, &charcell, viewport_size, &mut *buffer, cx)?;
		}
	}
	Ok(())
}

/// The [`PaintContext`] each node paints with, computed top-down.
///
/// The root paints within the full viewport ([`Clip::NONE`]) at zero offset. A
/// node whose [`LayoutStyle::clips`] narrows the clip handed to its descendants:
/// to its scrollport for a scroll container (so content does not paint under the
/// bar), or to its padding box ([`BoxModel::inner_rect`]) for plain
/// `overflow:hidden`. A scroll container additionally shifts its descendants by
/// `-offset`. (CSS promotes a lone `visible` axis to `auto` when the other
/// clips, so clipping either axis clips both.)
pub(super) fn resolve_contexts(
	root: Entity,
	ordered: &[Entity],
	query: &CharcellQuery,
	tree: &CharcellTree,
	viewport: UVec2,
) -> HashMap<Entity, PaintContext> {
	let mut contexts = HashMap::<Entity, PaintContext>::default();
	contexts.insert(root, PaintContext::default());
	// pre-order visits a parent before its children, so each node's context is
	// set before we compute and propagate its children's.
	for &entity in ordered {
		let cx = contexts.get(&entity).copied().unwrap_or_default();
		let Ok(node) = query.unresolved_node(entity) else {
			continue;
		};
		// the clip and translation apply in the node's own (already-translated)
		// space, so derive the child context from this node's rect shifted by `cx`.
		let translated_rect = translate_rect(node.layout_rect(), cx.offset);
		let child_clip = if node.is_scroll_container() {
			// clip content to the scrollport so it never paints under the bar
			let scrollport =
				translate_rect(scrollport_rect(&node, query, viewport), cx.offset);
			Clip(cx.clip.intersect(scrollport))
		} else if node.layout_style().clips() {
			let padding_box =
				BoxModel::from_node(&node, viewport).inner_rect(translated_rect);
			Clip(cx.clip.intersect(padding_box))
		} else {
			cx.clip
		};
		// descendants additionally scroll by this node's own offset
		let child_offset = cx.offset - node.scroll_offset();
		let child_cx = PaintContext {
			clip: child_clip,
			offset: child_offset,
		};
		for child in tree.children_of(entity) {
			contexts.insert(child, child_cx);
		}
	}
	contexts
}

fn paint_node(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	viewport: UVec2,
	buffer: &mut impl AsBuffer,
	cx: PaintContext,
) -> Result {
	let clip = cx.clip;
	let box_model = BoxModel::from_node(node, viewport);
	// the node's box is painted at its laid-out rect shifted by the accumulated
	// ancestor scroll translation.
	let layout_rect = translate_rect(node.layout_rect(), cx.offset);
	let border_rect = box_model.border_rect(layout_rect);
	let inner_rect = box_model.inner_rect(layout_rect);
	let content_rect = box_model.content_rect(layout_rect);

	// 1. Fill inner rect with background only when the node has a background color.
	//    Skipping transparent nodes keeps "empty" rows as Cell::BLANK so
	//    trim_lines can strip trailing blank rows correctly.
	if node.visual_style().background.is_some()
		&& inner_rect.width() > 0
		&& inner_rect.height() > 0
	{
		buffer.fill_rect(
			inner_rect,
			Cell::new(" ", node.visual_style().clone(), node.entity),
			clip,
		);
	}

	// 2. Draw any present border sides
	if box_model.border.any() {
		draw_border(&mut *buffer, border_rect, box_model.border, node, clip);
	}

	// 3. Paint content: flow inline descendants if this owns an inline
	//    formatting context, otherwise paint this node's own text (a no-op
	//    when it has no value). A scroll container's own inline/text content
	//    scrolls with it, so shift the content rect by its scroll offset (child
	//    entities are already shifted via the descendant context).
	let content_rect = translate_rect(content_rect, -node.scroll_offset());
	if establishes_inline_flow(node, query) {
		paint_inline_flow(node, query, content_rect, &mut *buffer, clip);
	} else {
		paint_text(node, content_rect, &mut *buffer, clip)?;
	}

	// 4. A scroll container paints its track/thumb in the reserved gutter, fixed
	//    to the container (not scrolled with the content). The shared geometry
	//    (read here and by the mouse hit-test) is derived from the screen offset.
	if node.is_scroll_container() {
		paint_scrollbar(&mut *buffer, node, query, viewport, cx.offset, cx.clip);
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use crate::style::*;
	use beet_core::prelude::*;
	use bevy::math::UVec2;

	/// Render a `<div class="container">` wrapping a 5-line `<pre>` into a sized
	/// buffer, with a rule set giving `.container` a 2-row height and the given
	/// overflow. Overflow is driven through the rule set (not a hand-attached
	/// `LayoutStyle`) because that is how a page authors it, and resolution would
	/// otherwise overwrite a hand-attached style.
	fn overflow_frame(overflow: Overflow, extra: Vec<Rule>) -> String {
		let mut world = CharcellPlugin::world();
		let mut rules = vec![
			Rule::class("container")
				.with_value(common_props::Height, Length::Rem(2.))
				.with_value(common_props::OverflowXProp, overflow)
				.with_value(common_props::OverflowYProp, overflow),
		];
		rules.extend(extra);
		world.get_resource_or_init::<RuleSet>().extend_rules(rules);
		// the container is wrapped so it is not the root (the root always fills the
		// viewport, ignoring its own height); as a child its height: 2 constrains
		// its rect, giving a real 2-row scrollport to clip against.
		let entity = world
			.spawn((
				Buffer::new(UVec2::new(20, 8)).into_double_buffer(),
				rsx! {
					<div>
						<div class="container">
							<pre>"l1\nl2\nl3\nl4\nl5"</pre>
						</div>
					</div>
				},
			))
			.id();
		world.run_schedule(PostParseTree);
		world
			.entity_mut(entity)
			.take::<DoubleBuffer>()
			.unwrap()
			.into_buffer()
			.render_plain()
			.trim_lines()
	}

	/// `overflow: hidden` clips descendants to the container's padding box, so a
	/// 5-line paragraph in a 2-row container shows only the first 2 lines.
	#[beet_core::test]
	fn overflow_hidden_clips_to_padding_box() {
		let out = overflow_frame(Overflow::Hidden, vec![]);
		out.as_str().xpect_contains("l1").xpect_contains("l2");
		out.as_str().xnot().xpect_contains("l3");
		out.lines().count().xpect_eq(2);
	}

	/// `overflow: visible` (default) keeps today's behavior: nothing is clipped,
	/// all 5 lines paint past the 2-row box.
	#[beet_core::test]
	fn overflow_visible_does_not_clip() {
		let out = overflow_frame(Overflow::Visible, vec![]);
		out.as_str()
			.xpect_contains("l1")
			.xpect_contains("l3")
			.xpect_contains("l5");
		out.lines().count().xpect_eq(5);
	}

	/// A bordered overflow container clips at the padding box, so the border
	/// itself (outside the clip) stays visible while inner content is cut.
	#[beet_core::test]
	fn overflow_hidden_keeps_border() {
		let out = overflow_frame(
			Overflow::Hidden,
			vec![
				Rule::class("container")
					.with_value(common_props::OutlineWidth, Length::Px(1.)),
			],
		);
		// the top border (and its corners) survive the clip at the padding box
		out.as_str().xpect_contains("┌").xpect_contains("┐");
		// content past the clipped 2-row box is gone
		out.xnot().xpect_contains("l5");
	}
}
