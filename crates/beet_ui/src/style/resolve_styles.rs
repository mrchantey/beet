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
	query: Query<Entity, Changed<Element>>,
	ancestors: Query<&ChildOf>,
	children: Query<&Children>,
	mut styles: Query<(
		Option<&mut VisualStyle>,
		Option<&mut LayoutStyle>,
		Option<&mut BoxStyle>,
	)>,
) -> Result {
	// TODO fine-grained listeners
	// listen for class attribute changes,
	// reparenting etc.
	// only update whats needed
	let roots = query
		.iter()
		.map(|entity| ancestors.root_ancestor(entity))
		.collect::<HashSet<_>>();

	// inheritance cache friendly parallelism, top down queue,
	// as described in stylo https://youtu.be/Y6SSTRr2mFU?t=310
	let mut queue = roots.into_iter().collect::<Vec<_>>();
	while !queue.is_empty() {
		for entity in queue.drain(..).collect::<Vec<_>>() {
			// resolve visual style
			let visual = resolve_visual(&ruleset_query, entity)?;
			if let Some(mut style) = styles.get_mut(entity)?.0 {
				style.set_if_neq(visual);
			} else {
				commands.entity(entity).insert(visual);
			}

			// resolve layout style
			let layout = resolve_layout(&ruleset_query, entity)?;
			if let Some(mut style) = styles.get_mut(entity)?.1 {
				style.set_if_neq(layout);
			} else {
				commands.entity(entity).insert(layout);
			}

			// resolve box style
			let box_s = resolve_box(&ruleset_query, entity)?;
			if let Some(mut style) = styles.get_mut(entity)?.2 {
				style.set_if_neq(box_s);
			} else {
				commands.entity(entity).insert(box_s);
			}

			if let Some(children_list) = children.get(entity).ok() {
				queue.extend(children_list.into_iter().cloned());
			}
		}
	}
	Ok(())
}

fn resolve_visual(query: &RuleSetQuery, entity: Entity) -> Result<VisualStyle> {
	let foreground = query.resolve(entity, ForegroundColor).ok();
	let background = query.resolve(entity, BackgroundColor).ok();
	let decoration_color = query.resolve(entity, DecorationColor).ok();
	let decoration_line = query
		.resolve(entity, DecorationLineProp)
		.unwrap_or_default();
	let decoration_style = query
		.resolve(entity, DecorationStyleProp)
		.unwrap_or_default();
	let text_align = query.resolve(entity, TextAlignProp).unwrap_or_default();
	let font_weight = query.resolve(entity, FontWeightProp).unwrap_or_default();
	let font_style = query.resolve(entity, FontStyleProp).unwrap_or_default();
	let blink = query.resolve(entity, BlinkStyleProp).unwrap_or_default();
	let visibility = query.resolve(entity, VisibilityProp).unwrap_or_default();

	VisualStyle {
		foreground,
		background,
		decoration_color,
		decoration_line,
		decoration_style,
		font_weight,
		font_style,
		blink,
		visibility,
		text_align,
	}
	.xok()
}

fn resolve_layout(query: &RuleSetQuery, entity: Entity) -> Result<LayoutStyle> {
	let flex_grow = query.resolve(entity, FlexGrowProp).unwrap_or_default();
	let flex_order = query.resolve(entity, FlexOrderProp).unwrap_or_default();
	let align_self = query.resolve(entity, AlignSelfProp).unwrap_or_default();
	let display = query.resolve(entity, DisplayProp).unwrap_or_default();
	let white_space = query.resolve(entity, WhiteSpaceProp).unwrap_or_default();
	let direction =
		query.resolve(entity, FlexDirectionProp).unwrap_or_default();
	let wrap = query.resolve(entity, FlexWrapProp).unwrap_or_default();
	let justify_content = query
		.resolve(entity, JustifyContentProp)
		.unwrap_or_default();
	let align_items = query.resolve(entity, AlignItemsProp).unwrap_or_default();
	let align_content =
		query.resolve(entity, AlignContentProp).unwrap_or_default();
	let row_gap = query.resolve(entity, RowGapProp).unwrap_or_default();
	let column_gap = query.resolve(entity, ColumnGapProp).unwrap_or_default();
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
		flex_grow,
		flex_order,
		align_self,
	}
	.xok()
}

fn resolve_box(query: &RuleSetQuery, entity: Entity) -> Result<BoxStyle> {
	let padding = query.resolve(entity, Padding).unwrap_or_default();
	let margin = query.resolve(entity, MarginProp).unwrap_or_default();
	let border_width = query.resolve(entity, OutlineWidth).ok();
	let border_color = query.resolve(entity, BorderColorProp).ok();
	BoxStyle {
		border_left: border_color,
		border_right: border_color,
		border_top: border_color,
		border_bottom: border_color,
		border: border_width.map(Spacing::all).unwrap_or_default(),
		margin,
		padding,
	}
	.xok()
}
