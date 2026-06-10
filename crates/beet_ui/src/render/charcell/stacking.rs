//! CSS stacking order: the back-to-front paint order driven by `z-index`,
//! positioning, and scroll containers, replacing the raw tree pre-order.
//!
//! The order is a flat `Vec<Entity>` the paint walk consumes. Stacking contexts
//! are formed by the root, positioned elements with a non-`auto` `z-index`, and
//! scroll containers. Within a context the paint order is (simplified CSS paint
//! order): the context's own box, then negative-`z` items, then non-positioned
//! in-flow descendants in tree order, then `auto`-`z` positioned items in tree
//! order, then positive-`z` items, each nested context recursed into.
//!
//! This is renderer-agnostic in spirit (no cell/buffer types): it reads only the
//! resolved position/z-index/overflow on each node, so a native renderer reuses
//! the same ordering. (`opacity`, `transform`, etc do not form contexts here yet,
//! noted as a deliberate omission.)

use super::*;
use beet_core::prelude::*;

/// The back-to-front paint order for a buffer tree rooted at `root`.
///
/// `managed` holds descendants of an inline formatting context, painted by their
/// owner, so they are skipped here exactly as the old pre-order skipped them.
pub(super) fn stacking_order(
	root: Entity,
	query: &CharcellQuery,
	tree: &CharcellTree,
	managed: &HashSet<Entity>,
) -> Vec<Entity> {
	let mut order = Vec::new();
	collect(root, query, tree, managed, &mut order);
	order
}

/// A descendant's role in its containing stacking context's paint order.
enum ZBucket {
	/// A non-positioned, non-stacking-context node: paints in tree order.
	InFlow,
	/// A positioned node or stacking context, lifted and z-ordered.
	Z(i32),
	/// A positioned node or stacking context with `z-index: auto` (0-ish, but in
	/// the auto bucket which paints above in-flow content regardless of value).
	Auto,
}

/// Emit `node` then its participating descendants in paint order, recursing into
/// nested z-items (positioned elements / stacking contexts).
fn collect(
	node: Entity,
	query: &CharcellQuery,
	tree: &CharcellTree,
	managed: &HashSet<Entity>,
	order: &mut Vec<Entity>,
) {
	// the node forming this (sub)order paints its own box first.
	order.push(node);

	// gather participating descendants: walk the subtree but do not cross into a
	// z-item (it is ordered as its own unit and recursed into separately).
	let mut in_flow = Vec::new();
	let mut z_items: Vec<(Entity, ZBucket)> = Vec::new();
	gather(node, query, tree, managed, &mut in_flow, &mut z_items);

	// partition the z-items and sort the explicit ones by z (negatives below the
	// in-flow content, positives above; ties keep tree order, which `sort_by` is
	// stable for).
	let mut negative: Vec<(Entity, i32)> = Vec::new();
	let mut auto: Vec<Entity> = Vec::new();
	let mut positive: Vec<(Entity, i32)> = Vec::new();
	for (entity, bucket) in z_items {
		match bucket {
			ZBucket::Z(z) if z < 0 => negative.push((entity, z)),
			ZBucket::Z(z) if z > 0 => positive.push((entity, z)),
			// z == 0 paints with the auto bucket (above in-flow)
			ZBucket::Z(_) | ZBucket::Auto => auto.push(entity),
			ZBucket::InFlow => {}
		}
	}
	negative.sort_by_key(|(_, z)| *z);
	positive.sort_by_key(|(_, z)| *z);

	// assemble: negative z, then in-flow tree order, then auto/0 z, then positive z
	for (entity, _) in negative {
		collect(entity, query, tree, managed, order);
	}
	order.extend(in_flow);
	for entity in auto {
		collect(entity, query, tree, managed, order);
	}
	for (entity, _) in positive {
		collect(entity, query, tree, managed, order);
	}
}

/// Walk `node`'s subtree collecting in-flow descendants (tree order) and z-items
/// (lifted), stopping at each z-item so it is ordered as its own unit.
fn gather(
	node: Entity,
	query: &CharcellQuery,
	tree: &CharcellTree,
	managed: &HashSet<Entity>,
	in_flow: &mut Vec<Entity>,
	z_items: &mut Vec<(Entity, ZBucket)>,
) {
	for child in tree.children_of(node) {
		if managed.contains(&child) {
			continue;
		}
		let Ok(child_node) = query.unresolved_node(child) else {
			continue;
		};
		match z_bucket(&child_node) {
			ZBucket::InFlow => {
				in_flow.push(child);
				gather(child, query, tree, managed, in_flow, z_items);
			}
			bucket => z_items.push((child, bucket)),
		}
	}
}

