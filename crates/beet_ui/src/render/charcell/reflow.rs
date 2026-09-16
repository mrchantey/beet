//! Fit a text diagram's art to its laid-out width (`parse/mermaid`).
//!
//! A `<pre class="diagram-text">` holds box art `mermaid-text` reflows to a
//! column budget, and the right budget is the width layout assigns the
//! `<pre>`: the surface less the docs column, the sidebar rail and every other
//! ancestor inset, which nothing before layout knows. So the diagram pass
//! builds the art unbounded and this pass, between layout and paint,
//! re-renders it to the assigned columns in place, then settles the rects
//! again beneath the new rows. The figure's own rect owes nothing to its art
//! (a block scrolling its overflow), so the fit converges in one pass, and a
//! later frame at the same width finds the art fitted and does nothing; a
//! resize refits it the frame it lands.
use super::*;
use crate::parse::DiagramForm;
use crate::parse::MermaidDiagram;
use crate::prelude::*;
use beet_core::prelude::*;

/// Exclusive step between layout and paint: fit every text diagram under a
/// `B` surface, and when any art changed run the measure and the layout again
/// so paint reads rects that hold the new rows.
pub(super) fn reflow_diagrams<B: Component + AsBuffer>(
	world: &mut World,
) -> Result {
	if world.run_system_cached(fit_text_diagrams::<B>)? {
		world.run_system_cached::<(), _, _>(measure_nodes::<B>)?;
		world.run_system_cached::<(), _, _>(layout_nodes::<B>)?;
	}
	Ok(())
}

/// Re-render each text figure whose `<pre>` was laid out at a width other than
/// the columns its art was fitted to, writing the art into the `<pre>`'s text
/// node so the resolved styles hold, and recording the fit on the figure's
/// [`DiagramForm`]. Returns whether any art changed.
fn fit_text_diagrams<B: Component + AsBuffer>(
	// the charcell view and the cascade read every `Value`, the write lands
	// after them
	mut params: ParamSet<((CharcellQuery, RuleSetQuery), Query<&mut Value>)>,
	buffers: Query<&B>,
	mut figures: Query<(Entity, &MermaidDiagram, &mut DiagramForm, &Children)>,
	art: Query<(&Classes, &Children)>,
) -> bool {
	// read phase: the text node and the art fitted to its assigned columns
	let mut fits = Vec::new();
	let (charcell, rules) = params.p0();
	for (figure, diagram, mut form, children) in figures.iter_mut() {
		let DiagramForm::Text { columns } = *form else {
			continue;
		};
		// under a `B` surface, whose cell size the box model resolves against
		let Some(viewport) = rules
			.surface_viewport(figure)
			.and_then(|(surface, _)| buffers.get(surface).ok())
			.map(|buffer| buffer.size())
		else {
			continue;
		};
		// the art is the `<pre class="diagram-text">`, its text the one child
		let Some((pre, text)) = children.iter().find_map(|child| {
			art.get(child)
				.ok()
				.filter(|(classes, _)| classes.contains_name(&DIAGRAM_TEXT))
				.and_then(|(_, children)| children.first().copied())
				.map(|text| (child, text))
		}) else {
			continue;
		};
		let Ok(node) = charcell.unresolved_node(pre) else {
			continue;
		};
		let fitted = BoxModel::from_node(&node, viewport)
			.content_rect(node.layout_rect())
			.width()
			.max(1) as usize;
		if columns == Some(fitted) {
			continue;
		}
		// the source rendered once already, so only a width it cannot lay out
		// fails, and the unfitted art stays
		let Ok(rendered) =
			mermaid_text::render_with_width(&diagram.source, Some(fitted))
		else {
			continue;
		};
		*form = DiagramForm::Text {
			columns: Some(fitted),
		};
		fits.push((text, Value::str(rendered)));
	}
	// write phase: only art that differs dirties the tree
	let mut values = params.p1();
	let mut changed = false;
	for (text, rendered) in fits {
		if let Ok(mut value) = values.get_mut(text)
			&& *value != rendered
		{
			*value = rendered;
			changed = true;
		}
	}
	changed
}

#[cfg(all(test, feature = "markdown_parser"))]
mod test {
	use super::*;
	use crate::style::Length;
	use crate::style::Spacing;
	use crate::style::common_props::Padding;

	/// A chain that lays out wider than 36 columns unbounded, so a fit shows.
	const SOURCE: &str =
		"graph LR; A[Parse] --> B[Style] --> C[Layout] --> D[Paint]";

	/// The figure's form and its art.
	fn fit(world: &mut World, figure: Entity) -> (DiagramForm, String) {
		let form = *world.entity(figure).get::<DiagramForm>().unwrap();
		let art = world.with_state::<ElementQuery, _>(|elements| {
			elements
				.iter_descendants_inclusive(figure)
				.find(|view| view.contains_class_name(&DIAGRAM_TEXT))
				.and_then(|view| {
					view.inner_text.map(|(_, text)| text.to_string())
				})
				.unwrap()
		});
		(form, art)
	}

	/// The widest line of some art.
	fn widest(art: &str) -> usize {
		art.lines().map(|line| line.chars().count()).max().unwrap()
	}

	/// The art fits the columns layout gives the `<pre>`, every ancestor inset
	/// included: a padded column narrows it below what the surface alone would
	/// allow, and a resize refits it in place.
	#[beet_core::test]
	fn fits_the_laid_out_width() {
		let mut world = CharcellPlugin::world();
		let root = world
			.spawn((FlexBuffer::new(60), RenderSurface::self_referential()))
			.id();
		// 10 cells of padding either side, then the figure's own 2
		let column = world
			.spawn((
				Element::new("div"),
				inline_class![(Padding, Spacing::all(Length::Rem(5.)))],
				ChildOf(root),
			))
			.id();
		MarkdownParser::new()
			.parse(ParseContext::new(
				&mut world.entity_mut(column),
				&MediaBytes::new_markdown(&format!(
					"```mermaid\n{SOURCE}\n```"
				)),
			))
			.unwrap();
		let figure = world.entity(column).get::<Children>().unwrap()[0];
		let children = world.entity(figure).get::<Children>().unwrap().to_vec();
		let (form, art) = fit(&mut world, figure);
		form.xpect_eq(DiagramForm::Text { columns: Some(36) });
		// the crate's fit at the column, compacted from the unbounded art
		art.xref().xpect_eq(
			mermaid_text::render_with_width(SOURCE, Some(36)).unwrap(),
		);
		widest(&art).xpect_less_than(widest(
			&mermaid_text::render_with_width(SOURCE, None).unwrap(),
		));
		// a wider surface refits the same entities
		world.entity_mut(root).insert(FlexBuffer::new(100));
		world.run_schedule(PostParseTree);
		fit(&mut world, figure)
			.0
			.xpect_eq(DiagramForm::Text { columns: Some(76) });
		world
			.entity(figure)
			.get::<Children>()
			.unwrap()
			.to_vec()
			.xpect_eq(children);
	}
}
