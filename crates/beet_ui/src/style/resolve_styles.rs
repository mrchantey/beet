use crate::prelude::*;
use crate::style::FlexBox;
use crate::style::LayoutStyle;
use crate::style::VisualStyle;
use crate::style::common_props::BackgroundColor;
use crate::style::common_props::*;
use crate::style::*;
use beet_core::prelude::*;

pub fn resolve_styles(
	mut commands: Commands,
	ruleset_query: RuleSetQuery,
	// classes drive the cascade (eg `.dark-scheme`), so a runtime class change
	// must re-resolve even when the [`Element`] itself is untouched. Interactive
	// state (eg `:focus`) does too, so a focus change re-resolves the cascade.
	query: Query<
		Entity,
		Or<(Changed<Element>, Changed<Classes>, Changed<ElementStateMap>)>,
	>,
	// a resized surface re-evaluates width-gated media rules for its whole
	// tree. `MediaViewport` is `set_if_neq`-maintained (`sync_media_viewport`),
	// so this fires on a real resize or first sight, not on paint's per-frame
	// buffer writes.
	resized: Query<Entity, Changed<MediaViewport>>,
	ancestors: Query<&ChildOf>,
	children: Query<&Children>,
	// content transcluded by reference has no `ChildOf` edge to the layout, so the
	// traversal follows holders to re-resolve referenced content under the layout's
	// cascade (eg the color scheme), even when the content itself is unchanged.
	render_refs: Query<&Portal>,
	// the box model (margin/border/padding/background) is element-level; text and
	// fragment nodes must not resolve their nearest ancestor's box and re-paint it.
	elements: Query<(), With<Element>>,
	mut styles: Query<(
		Option<&mut VisualStyle>,
		Option<&mut LayoutStyle>,
		Option<&mut BoxStyle>,
		Option<&mut PositionStyle>,
		Option<&mut ScrollbarStyle>,
	)>,
	mut transitions: Query<Option<&mut TransitionStyle>>,
) -> Result {
	// TODO fine-grained listeners
	// reparenting etc. only update whats needed
	let mut roots = query
		.iter()
		.map(|entity| ancestors.root_ancestor(entity))
		.collect::<HashSet<_>>();
	// a resized surface seeds the traversal directly (its whole tree, portals
	// included, re-resolves below) — only worth it when a width-gated rule
	// exists to change the outcome.
	if !resized.is_empty() && ruleset_query.has_width_media() {
		roots.extend(resized.iter());
	}

	// within-pass cascade memo, keyed by `(entity, token)`. Resolving the page
	// touches ~30 properties per entity and inherited tokens re-walk ancestors;
	// without this the ancestor re-walk is O(n²). Fresh each run so no stale
	// value leaks across frames.
	let mut memo = CascadeMemo::default();

	// inheritance cache friendly parallelism, top down queue,
	// as described in stylo https://youtu.be/Y6SSTRr2mFU?t=310
	let mut queue = roots.into_iter().collect::<Vec<_>>();
	while !queue.is_empty() {
		for entity in queue.drain(..).collect::<Vec<_>>() {
			// resolve visual style
			let visual = resolve_visual(&ruleset_query, entity, &mut memo)?;
			if let Some(mut style) = styles.get_mut(entity)?.0 {
				style.set_if_neq(visual);
			} else {
				commands.entity(entity).insert(visual);
			}

			// resolve layout style
			let layout = resolve_layout(&ruleset_query, entity, &mut memo)?;
			if let Some(mut style) = styles.get_mut(entity)?.1 {
				style.set_if_neq(layout);
			} else {
				commands.entity(entity).insert(layout);
			}

			// resolve box style — only for elements, so a text/fragment child does
			// not inherit and re-paint its ancestor element's border/background.
			let box_s = if elements.contains(entity) {
				resolve_box(&ruleset_query, entity, &mut memo)?
			} else {
				BoxStyle::default()
			};
			if let Some(mut style) = styles.get_mut(entity)?.2 {
				style.set_if_neq(box_s);
			} else {
				commands.entity(entity).insert(box_s);
			}

			// resolve positioning — element-level like the box model, so a text
			// node never carries its ancestor's position.
			let position = if elements.contains(entity) {
				resolve_position(&ruleset_query, entity, &mut memo)?
			} else {
				PositionStyle::default()
			};
			if let Some(mut style) = styles.get_mut(entity)?.3 {
				style.set_if_neq(position);
			} else {
				commands.entity(entity).insert(position);
			}

			// resolve scrollbar styling (element-level), read by the charcell
			// scrollbar paint on scroll containers.
			let scrollbar = if elements.contains(entity) {
				resolve_scrollbar(&ruleset_query, entity, &mut memo)?
			} else {
				ScrollbarStyle::default()
			};
			if let Some(mut style) = styles.get_mut(entity)?.4 {
				style.set_if_neq(scrollbar);
			} else {
				commands.entity(entity).insert(scrollbar);
			}

			// resolve transition settings for every node, not just elements: a
			// text leaf resolves through its nearest element's rules, so it
			// carries the same transition and its inherited colours ease in
			// step with the element (CSS gets this by re-deriving inherited
			// used values; beet resolves each node's own). Attached only while
			// the duration is nonzero, so the animation system iterates
			// transitioned nodes alone.
			let transition =
				resolve_transition(&ruleset_query, entity, &mut memo)?;
			match transitions.get_mut(entity)? {
				Some(mut style) if transition.duration > Duration::ZERO => {
					style.set_if_neq(transition);
				}
				Some(_) => {
					commands
						.entity(entity)
						.remove::<(TransitionStyle, VisualTransition)>();
				}
				None if transition.duration > Duration::ZERO => {
					commands.entity(entity).insert(transition);
				}
				None => {}
			}

			if let Some(children_list) = children.get(entity).ok() {
				queue.extend(children_list.into_iter().cloned());
			}
			// follow a `Portal` holder into the content it renders in place, so
			// transcluded content re-resolves under this (layout) cascade. An
			// unresolved holder (no `Portal` yet) has no content to cascade into.
			if let Ok(render_ref) = render_refs.get(entity) {
				queue.push(render_ref.target());
			}
		}
	}
	Ok(())
}