/// Classify a node within its containing stacking context: a positioned element
/// or a scroll container is z-ordered (by its `z-index`, `auto` if unset), every
/// other node is in-flow.
fn z_bucket(node: &CharcellNodeData) -> ZBucket {
	let position = node.position_style();
	let lifted = position.is_positioned() || node.is_scroll_container();
	if !lifted {
		return ZBucket::InFlow;
	}
	match position.z_index {
		Some(z) => ZBucket::Z(z),
		None => ZBucket::Auto,
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::*;
	use beet_core::prelude::*;
	use bevy::math::UVec2;

	/// Render `content` into a `size` buffer with `rules`, returning the [`Buffer`]
	/// so a specific cell can be inspected (stacking decides which glyph wins).
	fn stacked_buffer(
		size: UVec2,
		rules: Vec<Rule>,
		content: impl Bundle,
	) -> Buffer {
		let mut world = CharcellPlugin::world();
		world.get_resource_or_init::<RuleSet>().extend_rules(rules);
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

	/// A rule placing a class absolutely at the top-left with a given z-index.
	fn abs_at(class: &str, z: i32) -> Rule {
		Rule::class(class)
			.with_value(common_props::PositionProp, Position::Absolute)
			.with_value(common_props::InsetTop, Length::Rem(0.))
			.with_value(common_props::InsetLeft, Length::Rem(0.))
			.with_value(common_props::ZIndexProp, z)
	}

	/// Two overlapping absolute boxes: the higher `z-index` wins the shared cell.
	#[beet_core::test]
	fn higher_z_index_paints_on_top() {
		// "low" before "high" in tree order, but high z-index wins
		let buffer = stacked_buffer(
			UVec2::new(8, 4),
			vec![abs_at("low", 1), abs_at("high", 2)],
			rsx! {
				<div>
					<div class="high">"H"</div>
					<div class="low">"L"</div>
				</div>
			},
		);
		// at the shared top-left cell, the higher z-index glyph (H) is on top
		buffer.get(UVec2::new(0, 0)).unwrap().symbol_str().xpect_eq("H");

		// reversing z-index reverses the winner, proving it is z not tree order
		let buffer = stacked_buffer(
			UVec2::new(8, 4),
			vec![abs_at("low", 5), abs_at("high", 1)],
			rsx! {
				<div>
					<div class="high">"H"</div>
					<div class="low">"L"</div>
				</div>
			},
		);
		buffer.get(UVec2::new(0, 0)).unwrap().symbol_str().xpect_eq("L");
	}

	/// A negative `z-index` child paints behind its parent's background.
	#[beet_core::test]
	fn negative_z_index_paints_behind_parent_background() {
		let bg = Color::srgb(0.2, 0.5, 0.9);
		let buffer = stacked_buffer(
			UVec2::new(8, 4),
			vec![
				Rule::class("parent")
					.with_value(common_props::PositionProp, Position::Relative)
					.with_value(common_props::BackgroundColor, bg),
				Rule::class("behind")
					.with_value(common_props::PositionProp, Position::Absolute)
					.with_value(common_props::InsetTop, Length::Rem(0.))
					.with_value(common_props::InsetLeft, Length::Rem(0.))
					.with_value(common_props::ZIndexProp, -1),
			],
			rsx! {
				<div class="parent">
					<div class="behind">"B"</div>
					"P"
				</div>
			},
		);
		// the parent background covers the negative-z child at the top-left cell
		let cell = buffer.get(UVec2::new(0, 0)).unwrap();
		cell.style.background.xpect_eq(Some(bg));
		// the behind glyph does not win the cell (parent paints over it)
		(cell.symbol_str() != "B").xpect_true();
	}

	/// A positioned child with `z-index: auto` paints above an earlier in-flow
	/// sibling (CSS lifts positioned content above non-positioned).
	#[beet_core::test]
	fn auto_z_positioned_paints_above_in_flow_sibling() {
		let buffer = stacked_buffer(
			UVec2::new(8, 4),
			vec![
				// a relative box pulled back over the in-flow sibling via a negative
				// top inset, with auto z-index (no explicit z).
				Rule::class("over")
					.with_value(common_props::PositionProp, Position::Relative)
					.with_value(common_props::InsetTop, Length::Rem(-1.)),
			],
			rsx! {
				<div>
					<div>"I"</div>
					<div class="over">"O"</div>
				</div>
			},
		);
		// "O" is relatively shifted up onto "I"'s row and, being positioned, paints
		// above it
		buffer.get(UVec2::new(0, 0)).unwrap().symbol_str().xpect_eq("O");
	}
}
