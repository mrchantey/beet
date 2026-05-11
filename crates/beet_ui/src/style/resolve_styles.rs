use crate::prelude::*;
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
			let visual = resolve_visual(&ruleset_query, entity)?;
			println!("here we are! {:#?}", visual);
			if let Some(mut style) = styles.get_mut(entity)?.0 {
				style.set_if_neq(visual);
			} else {
				commands.entity(entity).insert(visual);
			}
			if let Some(children) = children.get(entity).ok() {
				queue.extend(children.into_iter().cloned());
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
	let text_style = query.resolve(entity, TextStyleProp).unwrap_or_default();

	VisualStyle {
		foreground,
		background,
		decoration_color,
		decoration_line,
		decoration_style,
		text_align,
		text_style,
	}
	.xok()
}
