use crate::style::*;
use beet_core::prelude::*;

/// The form a diagram (a ```` ```mermaid ```` fence, a `<Mermaid>`) takes: the
/// `diagram-render` cascade property. Inherited like a text prop, so it is set
/// per app (a rule), per page (`bx:style` on the page root) or per diagram (the
/// fence info word, ```` ```mermaid text ````).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiagramRender {
	/// Resolved by sink: svg on the web; on a terminal, text for a flowchart
	/// (it reflows to the column budget) and a raster for every other type
	/// where the session's terminal has kitty graphics, else text.
	#[default]
	Auto,
	/// The rendered svg. A build without the svg renderer (wasm, `mermaid`
	/// without `mermaid_svg`) falls back to text with a warning, never an error:
	/// a lean binary keeps the diagram, only its form degrades.
	Svg,
	/// Unicode box-drawing text on every sink.
	Text,
}

impl DiagramRender {
	/// Parse a fence info word (`text`, `svg`, `auto`, any case).
	pub fn parse_word(word: &str) -> Option<Self> {
		match word.to_ascii_lowercase().as_str() {
			"auto" => Some(Self::Auto),
			"svg" => Some(Self::Svg),
			"text" => Some(Self::Text),
			_ => None,
		}
	}
}

impl AsCssValue for DiagramRender {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Auto => "auto",
			Self::Svg => "svg",
			Self::Text => "text",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use crate::style::common_props::DiagramRenderProp;
	use crate::style::material::MaterialStylePlugin;

	#[beet_core::test]
	fn serializes_as_a_custom_property() {
		CssRule::from_rule(
			&common_props::token_map(),
			&Rule::new().with_canonical(DiagramRender::Text),
		)
		.unwrap()
		.into_declarations()
		.into_iter()
		.map(|(key, value)| format!("{key}: {value}"))
		.collect::<Vec<_>>()
		.xpect_eq(vec!["--diagram-render: text".to_string()]);
	}

	/// A page root's `bx:style` reaches every figure below it (the prop is
	/// inherited), and a nested declaration overrides it.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn inherits_from_the_page_root() {
		let mut world = MaterialStylePlugin::world();
		let container = world
			.spawn_template(BsxTemplate::container(
				BsxNode::parse_document(
					r#"<div bx:style="diagram-render=Text"><figure/><div bx:style="diagram-render=Svg"><figure/></div></div>"#,
					&BsxParseConfig::bsx(),
				)
				.unwrap(),
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world.flush();
		let page = world.entity(container).get::<Children>().unwrap()[0];
		let page_children =
			world.entity(page).get::<Children>().unwrap().to_vec();
		let nested_figure =
			world.entity(page_children[1]).get::<Children>().unwrap()[0];
		let resolve = |world: &mut World, entity: Entity| {
			world.with_state::<RuleSetQuery, _>(|rules| {
				rules
					.resolve(
						entity,
						DiagramRenderProp,
						&mut CascadeMemo::default(),
					)
					.unwrap()
			})
		};
		resolve(&mut world, page_children[0]).xpect_eq(DiagramRender::Text);
		resolve(&mut world, nested_figure).xpect_eq(DiagramRender::Svg);
	}
}
