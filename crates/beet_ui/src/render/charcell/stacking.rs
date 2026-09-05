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
//! A positioned element with `z-index: auto` forms NO stacking context, so it
//! cannot trap a descendant's `z-index`: its own in-flow content paints with it,
//! while its positioned descendants are hoisted into the containing context and
//! sorted there. That is what lets a `<select>`'s `z-index: 1000` panel paint
//! over the next positioned sibling rather than only within its own control.
//!
//! This is renderer-agnostic in spirit (no cell/buffer types): it reads only the
//! resolved position/z-index/overflow on each node, so a native renderer reuses
//! the same ordering. (`opacity`, `transform`, etc do not form contexts here yet,
//! noted as a deliberate omission; a scroll container does, which CSS reserves
//! for a positioned one, so a scroller bounds the hoist above.)

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

/// A node lifted out of tree order into its containing stacking context, sorted
/// by `z` (`auto` sorting with `0`).
struct ZItem {
	z: i32,
	kind: ZKind,
}

/// What a lifted node paints as.
enum ZKind {
	/// A stacking context, recursed into when emitted so its own descendants
	/// sort within it.
	Context(Entity),
	/// A positioned node with `z-index: auto`, forming no stacking context: the
	/// unit is the node followed by its in-flow descendants in tree order, its
	/// positioned descendants having been hoisted into the containing context.
	Pseudo(Vec<Entity>),
}

/// A node's role in its containing stacking context.
enum ZRole {
	/// Non-positioned: paints in tree order.
	InFlow,
	/// Forms a stacking context: a positioned node with an explicit `z-index`,
	/// or a scroll container.
	Context(i32),
	/// Positioned with `z-index: auto`: lifted like a context but forming none.
	Pseudo,
}

/// Emit the paint order of the stacking context rooted at `node`.
fn collect(
	node: Entity,
	query: &CharcellQuery,
	tree: &CharcellTree,
	managed: &HashSet<Entity>,
	order: &mut Vec<Entity>,
) {
	// the node forming this context paints its own box first.
	order.push(node);

	// gather this context's participating descendants: in-flow content in tree
	// order, and the lifted z-items (a nested context is gathered but not
	// descended into, being ordered as its own unit).
	let mut in_flow = Vec::new();
	let mut z_items = Vec::new();
	gather(node, query, tree, managed, &mut in_flow, &mut z_items);

	// stable, so ties keep tree order: the `auto`/`0` bucket sorts by nothing else
	z_items.sort_by_key(|item| item.z);
	// negative z paints below the in-flow content, everything else above it
	let split = z_items
		.iter()
		.position(|item| item.z >= 0)
		.unwrap_or(z_items.len());
	let above = z_items.split_off(split);

	for item in z_items {
		emit(item.kind, query, tree, managed, order);
	}
	order.extend(in_flow);
	for item in above {
		emit(item.kind, query, tree, managed, order);
	}
}

/// Emit one lifted item: a nested stacking context orders its own subtree, a
/// pseudo context is already flattened.
fn emit(
	kind: ZKind,
	query: &CharcellQuery,
	tree: &CharcellTree,
	managed: &HashSet<Entity>,
	order: &mut Vec<Entity>,
) {
	match kind {
		ZKind::Context(entity) => collect(entity, query, tree, managed, order),
		ZKind::Pseudo(unit) => order.extend(unit),
	}
}

/// Walk `node`'s subtree collecting in-flow descendants (tree order) and z-items
/// (lifted), stopping at each nested stacking context so it is ordered as its
/// own unit. A pseudo context is descended into instead: its in-flow content
/// becomes its own unit and its z-items are hoisted into this context.
fn gather(
	node: Entity,
	query: &CharcellQuery,
	tree: &CharcellTree,
	managed: &HashSet<Entity>,
	in_flow: &mut Vec<Entity>,
	z_items: &mut Vec<ZItem>,
) {
	for child in tree.children_of(node) {
		if managed.contains(&child) {
			continue;
		}
		let Ok(child_node) = query.unresolved_node(child) else {
			continue;
		};
		match z_role(&child_node) {
			ZRole::InFlow => {
				in_flow.push(child);
				gather(child, query, tree, managed, in_flow, z_items);
			}
			ZRole::Context(z) => z_items.push(ZItem {
				z,
				kind: ZKind::Context(child),
			}),
			ZRole::Pseudo => {
				// the slot is reserved before descending so the hoisted descendants
				// land after their own pseudo context, keeping tree order.
				let index = z_items.len();
				z_items.push(ZItem {
					z: 0,
					kind: ZKind::Pseudo(Vec::new()),
				});
				let mut unit = vec![child];
				gather(child, query, tree, managed, &mut unit, z_items);
				z_items[index].kind = ZKind::Pseudo(unit);
			}
		}
	}
}

/// Classify a node within its containing stacking context: a positioned element
/// or a scroll container is z-ordered, and every other node is in-flow. Only an
/// explicit `z-index` forms a stacking context, `auto` forming none (CSS), so a
/// positioned node cannot trap its descendants' `z-index`.
fn z_role(node: &CharcellNodeData) -> ZRole {
	let position = node.position_style();
	// a scroll container is a context whether or not it is positioned, the one
	// place this model is broader than CSS (see the module docs)
	if node.is_scroll_container() {
		return ZRole::Context(position.z_index.unwrap_or(0));
	}
	if !position.is_positioned() {
		return ZRole::InFlow;
	}
	match position.z_index {
		Some(z) => ZRole::Context(z),
		None => ZRole::Pseudo,
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
		buffer
			.get(UVec2::new(0, 0))
			.unwrap()
			.symbol_str()
			.xpect_eq("H");

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
		buffer
			.get(UVec2::new(0, 0))
			.unwrap()
			.symbol_str()
			.xpect_eq("L");
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

	/// A `z-index` panel inside a `z-index: auto` control is NOT trapped in it:
	/// `auto` forms no stacking context, so the panel sorts against the control's
	/// siblings and paints over the next one.
	///
	/// Regression: an open `<select>`'s dropdown was overdrawn by the following
	/// `<select>`, both being `position: relative` with no `z-index`.
	#[beet_core::test]
	fn auto_z_positioned_does_not_trap_descendant_z() {
		let buffer = stacked_buffer(
			UVec2::new(8, 4),
			vec![
				// the select control: positioned (the panel's containing block) with
				// no z-index of its own
				Rule::class("control")
					.with_value(common_props::PositionProp, Position::Relative),
				// the open panel, overhanging its control onto the next control's row
				Rule::class("panel")
					.with_value(common_props::PositionProp, Position::Absolute)
					.with_value(common_props::InsetTop, Length::Rem(1.))
					.with_value(common_props::InsetLeft, Length::Rem(0.))
					.with_value(common_props::ZIndexProp, 1000),
			],
			rsx! {
				<div>
					<div class="control">
						<div>"C"</div>
						<div class="panel">"P"</div>
					</div>
					<div class="control">
						<div>"N"</div>
					</div>
				</div>
			},
		);
		// row 1 belongs to the next control, overhung by the first one's panel
		buffer
			.get(UVec2::new(0, 1))
			.unwrap()
			.symbol_str()
			.xpect_eq("P");
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
		buffer
			.get(UVec2::new(0, 0))
			.unwrap()
			.symbol_str()
			.xpect_eq("O");
	}
}