fn resolve_visual(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<VisualStyle> {
	let foreground = query.resolve(entity, ForegroundColor, memo).ok();
	let background = query.resolve(entity, BackgroundColor, memo).ok();
	let decoration_color = query.resolve(entity, DecorationColor, memo).ok();
	let decoration_line = query
		.resolve(entity, DecorationLineProp, memo)
		.unwrap_or_default();
	let decoration_style = query
		.resolve(entity, DecorationStyleProp, memo)
		.unwrap_or_default();
	let text_align = query
		.resolve(entity, TextAlignProp, memo)
		.unwrap_or_default();
	let font_weight = query
		.resolve(entity, FontWeightProp, memo)
		.unwrap_or_default();
	let font_style = query
		.resolve(entity, FontStyleProp, memo)
		.unwrap_or_default();
	// font-size drives charcell glyph scaling. MD3 sets it per-heading (`<h1>`..)
	// and on the `text-*` typography classes, so the standard cascade (own beats
	// inherited, class beats tag) resolves the effective size; absent, text is
	// 1em (normal single-cell glyphs).
	let font_size = query
		.resolve(entity, FontSize, memo)
		.unwrap_or(Length::Rem(1.0));
	let blink = query
		.resolve(entity, BlinkStyleProp, memo)
		.unwrap_or_default();
	let visibility = query
		.resolve(entity, VisibilityProp, memo)
		.unwrap_or_default();
	// `opacity` bakes into the resolved colours (the charcell approximation,
	// see [`VisualStyle::apply_opacity`]), so a transition eases the dim.
	let opacity = query.resolve(entity, OpacityProp, memo).unwrap_or(1.);

	let mut style = VisualStyle {
		foreground,
		background,
		decoration_color,
		decoration_line,
		decoration_style,
		font_weight,
		font_style,
		font_size,
		blink,
		visibility,
		text_align,
	};
	style.apply_opacity(opacity);
	style.xok()
}

fn resolve_layout(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<LayoutStyle> {
	let flex_grow = query
		.resolve(entity, FlexGrowProp, memo)
		.unwrap_or_default();
	let flex_order = query
		.resolve(entity, FlexOrderProp, memo)
		.unwrap_or_default();
	let align_self = query
		.resolve(entity, AlignSelfProp, memo)
		.unwrap_or_default();
	let display = query.resolve(entity, DisplayProp, memo).unwrap_or_default();
	let white_space = query
		.resolve(entity, WhiteSpaceProp, memo)
		.unwrap_or_default();
	let direction = query
		.resolve(entity, FlexDirectionProp, memo)
		.unwrap_or_default();
	let wrap = query
		.resolve(entity, FlexWrapProp, memo)
		.unwrap_or_default();
	let justify_content = query
		.resolve(entity, JustifyContentProp, memo)
		.unwrap_or_default();
	let align_items = query
		.resolve(entity, AlignItemsProp, memo)
		.unwrap_or_default();
	let align_content = query
		.resolve(entity, AlignContentProp, memo)
		.unwrap_or_default();
	// gaps stay as `Length` here (the resolution-independent value): each renderer
	// converts at layout time, where the real viewport is known. The charcell
	// engine rounds to whole cells via `FlexBox::{row,column}_gap_cells`.
	let row_gap = query.resolve(entity, RowGapProp, memo).unwrap_or_default();
	let column_gap = query
		.resolve(entity, ColumnGapProp, memo)
		.unwrap_or_default();
	let overflow_x = query
		.resolve(entity, OverflowXProp, memo)
		.unwrap_or_default();
	let overflow_y = query
		.resolve(entity, OverflowYProp, memo)
		.unwrap_or_default();
	let grid = GridTracks {
		columns: query
			.resolve(entity, GridTemplateColumnsProp, memo)
			.unwrap_or_default(),
		rows: query
			.resolve(entity, GridAutoRowsProp, memo)
			.unwrap_or_default(),
	};
	LayoutStyle {
		display,
		white_space,
		flex_box: FlexBox {
			direction,
			wrap,
			justify_content,
			align_items,
			align_content,
			row_gap,
			column_gap,
		},
		grid,
		flex_grow,
		flex_order,
		align_self,
		overflow_x,
		overflow_y,
	}
	.xok()
}

fn resolve_position(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<PositionStyle> {
	let position = query
		.resolve(entity, PositionProp, memo)
		.unwrap_or_default();
	// each inset is `auto` (None) unless a rule sets it.
	PositionStyle {
		position,
		inset: [
			query.resolve(entity, InsetTop, memo).ok(),
			query.resolve(entity, InsetRight, memo).ok(),
			query.resolve(entity, InsetBottom, memo).ok(),
			query.resolve(entity, InsetLeft, memo).ok(),
		],
		z_index: query.resolve(entity, ZIndexProp, memo).ok(),
	}
	.xok()
}

fn resolve_scrollbar(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<ScrollbarStyle> {
	let width = query
		.resolve(entity, ScrollbarWidthProp, memo)
		.unwrap_or_default();
	// scrollbar-color sets both thumb and track; absent leaves renderer defaults.
	let color = query.resolve(entity, ScrollbarColorProp, memo).ok();
	ScrollbarStyle {
		thumb: color.map(|c| c.thumb),
		track: color.map(|c| c.track),
		width,
	}
	.xok()
}

fn resolve_transition(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<TransitionStyle> {
	let default = TransitionStyle::default();
	TransitionStyle {
		duration: query
			.resolve(entity, TransitionDurationProp, memo)
			.unwrap_or(default.duration),
		ease: query
			.resolve(entity, TransitionEaseProp, memo)
			.unwrap_or(default.ease),
	}
	.xok()
}

fn resolve_box(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<BoxStyle> {
	let padding = query.resolve(entity, Padding, memo).unwrap_or_default();
	let margin = query.resolve(entity, MarginProp, memo).unwrap_or_default();
	let border_color = query.resolve(entity, BorderColorProp, memo).ok();
	// per-side border widths fall back to the uniform `border-width`, so a rule
	// can reserve a single edge (eg `border-right`) or a full box.
	let uniform = query.resolve(entity, OutlineWidth, memo).ok();
	let resolve_side = |width: Result<Length>| {
		width.ok().or(uniform).unwrap_or(Length::DEFAULT)
	};
	BoxStyle {
		border_left: border_color,
		border_right: border_color,
		border_top: border_color,
		border_bottom: border_color,
		border: Spacing {
			top: resolve_side(query.resolve(entity, BorderTopWidth, memo)),
			right: resolve_side(query.resolve(entity, BorderRightWidth, memo)),
			bottom: resolve_side(query.resolve(
				entity,
				BorderBottomWidth,
				memo,
			)),
			left: resolve_side(query.resolve(entity, BorderLeftWidth, memo)),
		},
		margin,
		padding,
		width: query.resolve(entity, Width, memo).ok(),
		min_width: query.resolve(entity, MinWidth, memo).ok(),
		max_width: query.resolve(entity, MaxWidth, memo).ok(),
		height: query.resolve(entity, Height, memo).ok(),
		min_height: query.resolve(entity, MinHeight, memo).ok(),
		max_height: query.resolve(entity, MaxHeight, memo).ok(),
	}
	.xok()
}
